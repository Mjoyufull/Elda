use std::path::Path;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_recipe::{add_vendor_recipe, export_vendor_source, import_vendor_source};

impl AppContext {
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
            parsed.replace,
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
        let (input_path, replace) = parse_vendor_import_request(&request)?;
        let report = import_vendor_source(
            &self.database.layout().recipes_dir,
            Path::new(&input_path),
            replace,
        )?;

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

fn parse_vendor_import_request(request: &CommandRequest) -> Result<(String, bool), CoreError> {
    let mut input_path = None;
    let mut replace = false;

    for operand in &request.operands {
        match operand.as_str() {
            "--replace" => replace = true,
            _ if input_path.is_none() => input_path = Some(operand.clone()),
            other => {
                return Err(CoreError::Operator(format!(
                    "unexpected `vendor import` operand or flag `{other}`"
                )));
            }
        }
    }

    let input_path = input_path
        .ok_or_else(|| CoreError::Operator("vendor import requires a manifest path".to_owned()))?;

    Ok((input_path, replace))
}
