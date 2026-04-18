use std::path::Path;

use crate::app::{AppContext, BuiltInstallTarget, ParsedInstallRequest, ResolvedInstallTarget};
use crate::config::InstallPreference;
use crate::error::CoreError;
use crate::flags::parse_cli_flag_list;
use elda_build::{BuildRequest, build_recipe};
use elda_recipe::{
    SOURCE_LANE_BINARY, SOURCE_LANE_SOURCE, SourceDefinition, add_recipe, is_git_like_target,
    load_recipe,
};
use elda_repo::{RepoError, load_remote_payload_trust, resolve_package};

impl AppContext {
    pub(crate) fn build_resolved_target(
        &self,
        resolved: &ResolvedInstallTarget,
        offline: bool,
    ) -> Result<BuiltInstallTarget, CoreError> {
        let package = build_recipe(BuildRequest {
            recipe: &resolved.recipe,
            cache_src_dir: &self.database.layout().cache_src_dir,
            cache_pkg_dir: &self.database.layout().cache_pkg_dir,
            tmp_dir: &self.database.layout().tmp_dir,
            offline,
            binary_caches: self.configured_binary_caches()?,
            remote_name: resolved.remote_name.clone(),
            binary_source_verification: resolved.binary_source_verification.clone(),
            persisted_source_kind: resolved.persisted_source_kind.clone(),
            persisted_source_ref: resolved.source_ref.clone(),
            variant_id: resolved.flag_state.variant_id.clone(),
            ad_hoc_git: resolved.ad_hoc_git,
        })?;

        Ok(BuiltInstallTarget {
            resolved: resolved.clone(),
            package,
        })
    }

    pub(crate) fn resolve_install_target(
        &self,
        target: &str,
        request: &ParsedInstallRequest,
    ) -> Result<ResolvedInstallTarget, CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;

        if recipes_dir.join(target).join("pkg.lua").is_file() {
            let recipe = load_recipe(recipes_dir, target)?;
            return self.select_install_lane(
                target,
                recipe,
                request,
                Some(recipes_dir.join(target).display().to_string()),
            );
        }

        let path = Path::new(target);
        if path.exists() {
            let report = add_recipe(recipes_dir, target, None)?;
            let recipe = load_recipe(recipes_dir, &report.recipe_name)?;
            return self.select_install_lane(
                target,
                recipe,
                request,
                Some(path.display().to_string()),
            );
        }

        if is_git_like_target(target) {
            let report = add_recipe(recipes_dir, target, None)?;
            let recipe = load_recipe(recipes_dir, &report.recipe_name)?;
            let mut resolved =
                self.select_install_lane(target, recipe, request, Some(target.to_owned()))?;
            resolved.ad_hoc_git = true;
            resolved.persisted_source_kind = "git".to_owned();
            return Ok(resolved);
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
                if matches!(
                    resolved.selected_source_kind.as_str(),
                    "url_archive" | "github_release"
                ) {
                    let payload_trust = load_remote_payload_trust(
                        &self.repo_snapshot_path(),
                        &package.remote_name,
                    )?;
                    self.apply_remote_snapshot_metadata(&mut resolved, &package, &payload_trust)?;
                }

                Ok(resolved)
            }
            Ok(None) | Err(RepoError::SnapshotMissing) => Err(CoreError::Recipe(
                elda_recipe::RecipeError::InvalidInput(format!(
                    "no local recipe or synced package named `{target}` exists; run `elda sync` to refresh remote packages"
                )),
            )),
            Err(error) => Err(CoreError::Repo(error)),
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
        let mut cli_flag_overrides = std::collections::BTreeMap::new();
        let mut targets = Vec::new();
        let mut operands = request.operands.iter();

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
                "--use" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator(
                            "`--use` requires one comma-delimited flag list".to_owned(),
                        )
                    })?;
                    cli_flag_overrides.extend(parse_cli_flag_list(value)?);
                }
                _ if operand.starts_with("--use=") => {
                    cli_flag_overrides
                        .extend(parse_cli_flag_list(operand.trim_start_matches("--use="))?);
                }
                _ => targets.push(operand.clone()),
            }
        }

        Ok(ParsedInstallRequest {
            targets,
            hard_lane,
            preferred_lane,
            cli_flag_overrides,
        })
    }

    pub(crate) fn dependency_install_request(
        &self,
        request: &ParsedInstallRequest,
    ) -> ParsedInstallRequest {
        ParsedInstallRequest {
            targets: Vec::new(),
            hard_lane: None,
            preferred_lane: request.hard_lane.or(request.preferred_lane),
            cli_flag_overrides: request.cli_flag_overrides.clone(),
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
            binary_source_verification: None,
            ad_hoc_git: false,
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
            "nix_flake" | "gentoo_overlay" => "interbuild".to_owned(),
            "url_archive" | "github_release" if remote_backed && !customized_variant => {
                "repo_binary".to_owned()
            }
            "url_archive" | "github_release" => "local_recipe".to_owned(),
            other => other.to_owned(),
        }
    }
}
