use std::path::Path;

use serde_json::json;

use crate::app::AppContext;
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
