use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::artifacts::lock_document_path;
use super::model::CiSubmissionRecord;
use super::publish::publish_workspace;
use super::publish_plan::{PlannedCiWork, plan_ci_work};
use super::store::{load_submissions, save_submission};
use super::workspace::{CiWorkspacePaths, current_unix_timestamp};

pub(super) fn is_pending_submission(submission: &CiSubmissionRecord) -> bool {
    matches!(
        submission.state.as_str(),
        "ready-local" | "submitted" | "submitted-for-review" | "queued" | "retry-requested"
    )
}

impl AppContext {
    pub(super) fn handle_ci_scheduler_run(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let workspace = CiWorkspacePaths::new(self.database.layout());
        workspace.ensure_exists()?;
        let submissions = load_submissions(&workspace)?;
        let pending = submissions
            .into_iter()
            .filter(is_pending_submission)
            .collect::<Vec<_>>();

        if request.dry_run {
            return Ok(CommandReport {
                area: "ci",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!(
                    "ci scheduler would process {} pending submission(s).",
                    pending.len()
                ),
                details: Some(json!({
                    "pending_count": pending.len(),
                    "pending": pending,
                    "workspace": workspace,
                })),
            });
        }

        if pending.is_empty() {
            return Ok(CommandReport {
                area: "ci",
                status: "ok",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: false,
                summary: "no pending ci submissions were waiting in the local scheduler."
                    .to_owned(),
                details: Some(json!({
                    "processed": [],
                    "failed": [],
                    "pending_count": 0,
                    "workspace": workspace,
                })),
            });
        }

        let mut processed = Vec::new();
        let mut failed = Vec::new();
        for submission in pending {
            let submission_id = submission.id.clone();
            let plan = match plan_ci_work(self, &submission.requested_targets) {
                Ok(plan) => plan,
                Err(error) => {
                    let failed_submission =
                        record_scheduler_failure(&workspace, submission, &error)?;
                    failed.push(json!({
                        "id": submission_id,
                        "state": failed_submission.state,
                        "attempts": failed_submission.attempts,
                        "error": error.to_string(),
                    }));
                    continue;
                }
            };

            match process_submission(self, &workspace, &plan, submission) {
                Ok(updated) => processed.push(json!({
                    "id": updated.id,
                    "state": updated.state,
                    "attempts": updated.attempts,
                    "planned_layers": updated.planned_layers,
                    "completed_layers": updated.completed_layers,
                    "published_packages": updated.published_packages.len(),
                })),
                Err(error) => {
                    let latest = load_submissions(&workspace)?
                        .into_iter()
                        .find(|entry| entry.id == submission_id)
                        .ok_or_else(|| {
                            CoreError::Operator(format!(
                                "ci scheduler lost submission record `{submission_id}`"
                            ))
                        })?;
                    failed.push(json!({
                        "id": latest.id,
                        "state": latest.state,
                        "attempts": latest.attempts,
                        "error": error.to_string(),
                    }));
                }
            }
        }

        Ok(CommandReport {
            area: "ci",
            status: if failed.is_empty() { "ok" } else { "issues" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!(
                "ci scheduler processed {} submission(s): {} published, {} failed.",
                processed.len() + failed.len(),
                processed.len(),
                failed.len()
            ),
            details: Some(json!({
                "processed": processed,
                "failed": failed,
                "pending_count": 0,
                "workspace": workspace,
            })),
        })
    }
}

pub(super) fn process_submission(
    app: &AppContext,
    workspace: &CiWorkspacePaths,
    plan: &PlannedCiWork,
    mut submission: CiSubmissionRecord,
) -> Result<CiSubmissionRecord, CoreError> {
    let now = current_unix_timestamp();
    submission.state = "building".to_owned();
    submission.attempts += 1;
    submission.started_at = Some(now);
    submission.completed_at = None;
    submission.updated_at = now;
    submission.completed_layers = 0;
    submission.last_error = None;
    save_submission(workspace, &submission)?;
    append_scheduler_log(
        &submission.log_path,
        &format!(
            "scheduler_start attempt={} submission={} layers={} targets={:?}",
            submission.attempts,
            submission.id,
            submission.planned_layers,
            submission.requested_targets
        ),
    )?;

    match publish_workspace(app, workspace, plan, &submission.id) {
        Ok(published) => {
            let completed_at = current_unix_timestamp();
            submission.state = "published".to_owned();
            submission.updated_at = completed_at;
            submission.completed_at = Some(completed_at);
            submission.completed_layers = submission.planned_layers;
            submission.published_packages = published.packages;
            submission.lock_path = Some(lock_document_path(workspace));
            submission.index_path = Some(workspace.index_path.clone());
            submission.signature_path = Some(workspace.signature_path.clone());
            submission.trusted_key_fingerprint = Some(published.trusted_key_fingerprint);
            submission.repo_commit = published.repo_commit;
            save_submission(workspace, &submission)?;
            append_scheduler_log(
                &submission.log_path,
                &format!(
                    "scheduler_complete attempt={} submission={} packages={} layers={}/{}",
                    submission.attempts,
                    submission.id,
                    submission.published_packages.len(),
                    submission.completed_layers,
                    submission.planned_layers,
                ),
            )?;
            Ok(submission)
        }
        Err(error) => {
            let failed = record_scheduler_failure(workspace, submission, &error)?;
            Err(CoreError::Operator(
                failed.last_error.unwrap_or_else(|| error.to_string()),
            ))
        }
    }
}

fn record_scheduler_failure(
    workspace: &CiWorkspacePaths,
    mut submission: CiSubmissionRecord,
    error: &CoreError,
) -> Result<CiSubmissionRecord, CoreError> {
    let now = current_unix_timestamp();
    if submission.started_at.is_none() {
        submission.attempts += 1;
        submission.started_at = Some(now);
    }
    submission.state = "failed".to_owned();
    submission.updated_at = now;
    submission.completed_at = Some(now);
    submission.last_error = Some(error.to_string());
    save_submission(workspace, &submission)?;
    append_scheduler_log(
        &submission.log_path,
        &format!(
            "scheduler_failed attempt={} submission={} error={}",
            submission.attempts, submission.id, error
        ),
    )?;
    Ok(submission)
}

pub(super) fn state_counts(submissions: &[CiSubmissionRecord]) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for submission in submissions {
        *counts.entry(submission.state.clone()).or_insert(0) += 1;
    }
    counts
}

fn append_scheduler_log(path: &Path, line: &str) -> Result<(), CoreError> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}
