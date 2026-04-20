use serde::Serialize;
use serde_json::{Value, json};

use crate::app::AppContext;
use crate::app_repo::info_visibility::build_provider_asset_visibility;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::load_installed_system_metadata;
use elda_recipe::{RecipeDocument, load_recipe};
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
        let provider_asset_visibility: Value = build_provider_asset_visibility(
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
