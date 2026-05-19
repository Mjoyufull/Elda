use serde::Serialize;
use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_recipe::{
    IssueSeverity, PackageDefinition, RecipeDocument, ValidationIssue, load_recipe, validate_recipe,
};
use elda_repo::{RepoError, SyncedPackageRecord, resolve_package};

#[derive(Debug, Clone, Serialize)]
struct RecipeShowSource {
    source: String,
    recipe: RecipeDocument,
    validation: RecipeValidationSummary,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RecipeValidationSummary {
    pub(crate) errors: usize,
    pub(crate) warnings: usize,
    pub(crate) issues: Vec<ValidationIssue>,
}

pub(crate) fn validation_summary_for(recipe: &RecipeDocument) -> RecipeValidationSummary {
    validation_summary(recipe)
}

#[derive(Debug, Clone, Serialize)]
struct RecipeDiffChange {
    field: String,
    local: Option<String>,
    synced: Option<String>,
    changed: bool,
}

impl AppContext {
    pub(crate) fn handle_recipe_show(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let package_name = parse_recipe_show_target(&request)?;
        self.database.bootstrap()?;

        let local = self.load_local_recipe_show(&package_name)?;
        let synced = self.load_synced_recipe_show(&package_name)?;
        let selected = local.as_ref().or(synced.as_ref());
        let found = selected.is_some();

        Ok(CommandReport {
            area: "recipe",
            status: if found { "ok" } else { "missing" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if found {
                format!("reported recipe definition for `{package_name}`.")
            } else {
                format!("no local or synced recipe definition found for `{package_name}`.")
            },
            details: Some(json!({
                "show": {
                    "package": package_name,
                    "selected_source": selected.map(|source| source.source.as_str()),
                    "local": local,
                    "synced": synced,
                }
            })),
        })
    }

    pub(crate) fn handle_recipe_diff(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let package_name = parse_recipe_target(&request, "rc diff")?;
        self.database.bootstrap()?;

        let local = self.load_local_recipe_show(&package_name)?;
        let synced = self.load_synced_recipe_show(&package_name)?;
        let comparable = local.is_some() && synced.is_some();
        let changes = match (local.as_ref(), synced.as_ref()) {
            (Some(local), Some(synced)) => {
                recipe_diff_changes(&local.recipe.package, &synced.recipe.package)
            }
            _ => Vec::new(),
        };

        Ok(CommandReport {
            area: "recipe",
            status: if comparable { "ok" } else { "missing" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if comparable {
                format!("compared local and synced recipe definitions for `{package_name}`.")
            } else {
                format!(
                    "could not compare `{package_name}`; local and synced definitions are both required."
                )
            },
            details: Some(json!({
                "diff": {
                    "package": package_name,
                    "local": local,
                    "synced": synced,
                    "changes": changes,
                }
            })),
        })
    }

    pub(crate) fn handle_recipe_publish_ready(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let package_name = parse_recipe_target(&request, "rc publish-ready")?;
        self.database.bootstrap()?;
        let local = self.load_local_recipe_show(&package_name)?;
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        if let Some(source) = local.as_ref() {
            publish_readiness(
                &source.recipe.package,
                &source.validation,
                &mut blockers,
                &mut warnings,
            );
        } else {
            blockers.push("local recipe metadata is missing".to_owned());
        }
        let ready = blockers.is_empty();

        Ok(CommandReport {
            area: "recipe",
            status: if ready { "ok" } else { "not-ready" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if ready {
                format!("`{package_name}` is publish-ready for the current local checks.")
            } else {
                format!("`{package_name}` is not publish-ready for the current local checks.")
            },
            details: Some(json!({
                "publish_ready": {
                    "package": package_name,
                    "ready": ready,
                    "blockers": blockers,
                    "warnings": warnings,
                    "local": local,
                }
            })),
        })
    }

    fn load_local_recipe_show(
        &self,
        package_name: &str,
    ) -> Result<Option<RecipeShowSource>, CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;
        if !recipes_dir.join(package_name).join("pkg.lua").is_file() {
            return Ok(None);
        }

        let recipe = load_recipe(recipes_dir, package_name)?;
        Ok(Some(RecipeShowSource {
            source: "local".to_owned(),
            validation: validation_summary(&recipe),
            recipe,
        }))
    }

    fn load_synced_recipe_show(
        &self,
        package_name: &str,
    ) -> Result<Option<RecipeShowSource>, CoreError> {
        let Some(record) = optional_synced_package(&self.repo_snapshot_path(), package_name)?
        else {
            return Ok(None);
        };
        let source = synced_source_label(&record);
        let recipe = record.parse_recipe()?;

        Ok(Some(RecipeShowSource {
            source,
            validation: validation_summary(&recipe),
            recipe,
        }))
    }
}

fn parse_recipe_show_target(request: &CommandRequest) -> Result<String, CoreError> {
    parse_recipe_target(request, "rc show")
}

fn parse_recipe_target(request: &CommandRequest, command: &str) -> Result<String, CoreError> {
    if request.operands.len() != 1 {
        return Err(CoreError::Operator(format!(
            "{command} requires exactly one package name"
        )));
    }
    let package_name = request.operands[0].trim();
    if package_name.is_empty() {
        return Err(CoreError::Operator(format!(
            "{command} does not accept an empty package name"
        )));
    }
    crate::recipe_catalog::validate_recipe_pkgname(package_name)?;
    Ok(package_name.to_owned())
}

fn optional_synced_package(
    snapshot_path: &std::path::Path,
    package_name: &str,
) -> Result<Option<SyncedPackageRecord>, CoreError> {
    match resolve_package(snapshot_path, package_name) {
        Ok(package) => Ok(package),
        Err(RepoError::SnapshotMissing) => Ok(None),
        Err(error) => Err(CoreError::Repo(error)),
    }
}

fn synced_source_label(record: &SyncedPackageRecord) -> String {
    format!("synced:{}", record.remote_name)
}

fn validation_summary(recipe: &RecipeDocument) -> RecipeValidationSummary {
    let issues = validate_recipe(recipe);
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Error)
        .count();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == IssueSeverity::Warning)
        .count();
    RecipeValidationSummary {
        errors,
        warnings,
        issues,
    }
}

pub(crate) fn publish_readiness(
    package: &PackageDefinition,
    validation: &RecipeValidationSummary,
    blockers: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    blockers.extend(
        validation
            .issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Error)
            .map(|issue| format!("validation: {}", issue.message)),
    );
    push_missing(blockers, package.description.is_none(), "pkg.description");
    push_missing(blockers, package.licenses.is_empty(), "pkg.licenses");
    push_missing(warnings, package.upstream.is_none(), "pkg.upstream");
    if package.source.kind == "url_archive" && !package.source.fields.contains_key("sha256") {
        blockers.push("source.sha256 is required for archive publication".to_owned());
    }
}

fn push_missing(lines: &mut Vec<String>, missing: bool, field: &str) {
    if missing {
        lines.push(format!("missing {field}"));
    }
}

fn recipe_diff_changes(
    local: &PackageDefinition,
    synced: &PackageDefinition,
) -> Vec<RecipeDiffChange> {
    vec![
        diff_change(
            "version",
            Some(version_label(local)),
            Some(version_label(synced)),
        ),
        diff_change("kind", Some(local.kind.clone()), Some(synced.kind.clone())),
        diff_change(
            "source.kind",
            Some(local.source.kind.clone()),
            Some(synced.source.kind.clone()),
        ),
        diff_change(
            "arch",
            Some(local.arch.join(",")),
            Some(synced.arch.join(",")),
        ),
        diff_change(
            "depends",
            Some(local.depends.len().to_string()),
            Some(synced.depends.len().to_string()),
        ),
        diff_change(
            "makedepends",
            Some(local.makedepends.len().to_string()),
            Some(synced.makedepends.len().to_string()),
        ),
        diff_change(
            "provider_assets",
            Some(presence_label(local.provider_assets.is_some())),
            Some(presence_label(synced.provider_assets.is_some())),
        ),
        diff_change(
            "flags_allowed",
            Some(presence_label(local.flags_allowed.is_some())),
            Some(presence_label(synced.flags_allowed.is_some())),
        ),
        diff_change(
            "build.system",
            local.build.as_ref().map(|build| build.system.clone()),
            synced.build.as_ref().map(|build| build.system.clone()),
        ),
    ]
}

fn diff_change(field: &str, local: Option<String>, synced: Option<String>) -> RecipeDiffChange {
    RecipeDiffChange {
        field: field.to_owned(),
        changed: local != synced,
        local,
        synced,
    }
}

fn version_label(package: &PackageDefinition) -> String {
    format!("{}:{}-{}", package.epoch, package.version, package.rel)
}

fn presence_label(present: bool) -> String {
    if present { "declared" } else { "absent" }.to_owned()
}
