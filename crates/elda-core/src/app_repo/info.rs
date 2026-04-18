use serde::Serialize;
use serde_json::json;

use crate::app::AppContext;
use crate::app_profile::{PendingSystemChange, ProviderFamilies};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_build::{AlternativeAsset, SystemPackageMetadata};
use elda_install::load_installed_system_metadata;
use elda_recipe::{LuaValue, RecipeDocument, load_recipe};
use elda_repo::resolve_package;

#[derive(Debug, Clone, Serialize)]
struct ResolvedRecipeInfo {
    source: &'static str,
    package: elda_recipe::PackageDefinition,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledFilesSummary {
    total_paths: usize,
    regular_files: usize,
    directories: usize,
    symlinks: usize,
    conffiles: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderAssetVisibility {
    provider_families: ProviderFamilies,
    pending_provider_handlers: Vec<PendingSystemChange>,
    declared_provider_assets: Vec<ProviderAssetEntry>,
    declarative_assets: DeclarativeAssetVisibility,
    installed_system_assets: Option<InstalledSystemAssetVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderAssetEntry {
    provider: String,
    path: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct DeclarativeAssetVisibility {
    sysusers: Option<DeclarativeFamilyVisibility>,
    tmpfiles: Option<DeclarativeFamilyVisibility>,
    alternatives: Vec<AlternativeVisibility>,
    hooks: Vec<HookVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct DeclarativeFamilyVisibility {
    kind: String,
    entry_count: Option<usize>,
    file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AlternativeVisibility {
    name: String,
    link: String,
    path: String,
    priority: i64,
}

#[derive(Debug, Clone, Serialize)]
struct HookVisibility {
    phase: String,
    source_kind: String,
    file: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct InstalledSystemAssetVisibility {
    sysusers: Option<InstalledDeclarativeAssetVisibility>,
    tmpfiles: Option<InstalledDeclarativeAssetVisibility>,
    alternatives: Vec<AlternativeVisibility>,
    hooks: Vec<InstalledHookVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledDeclarativeAssetVisibility {
    path: String,
    source: String,
    line_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledHookVisibility {
    phase: String,
    source: String,
}

impl AppContext {
    pub(crate) fn handle_info(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("info requires one package name".to_owned()))?
            .clone();
        let local_recipe = self.load_local_recipe_for_info(&package_name)?;
        let installed = self.database.installed_package(&package_name)?;
        let installed_files_summary = installed
            .as_ref()
            .map(|_| self.installed_files_summary(&package_name))
            .transpose()?;
        let installed_system_assets =
            load_installed_system_metadata(self.database.layout(), &package_name)?;
        let synced = resolve_package(&self.repo_snapshot_path(), &package_name)
            .ok()
            .flatten();
        let synced_recipe = synced
            .as_ref()
            .map(elda_repo::SyncedPackageRecord::parse_recipe)
            .transpose()?;
        let recipe = resolved_recipe_for_info(&local_recipe, &synced_recipe);
        let recipe_info = recipe.map(|(source, recipe)| ResolvedRecipeInfo {
            source,
            package: recipe.package.clone(),
        });
        let profile = self.resolve_profile_state()?;
        let runtime_view = self.profile_runtime_view(&profile)?;
        let provider_asset_visibility = build_provider_asset_visibility(
            recipe.map(|(_, recipe)| recipe),
            installed_system_assets.as_ref(),
            &runtime_view.provider_families,
            &runtime_view.pending_handler_transitions,
        );
        let found = local_recipe.is_some() || installed.is_some() || synced.is_some();

        Ok(CommandReport {
            area: "info",
            status: if found { "ok" } else { "missing" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported package metadata for `{package_name}`."),
            details: Some(json!({
                "package": package_name,
                "recipe": recipe_info,
                "installed": installed,
                "installed_files_summary": installed_files_summary,
                "synced": synced,
                "provider_asset_visibility": provider_asset_visibility,
            })),
        })
    }

    fn load_local_recipe_for_info(
        &self,
        package_name: &str,
    ) -> Result<Option<RecipeDocument>, CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;
        if recipes_dir.join(package_name).join("pkg.lua").is_file() {
            return Ok(Some(load_recipe(recipes_dir, package_name)?));
        }

        Ok(None)
    }

    fn installed_files_summary(
        &self,
        package_name: &str,
    ) -> Result<InstalledFilesSummary, CoreError> {
        let files = self.database.package_files(package_name)?;
        Ok(InstalledFilesSummary {
            total_paths: files.len(),
            regular_files: files
                .iter()
                .filter(|record| record.path_kind == "file")
                .count(),
            directories: files
                .iter()
                .filter(|record| record.path_kind == "dir")
                .count(),
            symlinks: files
                .iter()
                .filter(|record| record.path_kind == "symlink")
                .count(),
            conffiles: files.iter().filter(|record| record.is_conffile).count(),
        })
    }
}

fn resolved_recipe_for_info<'a>(
    local_recipe: &'a Option<RecipeDocument>,
    synced_recipe: &'a Option<RecipeDocument>,
) -> Option<(&'static str, &'a RecipeDocument)> {
    local_recipe
        .as_ref()
        .map(|recipe| ("local", recipe))
        .or_else(|| synced_recipe.as_ref().map(|recipe| ("synced", recipe)))
}

fn build_provider_asset_visibility(
    recipe: Option<&RecipeDocument>,
    installed_system_assets: Option<&SystemPackageMetadata>,
    provider_families: &ProviderFamilies,
    pending_handlers: &[PendingSystemChange],
) -> ProviderAssetVisibility {
    ProviderAssetVisibility {
        provider_families: provider_families.clone(),
        pending_provider_handlers: pending_handlers
            .iter()
            .filter(|change| {
                matches!(
                    change.kind,
                    "profile-provider-reconciliation" | "init-provider-transition"
                )
            })
            .cloned()
            .collect(),
        declared_provider_assets: Vec::new(),
        declarative_assets: recipe
            .map(describe_recipe_assets)
            .unwrap_or_else(DeclarativeAssetVisibility::default),
        installed_system_assets: installed_system_assets.map(describe_installed_system_assets),
    }
}

fn describe_recipe_assets(recipe: &RecipeDocument) -> DeclarativeAssetVisibility {
    DeclarativeAssetVisibility {
        sysusers: summarize_declarative_family(recipe.package.sysusers.as_ref()),
        tmpfiles: summarize_declarative_family(recipe.package.tmpfiles.as_ref()),
        alternatives: summarize_alternatives(recipe.package.alternatives.as_ref()),
        hooks: summarize_hooks(recipe.package.hooks.as_ref()),
    }
}

fn summarize_declarative_family(value: Option<&LuaValue>) -> Option<DeclarativeFamilyVisibility> {
    match value {
        Some(LuaValue::Array(entries)) => Some(DeclarativeFamilyVisibility {
            kind: "inline".to_owned(),
            entry_count: Some(entries.len()),
            file: None,
        }),
        Some(LuaValue::Table(table)) => Some(DeclarativeFamilyVisibility {
            kind: "file".to_owned(),
            entry_count: None,
            file: table
                .get("file")
                .and_then(as_non_empty_string)
                .map(ToOwned::to_owned),
        }),
        _ => None,
    }
}

fn summarize_alternatives(value: Option<&LuaValue>) -> Vec<AlternativeVisibility> {
    let Some(LuaValue::Array(entries)) = value else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| {
            let LuaValue::Table(table) = entry else {
                return None;
            };
            Some(AlternativeVisibility {
                name: table
                    .get("name")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                link: table
                    .get("link")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                path: table
                    .get("path")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                priority: table
                    .get("priority")
                    .and_then(as_integer)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn summarize_hooks(value: Option<&LuaValue>) -> Vec<HookVisibility> {
    let Some(LuaValue::Table(hooks)) = value else {
        return Vec::new();
    };
    let mut summarized = hooks
        .iter()
        .filter_map(|(phase, spec)| {
            let LuaValue::Table(table) = spec else {
                return None;
            };
            Some(HookVisibility {
                phase: phase.clone(),
                source_kind: if table.get("file").and_then(as_non_empty_string).is_some() {
                    "file".to_owned()
                } else {
                    "lua".to_owned()
                },
                file: table
                    .get("file")
                    .and_then(as_non_empty_string)
                    .map(ToOwned::to_owned),
            })
        })
        .collect::<Vec<_>>();
    summarized.sort_by(|left, right| left.phase.cmp(&right.phase));
    summarized
}

fn describe_installed_system_assets(
    assets: &SystemPackageMetadata,
) -> InstalledSystemAssetVisibility {
    InstalledSystemAssetVisibility {
        sysusers: assets
            .sysusers
            .as_ref()
            .map(|asset| InstalledDeclarativeAssetVisibility {
                path: asset.path.clone(),
                source: asset.source.clone(),
                line_count: non_empty_line_count(&asset.content),
            }),
        tmpfiles: assets
            .tmpfiles
            .as_ref()
            .map(|asset| InstalledDeclarativeAssetVisibility {
                path: asset.path.clone(),
                source: asset.source.clone(),
                line_count: non_empty_line_count(&asset.content),
            }),
        alternatives: assets
            .alternatives
            .iter()
            .map(describe_alternative)
            .collect(),
        hooks: assets
            .hooks
            .iter()
            .map(|hook| InstalledHookVisibility {
                phase: hook.phase.clone(),
                source: hook.source.clone(),
            })
            .collect(),
    }
}

fn describe_alternative(alternative: &AlternativeAsset) -> AlternativeVisibility {
    AlternativeVisibility {
        name: alternative.name.clone(),
        link: alternative.link.clone(),
        path: alternative.path.clone(),
        priority: alternative.priority,
    }
}

fn non_empty_line_count(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn as_non_empty_string(value: &LuaValue) -> Option<&str> {
    match value {
        LuaValue::String(value) if !value.trim().is_empty() => Some(value.as_str()),
        _ => None,
    }
}

fn as_integer(value: &LuaValue) -> Option<i64> {
    match value {
        LuaValue::Integer(value) => Some(*value),
        _ => None,
    }
}
