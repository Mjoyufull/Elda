use serde_json::json;

use crate::app::AppContext;
use crate::app_ci::{PlannedCiWork, plan_ci_work};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

pub(crate) fn publish_plan_for_targets(
    app: &AppContext,
    targets: &[String],
) -> Result<PlannedCiWork, CoreError> {
    plan_ci_work(app, targets)
}

impl AppContext {
    pub(crate) fn handle_publish_plan(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let (targets, channel) = parse_publish_scope(&request)?;
        let plan = if let Some(tree_path) = parse_flag_value(&request, "--tree") {
            let tree =
                crate::app_host::resolve_recipe_tree(std::path::Path::new(&tree_path), "packages")?;
            let names = crate::app_host::discover_package_names(&tree, &targets)?;
            crate::app_host::sync_tree_packages_to_recipes(
                &tree,
                &self.database.layout().recipes_dir,
                &names,
            )?;
            publish_plan_for_targets(self, &names)?
        } else if targets.is_empty() {
            return Err(CoreError::Operator(
                "`publish plan` requires package names or `--tree <path>`".to_owned(),
            ));
        } else {
            publish_plan_for_targets(self, &targets)?
        };

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "publish plan for channel `{channel}`: {} package(s), {} requested.",
                plan.packages.len(),
                plan.requested_targets.len()
            ),
            details: Some(json!({
                "channel": channel,
                "requested_targets": plan.requested_targets,
                "packages": plan.packages.iter().map(|pkg| json!({
                    "package": pkg.package_name,
                    "layer": pkg.layer,
                    "recipe_path": pkg.recipe_path,
                })).collect::<Vec<_>>(),
            })),
        })
    }
}

pub(crate) fn parse_publish_scope(
    request: &CommandRequest,
) -> Result<(Vec<String>, String), CoreError> {
    let channel = parse_flag_value(request, "--channel").unwrap_or_else(|| "stable".to_owned());
    let targets = request
        .operands
        .iter()
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .collect();
    Ok((targets, channel))
}

pub(crate) fn parse_flag_value(request: &CommandRequest, flag: &str) -> Option<String> {
    let mut operands = request.operands.iter();
    while let Some(operand) = operands.next() {
        if operand == flag {
            return operands.next().cloned();
        }
    }
    None
}
