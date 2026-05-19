use std::collections::BTreeMap;
use std::path::Path;

use crate::app::{AppContext, BuiltInstallTarget, ParsedInstallRequest, ResolvedInstallTarget};
use crate::config::InstallPreference;
use crate::error::CoreError;
use crate::flags::parse_cli_flag_list;
use elda_build::{BuildRequest, build_recipe};
use elda_recipe::{
    GitRefKind, GitRefRequest, ImportOptions, SOURCE_LANE_BINARY, SOURCE_LANE_SOURCE,
    SourceDefinition, add_recipe_with_options, is_git_like_target, load_recipe,
};
use elda_repo::{RepoError, list_remotes, load_remote_payload_trust, resolve_package};

#[derive(Debug, Clone)]
pub(crate) enum ResolutionReport {
    Single(Box<ResolvedInstallTarget>),
    Bulk(elda_recipe::SnapshotImportReport),
}

impl AppContext {
    pub(crate) fn build_resolved_target(
        &self,
        resolved: &ResolvedInstallTarget,
        offline: bool,
        stream_child_output: bool,
        build_line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<BuiltInstallTarget, CoreError> {
        let materialized_recipe = resolved
            .remote_recipe_source
            .as_ref()
            .map(|source| self.materialize_remote_recipe(source, offline))
            .transpose()?;
        let projected_materialized_recipe = materialized_recipe
            .as_ref()
            .map(|recipe| self.project_materialized_recipe(recipe, &resolved.selected_lane))
            .transpose()?;
        let recipe = projected_materialized_recipe
            .as_ref()
            .unwrap_or_else(|| materialized_recipe.as_ref().unwrap_or(&resolved.recipe));
        let package = build_recipe(BuildRequest {
            recipe,
            cache_src_dir: &self.database.layout().cache_src_dir,
            cache_pkg_dir: &self.database.layout().cache_pkg_dir,
            tmp_dir: &self.database.layout().tmp_dir,
            offline,
            binary_caches: self.configured_binary_caches()?,
            remote_name: resolved.remote_name.clone(),
            binary_source_verification: resolved.binary_source_verification.clone(),
            release_trusted_keys: self.configured_release_trusted_keys(),
            allowed_git_protocols: self.config.git.allowed_protocols.clone(),
            persisted_source_kind: resolved.persisted_source_kind.clone(),
            persisted_source_ref: resolved.source_ref.clone(),
            variant_id: resolved.flag_state.variant_id.clone(),
            ad_hoc_git: resolved.ad_hoc_git,
            stream_child_output,
            build_line_hook,
        })?;

        Ok(BuiltInstallTarget {
            resolved: resolved.clone(),
            package,
        })
    }

    pub(crate) fn resolve_any_install_target(
        &self,
        target: &str,
        request: &ParsedInstallRequest,
    ) -> Result<ResolutionReport, CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;

        let requested_target = target;
        let target = request
            .git_source_refs
            .get(requested_target)
            .map(String::as_str)
            .unwrap_or(requested_target);

        if recipes_dir.join(target).join("pkg.lua").is_file() {
            let recipe = load_recipe(recipes_dir, target)?;
            return self
                .select_install_lane(
                    target,
                    recipe,
                    request,
                    Some(recipes_dir.join(target).display().to_string()),
                )
                .map(Box::new)
                .map(ResolutionReport::Single);
        }

        let path = Path::new(target);
        if path.exists() {
            let report = match add_recipe_with_options(
                recipes_dir,
                target,
                None,
                &self.metadata_import_options(request),
            )? {
                elda_recipe::ImportResult::Single(r) => r,
                elda_recipe::ImportResult::Bulk(b) => return Ok(ResolutionReport::Bulk(b)),
            };
            let recipe = load_recipe(recipes_dir, &report.recipe_name)?;
            let mut resolved = self.select_install_lane(
                target,
                recipe,
                request,
                Some(path.display().to_string()),
            )?;
            apply_generated_recipe_report(&mut resolved, &report);
            return Ok(ResolutionReport::Single(Box::new(resolved)));
        }

        if is_git_like_target(target) {
            let report = match add_recipe_with_options(
                recipes_dir,
                target,
                None,
                &self.metadata_import_options(request),
            )? {
                elda_recipe::ImportResult::Single(r) => r,
                elda_recipe::ImportResult::Bulk(b) => return Ok(ResolutionReport::Bulk(b)),
            };
            let recipe = load_recipe(recipes_dir, &report.recipe_name)?;
            let mut resolved =
                self.select_install_lane(target, recipe, request, Some(target.to_owned()))?;
            resolved.ad_hoc_git = resolved.selected_source_kind == "git";
            apply_ad_hoc_git_ref_override(&mut resolved, request, requested_target, target);
            if resolved.ad_hoc_git {
                resolved.source_ref = Some(ad_hoc_git_source_ref(target, request));
                resolved.ad_hoc_git_moving = resolved
                    .source_ref
                    .as_ref()
                    .map(|value| parse_ad_hoc_git_source_ref(value).moving)
                    .unwrap_or(false);
            }
            resolved.persisted_source_kind = if resolved.selected_source_kind == "git" {
                "git".to_owned()
            } else {
                Self::persisted_source_kind(&resolved.selected_source_kind, None, false)
            };
            apply_generated_recipe_report(&mut resolved, &report);
            return Ok(ResolutionReport::Single(Box::new(resolved)));
        }

        match resolve_package(&self.repo_snapshot_path(), target) {
            Ok(Some(package)) => {
                let recipe = package.parse_recipe()?;
                let mut resolved = self.select_install_lane(
                    target,
                    recipe,
                    request,
                    Some(format!(
                        "remote:{}/{}",
                        package.remote_name, package.pkgname
                    )),
                )?;
                resolved.remote_name = Some(package.remote_name.clone());
                if !matches!(
                    resolved.selected_source_kind.as_str(),
                    "url_archive" | "github_release" | "release_asset" | "appimage"
                ) && package.source_kind.as_deref() != Some("interemote")
                {
                    resolved.remote_recipe_source = Some(self.remote_recipe_source(&package)?);
                }
                if matches!(
                    resolved.selected_source_kind.as_str(),
                    "url_archive" | "github_release" | "release_asset" | "appimage"
                ) {
                    let payload_trust = load_remote_payload_trust(
                        &self.repo_snapshot_path(),
                        &package.remote_name,
                    )?;
                    self.apply_remote_snapshot_metadata(&mut resolved, &package, &payload_trust)?;
                }

                Ok(ResolutionReport::Single(Box::new(resolved)))
            }
            Ok(None) | Err(RepoError::SnapshotMissing) => {
                let remotes = list_remotes(&self.database.layout().remotes_dir)?;
                let message = if remotes.is_empty() {
                    format!(
                        "no local recipe or synced package named `{target}` exists, and no remotes are configured; add one with `elda rmt add <name>=<index-url>` and then run `elda sync`"
                    )
                } else {
                    format!(
                        "no local recipe or synced package named `{target}` exists; run `elda sync` to refresh remote packages"
                    )
                };
                Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    message,
                )))
            }
            Err(error) => Err(CoreError::Repo(error)),
        }
    }

    pub(crate) fn resolve_install_target(
        &self,
        target: &str,
        request: &ParsedInstallRequest,
    ) -> Result<ResolvedInstallTarget, CoreError> {
        match self.resolve_any_install_target(target, request)? {
            ResolutionReport::Single(resolved) => Ok(*resolved),
            ResolutionReport::Bulk(_) => Err(CoreError::Operator(format!(
                "target `{target}` is a bulk metadata snapshot; this command only supports single recipes"
            ))),
        }
    }

    pub(crate) fn select_install_lane(
        &self,
        target: &str,
        recipe: elda_recipe::RecipeDocument,
        request: &ParsedInstallRequest,
        source_ref: Option<String>,
    ) -> Result<ResolvedInstallTarget, CoreError> {
        let flag_state = self.resolve_flag_state(&recipe.package, request)?;
        let Some(mut selected_lane) = self.preferred_lane_name(&recipe.package.source, request)
        else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("target `{target}` has no selectable source lane"),
            )));
        };

        if selected_lane == SOURCE_LANE_BINARY && flag_state.customized {
            let explicit_binary = matches!(
                request.hard_lane.or(request.preferred_lane),
                Some(InstallPreference::Binary)
            );
            if recipe
                .package
                .source
                .lane_definition(SOURCE_LANE_SOURCE)
                .is_some()
                && !explicit_binary
            {
                selected_lane = SOURCE_LANE_SOURCE.to_owned();
            } else {
                return Err(CoreError::Operator(format!(
                    "target `{target}` resolves a non-default flag variant; the binary lane only supports the default variant in the current slice"
                )));
            }
        }

        self.select_named_lane(target, recipe, source_ref, &selected_lane, flag_state)
    }

    pub(crate) fn parse_install_request(
        &self,
        request: &crate::CommandRequest,
    ) -> Result<ParsedInstallRequest, CoreError> {
        let hard_lane = match request.command_path.first().map(String::as_str) {
            Some("ig") => Some(InstallPreference::Source),
            Some("ib") => Some(InstallPreference::Binary),
            _ => None,
        };
        let mut preferred_lane = None;
        let mut source_option = None;
        let mut source_strategy = None;
        let mut git_ref = None;
        let git_source_refs = BTreeMap::new();
        let mut cli_flag_overrides = BTreeMap::new();
        let mut replace = false;
        let mut exclude = Vec::new();
        let mut provider_choices = BTreeMap::new();
        let mut targets = Vec::new();
        let mut operands = request.operands.iter().peekable();

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--prefer-source" => {
                    if hard_lane == Some(InstallPreference::Binary) {
                        return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                            "`ib` cannot be combined with `--prefer-source`".to_owned(),
                        )));
                    }
                    if preferred_lane == Some(InstallPreference::Binary) {
                        return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                            "install lane preference flags are mutually exclusive".to_owned(),
                        )));
                    }
                    preferred_lane = Some(InstallPreference::Source);
                }
                "--prefer-binary" => {
                    if hard_lane == Some(InstallPreference::Source) {
                        return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                            "`ig` cannot be combined with `--prefer-binary`".to_owned(),
                        )));
                    }
                    if preferred_lane == Some(InstallPreference::Source) {
                        return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                            "install lane preference flags are mutually exclusive".to_owned(),
                        )));
                    }
                    preferred_lane = Some(InstallPreference::Binary);
                }
                "--source-option" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--source-option` requires a 1-based index".to_owned())
                    })?;
                    source_option = Some(parse_source_option_index(value)?);
                }
                "--strategy" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--strategy` requires one strategy name".to_owned())
                    })?;
                    source_strategy = Some(parse_source_strategy(value)?);
                }
                "--to-branch" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-branch` requires one branch name".to_owned())
                    })?;
                    set_git_ref(&mut git_ref, GitRefKind::Branch, value)?;
                }
                "--to-tag" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-tag` requires one tag name".to_owned())
                    })?;
                    set_git_ref(&mut git_ref, GitRefKind::Tag, value)?;
                }
                "--to-rev" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-rev` requires one revision".to_owned())
                    })?;
                    set_git_ref(&mut git_ref, GitRefKind::Rev, value)?;
                }
                "--use" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator(
                            "`--use` requires one comma-delimited flag list".to_owned(),
                        )
                    })?;
                    cli_flag_overrides.extend(parse_cli_flag_list(value)?);
                }
                "--provider" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--provider` requires `virtual=package`".to_owned())
                    })?;
                    let (virtual_name, provider) =
                        super::provider_choice::parse_provider_assignment(value)?;
                    provider_choices.insert(virtual_name, provider);
                }
                "--replace" => {
                    replace = true;
                }
                "--exclude" => {
                    let mut count = 0usize;
                    while let Some(rest) = operands.next() {
                        if rest.starts_with("--") {
                            return Err(CoreError::Operator(format!(
                                "`--exclude` consumes all trailing package names; place other flags before `--exclude` (unexpected `{rest}`)"
                            )));
                        }
                        count += 1;
                        crate::app_parse::append_exclude_from_piece(rest, &mut exclude);
                    }
                    if count == 0 {
                        return Err(CoreError::Operator(
                            "`--exclude` requires at least one package name".to_owned(),
                        ));
                    }
                    break;
                }
                _ if operand.starts_with("--source-option=") => {
                    source_option = Some(parse_source_option_index(
                        operand.trim_start_matches("--source-option="),
                    )?);
                }
                _ if operand.starts_with("--strategy=") => {
                    source_strategy = Some(parse_source_strategy(
                        operand.trim_start_matches("--strategy="),
                    )?);
                }
                _ if operand.starts_with("--to-branch=") => {
                    set_git_ref(
                        &mut git_ref,
                        GitRefKind::Branch,
                        operand.trim_start_matches("--to-branch="),
                    )?;
                }
                _ if operand.starts_with("--to-tag=") => {
                    set_git_ref(
                        &mut git_ref,
                        GitRefKind::Tag,
                        operand.trim_start_matches("--to-tag="),
                    )?;
                }
                _ if operand.starts_with("--to-rev=") => {
                    set_git_ref(
                        &mut git_ref,
                        GitRefKind::Rev,
                        operand.trim_start_matches("--to-rev="),
                    )?;
                }
                _ if operand.starts_with("--use=") => {
                    cli_flag_overrides
                        .extend(parse_cli_flag_list(operand.trim_start_matches("--use="))?);
                }
                _ if operand.starts_with("--exclude=") => {
                    crate::app_parse::append_exclude_from_piece(
                        operand.trim_start_matches("--exclude="),
                        &mut exclude,
                    );
                }
                _ if operand.starts_with("--provider=") => {
                    let (virtual_name, provider) =
                        super::provider_choice::parse_provider_assignment(
                            operand.trim_start_matches("--provider="),
                        )?;
                    provider_choices.insert(virtual_name, provider);
                }
                _ => targets.push(operand.clone()),
            }
        }

        Ok(ParsedInstallRequest {
            targets,
            hard_lane,
            preferred_lane,
            source_option,
            source_strategy,
            git_ref,
            git_source_refs,
            git_ref_overrides: BTreeMap::new(),
            cli_flag_overrides,
            replace,
            exclude,
            provider_choices,
        })
    }

    fn metadata_import_options(&self, request: &ParsedInstallRequest) -> ImportOptions {
        ImportOptions {
            strategy_priority: self.metadata_strategy_priority(request),
            release_binary_format_priority: self
                .config
                .metadata
                .release_binary_format_priority
                .clone(),
            selected_source_option: request.source_option,
            git_ref: self.metadata_git_ref_for_target(request),
            replace: request.replace,
            exclude: request.exclude.clone(),
        }
    }

    fn metadata_git_ref_for_target(&self, request: &ParsedInstallRequest) -> Option<GitRefRequest> {
        let target = request.targets.first()?;
        request
            .git_ref_overrides
            .get(target)
            .cloned()
            .or_else(|| request.git_ref.clone())
    }

    fn metadata_strategy_priority(&self, request: &ParsedInstallRequest) -> Vec<String> {
        if let Some(strategy) = &request.source_strategy {
            return prioritized_strategy(&self.config.metadata.link_strategy_priority, strategy);
        }
        self.config.metadata.link_strategy_priority.clone()
    }

    pub(crate) fn dependency_install_request(
        &self,
        request: &ParsedInstallRequest,
    ) -> ParsedInstallRequest {
        ParsedInstallRequest {
            targets: Vec::new(),
            hard_lane: None,
            preferred_lane: request.hard_lane.or(request.preferred_lane),
            source_option: None,
            source_strategy: None,
            git_ref: None,
            git_source_refs: BTreeMap::new(),
            git_ref_overrides: BTreeMap::new(),
            cli_flag_overrides: request.cli_flag_overrides.clone(),
            replace: false,
            exclude: Vec::new(),
            provider_choices: request.provider_choices.clone(),
        }
    }

    pub(crate) fn install_preference_name(preference: InstallPreference) -> &'static str {
        match preference {
            InstallPreference::Source => SOURCE_LANE_SOURCE,
            InstallPreference::Binary => SOURCE_LANE_BINARY,
        }
    }

    pub(crate) fn project_recipe_lane(
        mut recipe: elda_recipe::RecipeDocument,
        selected: elda_recipe::SourceLaneDefinition,
    ) -> elda_recipe::RecipeDocument {
        recipe.package.source = SourceDefinition::single_lane_with_assets(
            selected.kind,
            selected.fields,
            selected.github_release_assets,
        );
        recipe
    }

    fn preferred_lane_name(
        &self,
        source: &SourceDefinition,
        request: &ParsedInstallRequest,
    ) -> Option<String> {
        if let Some(hard_lane) = request.hard_lane {
            return Some(Self::install_preference_name(hard_lane).to_owned());
        }

        if let Some(preferred) = request.preferred_lane
            && source
                .lane_definition(Self::install_preference_name(preferred))
                .is_some()
        {
            return Some(Self::install_preference_name(preferred).to_owned());
        }

        if let Some(default_lane) = source.default_lane.clone()
            && source.lane_definition(&default_lane).is_some()
        {
            return Some(default_lane);
        }

        let preferred = Self::install_preference_name(self.config.defaults.install_preference);
        if source.lane_definition(preferred).is_some() {
            return Some(preferred.to_owned());
        }

        source.available_lanes().first().cloned()
    }

    fn select_named_lane(
        &self,
        target: &str,
        recipe: elda_recipe::RecipeDocument,
        source_ref: Option<String>,
        lane_name: &str,
        flag_state: crate::flags::ResolvedFlagState,
    ) -> Result<ResolvedInstallTarget, CoreError> {
        let Some(selected) = recipe.package.source.lane_definition(lane_name) else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("target `{target}` does not expose a `{lane_name}` install lane"),
            )));
        };

        Ok(ResolvedInstallTarget {
            recipe: Self::project_recipe_lane(recipe, selected.clone()),
            selected_lane: lane_name.to_owned(),
            selected_source_kind: selected.kind.clone(),
            persisted_source_kind: Self::persisted_source_kind(
                &selected.kind,
                source_ref.as_ref(),
                flag_state.customized,
            ),
            flag_state,
            source_ref,
            remote_name: None,
            remote_recipe_source: None,
            binary_source_verification: None,
            ad_hoc_git: false,
            ad_hoc_git_moving: false,
            generated_recipe_name: None,
            generated_recipe_dir: None,
            source_options: Vec::new(),
            selected_source_option: None,
        })
    }

    fn persisted_source_kind(
        selected_source_kind: &str,
        source_ref: Option<&String>,
        customized_variant: bool,
    ) -> String {
        let remote_backed = source_ref.is_some_and(|value| value.starts_with("remote:"));

        match selected_source_kind {
            "git" => "local_recipe".to_owned(),
            "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template" => {
                "interbuild".to_owned()
            }
            "url_archive" | "github_release" | "release_asset" | "appimage"
                if remote_backed && !customized_variant =>
            {
                "repo_binary".to_owned()
            }
            "url_archive" | "github_release" | "release_asset" | "appimage" => {
                "local_recipe".to_owned()
            }
            other => other.to_owned(),
        }
    }

    fn project_materialized_recipe(
        &self,
        recipe: &elda_recipe::RecipeDocument,
        selected_lane: &str,
    ) -> Result<elda_recipe::RecipeDocument, CoreError> {
        let Some(selected) = recipe.package.source.lane_definition(selected_lane) else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("materialized recipe is missing the selected `{selected_lane}` lane"),
            )));
        };

        Ok(Self::project_recipe_lane(recipe.clone(), selected.clone()))
    }
}

fn apply_generated_recipe_report(
    resolved: &mut ResolvedInstallTarget,
    report: &elda_recipe::ImportReport,
) {
    resolved.source_options = report.source_options.clone();
    resolved.selected_source_option = report.selected_source_option.clone();

    if !report.generated_pkg_lua && !report.generated_build_lua {
        return;
    }

    resolved.generated_recipe_name = Some(report.recipe_name.clone());
    resolved.generated_recipe_dir = Some(report.recipe_dir.clone());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedAdHocGitSourceRef {
    pub(crate) target: String,
    pub(crate) git_ref: Option<GitRefRequest>,
    pub(crate) moving: bool,
}

pub(crate) fn parse_ad_hoc_git_source_ref(value: &str) -> ParsedAdHocGitSourceRef {
    let Some((target, suffix)) = value.rsplit_once('#') else {
        return ParsedAdHocGitSourceRef {
            target: value.to_owned(),
            git_ref: None,
            moving: true,
        };
    };
    let Some((kind, ref_value)) = suffix.split_once(':') else {
        return ParsedAdHocGitSourceRef {
            target: value.to_owned(),
            git_ref: None,
            moving: true,
        };
    };
    let kind = match kind {
        "branch" => GitRefKind::Branch,
        "tag" => GitRefKind::Tag,
        "rev" => GitRefKind::Rev,
        _ => {
            return ParsedAdHocGitSourceRef {
                target: value.to_owned(),
                git_ref: None,
                moving: true,
            };
        }
    };
    ParsedAdHocGitSourceRef {
        target: target.to_owned(),
        git_ref: Some(GitRefRequest {
            kind,
            value: ref_value.to_owned(),
        }),
        moving: kind == GitRefKind::Branch,
    }
}

pub(crate) fn ad_hoc_git_source_ref(target: &str, request: &ParsedInstallRequest) -> String {
    let git_ref = request
        .git_ref_overrides
        .get(target)
        .or(request.git_ref.as_ref());
    let Some(git_ref) = git_ref else {
        return target.to_owned();
    };
    let prefix = match git_ref.kind {
        GitRefKind::Branch => "branch",
        GitRefKind::Tag => "tag",
        GitRefKind::Rev => "rev",
    };
    format!("{target}#{prefix}:{}", git_ref.value)
}

pub(crate) fn set_git_ref(
    git_ref: &mut Option<GitRefRequest>,
    kind: GitRefKind,
    value: &str,
) -> Result<(), CoreError> {
    if git_ref.is_some() {
        return Err(CoreError::Operator(
            "git ref selectors are mutually exclusive".to_owned(),
        ));
    }
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::Operator(
            "git ref selectors do not accept empty values".to_owned(),
        ));
    }
    *git_ref = Some(GitRefRequest {
        kind,
        value: value.to_owned(),
    });
    Ok(())
}

fn parse_source_option_index(value: &str) -> Result<usize, CoreError> {
    let index = value.parse::<usize>().map_err(|_| {
        CoreError::Operator(format!(
            "`--source-option` requires a positive integer, got `{value}`"
        ))
    })?;
    if index == 0 {
        return Err(CoreError::Operator(
            "`--source-option` uses 1-based indexes".to_owned(),
        ));
    }
    Ok(index)
}

fn parse_source_strategy(value: &str) -> Result<String, CoreError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Operator(
            "`--strategy` does not accept an empty value".to_owned(),
        ));
    }
    Ok(trimmed.to_owned())
}

fn prioritized_strategy(priority: &[String], selected: &str) -> Vec<String> {
    let mut ordered = vec![selected.to_owned()];
    ordered.extend(
        priority
            .iter()
            .filter(|item| item.as_str() != selected)
            .cloned(),
    );
    ordered
}

fn apply_ad_hoc_git_ref_override(
    resolved: &mut ResolvedInstallTarget,
    request: &ParsedInstallRequest,
    requested_target: &str,
    source_target: &str,
) {
    if resolved.selected_source_kind != "git" {
        return;
    }

    let git_ref = request
        .git_ref_overrides
        .get(requested_target)
        .or_else(|| request.git_ref_overrides.get(source_target))
        .or(request.git_ref.as_ref());
    let Some(git_ref) = git_ref else {
        return;
    };

    resolved.recipe.package.source.fields.remove("branch");
    resolved.recipe.package.source.fields.remove("tag");
    resolved.recipe.package.source.fields.remove("rev");
    let key = match git_ref.kind {
        GitRefKind::Branch => "branch",
        GitRefKind::Tag => "tag",
        GitRefKind::Rev => "rev",
    };
    resolved.recipe.package.source.fields.insert(
        key.to_owned(),
        elda_recipe::ScalarValue::String(git_ref.value.clone()),
    );
}
