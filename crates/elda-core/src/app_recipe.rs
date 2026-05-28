use std::path::{Path, PathBuf};

use serde_json::json;

use crate::app::AppContext;
use crate::editor::open_path_in_editor;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_recipe::{
    ImportOptions, add_recipe_with_options, check_local_recipes, format_recipe_file,
    normalize_recipe_file, write_formatted_recipe,
};

impl AppContext {
    pub(crate) fn handle_recipe_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (target, recipe_kind, replace) = parse_recipe_add_request(&request)?;
        let report = match add_recipe_with_options(
            &self.database.layout().recipes_dir,
            &target,
            recipe_kind.as_deref(),
            &ImportOptions {
                strategy_priority: self.config.metadata.link_strategy_priority.clone(),
                release_binary_format_priority: self
                    .config
                    .metadata
                    .release_binary_format_priority
                    .clone(),
                replace,
                ..ImportOptions::default()
            },
        )? {
            elda_recipe::ImportResult::Single(r) => r,
            elda_recipe::ImportResult::Bulk(_) => {
                return Err(CoreError::Operator(
                    "Bulk import via `rc add` is not supported. Use `elda add <url>`".to_owned(),
                ));
            }
        };

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
        let strict = request.operands.iter().any(|operand| operand == "--strict");
        let target = request
            .operands
            .iter()
            .find(|operand| !operand.starts_with("--"))
            .map(String::as_str);
        let report = check_local_recipes(&self.database.layout().recipes_dir, target)?;
        let has_errors = report
            .issues
            .iter()
            .any(|issue| issue.severity == elda_recipe::IssueSeverity::Error);
        let has_warnings = report
            .issues
            .iter()
            .any(|issue| issue.severity == elda_recipe::IssueSeverity::Warning);
        let status = if has_errors || strict && has_warnings {
            "invalid"
        } else {
            "ok"
        };

        Ok(CommandReport {
            area: "recipe",
            status,
            exit_status: if has_errors || (strict && has_warnings) {
                ExitStatus::OperatorFailure
            } else {
                ExitStatus::Success
            },
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "checked {} local recipe(s){}.",
                report.recipes.len(),
                if strict { " with --strict" } else { "" }
            ),
            details: Some(json!({ "check": report, "strict": strict })),
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
        let local_entries = crate::recipe_catalog::list_local_recipe_entries(&recipes_dir)?;
        let synced_entries =
            crate::recipe_catalog::list_synced_pkg_entries(&self.repo_snapshot_path())?;
        let local = local_entries
            .iter()
            .map(|entry| entry.pkgname.clone())
            .collect::<Vec<_>>();
        let synced = synced_entries
            .iter()
            .map(|entry| entry.pkgname.clone())
            .collect::<Vec<_>>();

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
                    "local_entries": local_entries,
                    "synced_entries": synced_entries,
                }
            })),
        })
    }

    pub(crate) fn handle_recipe_format(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let package = recipe_target_from_request(&request, "rc format")?;
        let pkg_lua =
            recipe_directory(&self.database.layout().recipes_dir, &package)?.join("pkg.lua");
        if request.dry_run {
            return Ok(planned_recipe_rewrite_report(
                &request, &package, &pkg_lua, "format",
            ));
        }
        let formatted = format_recipe_file(&pkg_lua).map_err(CoreError::from)?;
        write_formatted_recipe(&pkg_lua, &formatted).map_err(CoreError::from)?;
        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!("formatted local recipe `{package}`."),
            details: Some(json!({
                "package": package,
                "path": pkg_lua.display().to_string(),
                "action": "format",
            })),
        })
    }

    pub(crate) fn handle_recipe_normalize(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let package = recipe_target_from_request(&request, "rc normalize")?;
        let pkg_lua =
            recipe_directory(&self.database.layout().recipes_dir, &package)?.join("pkg.lua");
        if request.dry_run {
            return Ok(planned_recipe_rewrite_report(
                &request,
                &package,
                &pkg_lua,
                "normalize",
            ));
        }
        let normalized = normalize_recipe_file(&pkg_lua).map_err(CoreError::from)?;
        write_formatted_recipe(&pkg_lua, &normalized).map_err(CoreError::from)?;
        Ok(CommandReport {
            area: "recipe",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!("normalized local recipe `{package}`."),
            details: Some(json!({
                "package": package,
                "path": pkg_lua.display().to_string(),
                "action": "normalize",
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
}

fn recipe_target_from_request(
    request: &CommandRequest,
    command: &str,
) -> Result<String, CoreError> {
    request
        .operands
        .first()
        .cloned()
        .filter(|name| !name.starts_with("--"))
        .ok_or_else(|| CoreError::Operator(format!("{command} requires one package name")))
}

fn planned_recipe_rewrite_report(
    request: &CommandRequest,
    package: &str,
    pkg_lua: &Path,
    action: &str,
) -> CommandReport {
    CommandReport {
        area: "recipe",
        status: "planned",
        exit_status: ExitStatus::Success,
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
        output_mode: request.output_mode,
        dry_run: true,
        summary: format!("would {action} local recipe `{package}`."),
        details: Some(json!({
            "package": package,
            "path": pkg_lua.display().to_string(),
            "action": action,
        })),
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
) -> Result<(String, Option<String>, bool), CoreError> {
    let mut operands = request.operands.iter();
    let target = operands
        .next()
        .cloned()
        .ok_or_else(|| CoreError::Operator("rc add requires one input".to_owned()))?;
    let mut recipe_kind = None;
    let mut replace = false;

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
            "--replace" => {
                replace = true;
            }
            other => {
                return Err(CoreError::Operator(format!(
                    "unexpected `rc add` operand or flag `{other}`"
                )));
            }
        }
    }

    Ok((target, recipe_kind, replace))
}

#[cfg(test)]
mod tests {
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

        assert_eq!(resolved, recipe_dir);
    }
}
