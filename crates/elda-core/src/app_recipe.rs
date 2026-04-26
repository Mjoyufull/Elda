use std::path::{Path, PathBuf};

use serde_json::json;

use crate::app::AppContext;
use crate::editor::open_path_in_editor;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_recipe::{
    add_recipe, add_vendor_recipe, check_local_recipes, export_vendor_source, import_vendor_source,
};

impl AppContext {
    pub(crate) fn handle_recipe_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (target, recipe_kind) = parse_recipe_add_request(&request)?;
        let report = add_recipe(
            &self.database.layout().recipes_dir,
            &target,
            recipe_kind.as_deref(),
        )?;

        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("prepared local recipe `{}`.", report.recipe_name),
            details: Some(json!({ "recipe": report })),
        })
    }

    pub(crate) fn handle_recipe_check(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let target = request.operands.first().map(String::as_str);
        let report = check_local_recipes(&self.database.layout().recipes_dir, target)?;
        let status = if report
            .issues
            .iter()
            .any(|issue| issue.severity == elda_recipe::IssueSeverity::Error)
        {
            "invalid"
        } else {
            "ok"
        };

        Ok(CommandReport {
            area: "recipe",
            status,
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("checked {} local recipe(s).", report.recipes.len()),
            details: Some(json!({ "check": report })),
        })
    }

    pub(crate) fn handle_recipe_edit(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let recipe_name =
            request.operands.first().cloned().ok_or_else(|| {
                CoreError::Operator("rc edit requires one package name".to_owned())
            })?;
        let recipe_dir = recipe_directory(&self.database.layout().recipes_dir, &recipe_name)?;
        if request.dry_run {
            return Ok(CommandReport {
                area: "recipe",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("would open local recipe `{recipe_name}` in the selected editor."),
                details: Some(json!({
                    "recipe": {
                        "package_name": recipe_name,
                        "path": recipe_dir,
                    },
                })),
            });
        }
        let editor = open_path_in_editor(&recipe_dir)?;

        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("opened local recipe `{recipe_name}` in the selected editor."),
            details: Some(json!({
                "recipe": {
                    "package_name": recipe_name,
                    "path": recipe_dir,
                    "editor": {
                        "source": editor.source(),
                        "program": editor.display_program(),
                    },
                },
            })),
        })
    }

    pub(crate) fn handle_recipe_ls(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        if !request.operands.is_empty() {
            return Err(CoreError::Operator(
                "rc ls does not take operands".to_owned(),
            ));
        }

        self.database.bootstrap()?;
        let recipes_dir = self.database.layout().recipes_dir.clone();
        let local = crate::recipe_catalog::list_local_recipe_names(&recipes_dir)?;
        let synced = crate::recipe_catalog::list_synced_pkg_names(&self.repo_snapshot_path())?;

        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "{} local recipe(s); {} synced install name(s).",
                local.len(),
                synced.len(),
            ),
            details: Some(json!({
                "catalog": {
                    "recipes_dir": recipes_dir.display().to_string(),
                    "local_recipes": local,
                    "synced_packages": synced,
                }
            })),
        })
    }

    pub(crate) fn handle_recipe_rm(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let operands = request.operands.clone();
        let pkgname = operands
            .first()
            .map(String::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| CoreError::Operator("rc rm requires one package name".to_owned()))?;
        if operands.len() > 1 {
            return Err(CoreError::Operator(
                "rc rm accepts exactly one package name".to_owned(),
            ));
        }

        self.database.bootstrap()?;
        if self
            .database
            .installed_package(pkgname)
            .map_err(CoreError::Db)?
            .is_some()
        {
            return Err(CoreError::Operator(format!(
                "`{pkgname}` is still installed; run `elda rm {pkgname}` before removing local recipe metadata"
            )));
        }

        let recipes_dir = self.database.layout().recipes_dir.clone();
        if request.dry_run {
            crate::recipe_catalog::validate_recipe_pkgname(pkgname)?;
            let path = recipes_dir.join(pkgname);
            if !path.join("pkg.lua").is_file() {
                return Err(CoreError::Operator(format!(
                    "no local recipe tree at `{}` (missing pkg.lua)",
                    path.display()
                )));
            }
            return Ok(CommandReport {
                area: "recipe",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("would remove local recipe metadata for `{pkgname}`."),
                details: Some(json!({
                    "removed": {
                        "pkgname": pkgname,
                        "path": path.display().to_string(),
                    }
                })),
            });
        }

        let removed_path =
            crate::recipe_catalog::remove_local_recipe_directory(&recipes_dir, pkgname)?;

        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!("removed local recipe metadata for `{pkgname}`."),
            details: Some(json!({
                "removed": {
                    "pkgname": pkgname,
                    "path": removed_path.display().to_string(),
                }
            })),
        })
    }

    pub(crate) fn handle_vendor_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let parsed = self.parse_vendor_add_request(&request)?;
        let report = add_vendor_recipe(
            &self.database.layout().recipes_dir,
            &parsed.package_name,
            &parsed.source,
            parsed.binary.as_deref(),
            parsed.asset.as_deref(),
        )?;

        Ok(CommandReport {
            area: "vendor",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("wrote vendor recipe `{}`.", report.package_name),
            details: Some(json!({ "vendor": report })),
        })
    }

    pub(crate) fn handle_vendor_import(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let input_path = request.operands.first().ok_or_else(|| {
            CoreError::Operator("vendor import requires a manifest path".to_owned())
        })?;
        let report =
            import_vendor_source(&self.database.layout().recipes_dir, Path::new(input_path))?;

        Ok(CommandReport {
            area: "vendor",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("imported {} vendor recipe(s).", report.packages.len()),
            details: Some(json!({ "import": report })),
        })
    }

    pub(crate) fn handle_vendor_export(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (output_path, package_names) = request.operands.split_first().ok_or_else(|| {
            CoreError::Operator(
                "vendor export requires an output path and package names".to_owned(),
            )
        })?;
        if package_names.is_empty() {
            return Err(CoreError::Operator(
                "vendor export requires at least one package name".to_owned(),
            ));
        }

        let report = export_vendor_source(
            &self.database.layout().recipes_dir,
            Path::new(output_path),
            package_names,
        )?;

        Ok(CommandReport {
            area: "vendor",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("exported {} vendor recipe(s).", report.packages.len()),
            details: Some(json!({ "export": report })),
        })
    }
}

fn recipe_directory(recipes_dir: &Path, recipe_name: &str) -> Result<PathBuf, CoreError> {
    if recipe_name.trim().is_empty() {
        return Err(CoreError::Operator(
            "rc edit does not accept an empty package name".to_owned(),
        ));
    }

    let recipe_dir = recipes_dir.join(recipe_name);
    if !recipe_dir.join("pkg.lua").is_file() {
        return Err(CoreError::Operator(format!(
            "local recipe `{recipe_name}` does not exist under {}",
            recipes_dir.display(),
        )));
    }

    Ok(recipe_dir)
}

fn parse_recipe_add_request(
    request: &CommandRequest,
) -> Result<(String, Option<String>), CoreError> {
    let mut operands = request.operands.iter();
    let target = operands
        .next()
        .cloned()
        .ok_or_else(|| CoreError::Operator("rc add requires one input".to_owned()))?;
    let mut recipe_kind = None;

    while let Some(operand) = operands.next() {
        match operand.as_str() {
            "--kind" => {
                let value = operands.next().ok_or_else(|| {
                    CoreError::Operator("rc add --kind requires one recipe kind".to_owned())
                })?;
                if value.trim().is_empty() {
                    return Err(CoreError::Operator(
                        "rc add --kind does not accept an empty value".to_owned(),
                    ));
                }
                if recipe_kind.replace(value.clone()).is_some() {
                    return Err(CoreError::Operator(
                        "rc add accepts at most one `--kind` value".to_owned(),
                    ));
                }
            }
            other => {
                return Err(CoreError::Operator(format!(
                    "unexpected `rc add` operand or flag `{other}`"
                )));
            }
        }
    }

    Ok((target, recipe_kind))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::recipe_directory;

    #[test]
    fn recipe_directory_requires_existing_pkg_lua() {
        let tempdir = tempfile::TempDir::new().expect("tempdir should exist");

        let error = recipe_directory(tempdir.path(), "missing")
            .expect_err("missing recipe should be rejected");

        assert!(
            error
                .to_string()
                .contains("local recipe `missing` does not exist")
        );
    }

    #[test]
    fn recipe_directory_returns_recipe_root() {
        let tempdir = tempfile::TempDir::new().expect("tempdir should exist");
        let recipe_dir = tempdir.path().join("example");
        std::fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
        std::fs::write(recipe_dir.join("pkg.lua"), "pkg = { name = 'example' }")
            .expect("pkg.lua should exist");

        let resolved =
            recipe_directory(tempdir.path(), "example").expect("recipe directory should resolve");

        assert_eq!(resolved, PathBuf::from(recipe_dir));
    }
}
