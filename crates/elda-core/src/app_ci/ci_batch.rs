use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::model::CiBatchRecord;
use super::publish_plan::resolve_ci_targets;
use super::store::{load_batch, save_batch};
use super::workspace::{CiWorkspacePaths, current_unix_timestamp};

impl AppContext {
    pub(super) fn handle_ci_batch_new(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let batch_name =
            request.operands.first().cloned().ok_or_else(|| {
                CoreError::Operator("ci batch new requires a batch name".to_owned())
            })?;
        let workspace = CiWorkspacePaths::new(self.database.layout());
        workspace.ensure_exists()?;
        let batch = CiBatchRecord {
            name: batch_name.clone(),
            packages: Vec::new(),
            last_submission_id: None,
            state: "draft".to_owned(),
            updated_at: current_unix_timestamp(),
        };
        save_batch(&workspace, &batch)?;

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("created ci batch `{batch_name}`."),
            details: Some(json!({ "batch": batch })),
        })
    }

    pub(super) fn handle_ci_batch_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (batch_name, package_operands) = request.operands.split_first().ok_or_else(|| {
            CoreError::Operator("ci batch add requires a batch name and package names".to_owned())
        })?;
        let batch_name = batch_name.clone();
        if package_operands.is_empty() {
            return Err(CoreError::Operator(
                "ci batch add requires at least one package name".to_owned(),
            ));
        }

        let workspace = CiWorkspacePaths::new(self.database.layout());
        let mut batch = load_batch(&workspace, &batch_name)?;
        let packages = resolve_ci_targets(self, package_operands)?;
        batch.packages.extend(packages);
        batch.packages.sort();
        batch.packages.dedup();
        batch.updated_at = current_unix_timestamp();
        batch.state = "ready".to_owned();
        save_batch(&workspace, &batch)?;

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("updated ci batch `{batch_name}`."),
            details: Some(json!({ "batch": batch })),
        })
    }

    pub(super) fn handle_ci_batch_push(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let batch_name =
            request.operands.first().cloned().ok_or_else(|| {
                CoreError::Operator("ci batch push requires a batch name".to_owned())
            })?;
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let mut batch = load_batch(&workspace, &batch_name)?;
        let publish_request = CommandRequest::new(
            vec!["ci".to_owned(), "run".to_owned()],
            batch.packages.clone(),
            request.output_mode,
            request.dry_run,
        );
        let report = self.handle_ci_submit(publish_request, true, Some(batch_name.clone()))?;
        if let Some(details) = &report.details
            && let Some(submission_id) = details
                .get("submission")
                .and_then(|submission| submission.get("id"))
                .and_then(|value| value.as_str())
        {
            batch.last_submission_id = Some(submission_id.to_owned());
            batch.state = "published".to_owned();
            batch.updated_at = current_unix_timestamp();
            save_batch(&workspace, &batch)?;
        }

        Ok(report)
    }
}
