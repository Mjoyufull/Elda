use std::fs;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::model::CiSubmissionRecord;
use super::publish_plan::{plan_ci_work, resolve_ci_targets};
use super::remote_push::{prepare_submission_checkout, push_submission_remote};
use super::scheduler::{is_pending_submission, process_submission, state_counts};
use super::store::{
    branch_name, find_submission, load_batches, load_submissions, save_submission, submission_id,
    submission_mode_name,
};
use super::workspace::{CiWorkspacePaths, current_unix_timestamp, sync_recipe_into_packages_repo};

impl AppContext {
    pub(super) fn handle_ci_submit(
        &self,
        request: CommandRequest,
        immediate: bool,
        batch_name: Option<String>,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let workspace = CiWorkspacePaths::new(self.database.layout());
        workspace.ensure_exists()?;
        let targets = resolve_ci_targets(self, &request.operands)?;
        let plan = plan_ci_work(self, &targets)?;
        let submission_config = self.config.submission.resolve_target();
        let submission_mode = submission_config.mode;
        let submission_branch_name = branch_name(&plan.requested_targets, batch_name.as_deref());
        let remote_name = submission_config.remote_name.as_str();
        let base_branch = submission_config.base_branch.as_str();
        let planned_layers = plan
            .packages
            .iter()
            .map(|package| package.layer)
            .max()
            .unwrap_or(0)
            + 1;

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
                    "ci would process {} package(s) across {} build layer(s).",
                    plan.packages.len(),
                    planned_layers,
                ),
                details: Some(json!({
                    "requested_targets": plan.requested_targets,
                    "packages": plan
                        .packages
                        .iter()
                        .map(|package| json!({
                            "pkgname": package.package_name,
                            "layer": package.layer,
                        }))
                        .collect::<Vec<_>>(),
                    "workspace": workspace,
                })),
            });
        }

        prepare_submission_checkout(
            &workspace,
            &submission_branch_name,
            submission_mode,
            base_branch,
        )?;

        for package in &plan.packages {
            sync_recipe_into_packages_repo(
                &workspace,
                &self.database.layout().recipes_dir,
                &package.package_name,
            )?;
        }
        let remote_push = push_submission_remote(
            &workspace,
            remote_name,
            &submission_branch_name,
            submission_mode,
            base_branch,
        )?;
        let submission_id = submission_id(&plan.requested_targets);
        let log_path = workspace.logs_dir.join(format!("{submission_id}.log"));
        fs::write(
            &log_path,
            format!(
                "targets={:?}\npackages={:?}\nimmediate={immediate}\nmode={}\nbranch={}\nbase_branch={}\nremote={}\nremote_ref={}\n",
                plan.requested_targets,
                plan.packages
                    .iter()
                    .map(|package| package.package_name.as_str())
                    .collect::<Vec<_>>(),
                submission_mode_name(submission_mode),
                submission_branch_name,
                base_branch,
                remote_push
                    .as_ref()
                    .map(|push| push.remote_url.as_str())
                    .unwrap_or("local-only"),
                remote_push
                    .as_ref()
                    .map(|push| push.pushed_ref.as_str())
                    .unwrap_or("none"),
            ),
        )?;

        let now = current_unix_timestamp();
        let mut submission = CiSubmissionRecord {
            id: submission_id.clone(),
            requested_targets: plan.requested_targets.clone(),
            packages: plan
                .packages
                .iter()
                .map(|package| package.package_name.clone())
                .collect(),
            branch_name: submission_branch_name,
            target_branch: base_branch.to_owned(),
            mode: submission_mode_name(submission_mode).to_owned(),
            state: if immediate {
                "queued".to_owned()
            } else if let Some(push) = &remote_push {
                if submission_mode == crate::config::SubmissionMode::Pr {
                    if push.pushed_ref == format!("refs/heads/{}", base_branch) {
                        "submitted".to_owned()
                    } else {
                        "submitted-for-review".to_owned()
                    }
                } else {
                    "pushed".to_owned()
                }
            } else {
                "ready-local".to_owned()
            },
            immediate,
            batch_name,
            created_at: now,
            updated_at: now,
            attempts: 0,
            planned_layers,
            completed_layers: 0,
            queued_at: Some(now),
            started_at: None,
            completed_at: None,
            last_error: None,
            issues: Vec::new(),
            published_packages: Vec::new(),
            lock_path: None,
            index_path: None,
            signature_path: None,
            log_path: log_path.clone(),
            packages_repo_path: workspace.packages_repo_dir.clone(),
            trusted_key_fingerprint: None,
            repo_commit: None,
            remote_name: remote_push.as_ref().map(|push| push.remote_name.clone()),
            remote_url: remote_push.as_ref().map(|push| push.remote_url.clone()),
            pushed_ref: remote_push.as_ref().map(|push| push.pushed_ref.clone()),
            pushed_commit: remote_push
                .as_ref()
                .and_then(|push| push.pushed_commit.clone()),
            pushed_at: remote_push.as_ref().map(|_| current_unix_timestamp()),
            review_url: None,
            review_kind: None,
            review_id: None,
            review_created_at: None,
        };

        save_submission(&workspace, &submission)?;

        if immediate {
            submission = process_submission(self, &workspace, &plan, submission)?;
        }

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: if immediate {
                format!(
                    "published {} package(s) through the local ci scheduler.",
                    submission.published_packages.len()
                )
            } else if submission.remote_url.is_some() {
                format!(
                    "registered ci submission `{}` and pushed it to `{}`.",
                    submission.id,
                    submission.remote_name.as_deref().unwrap_or(remote_name)
                )
            } else {
                format!("registered ci submission `{}`.", submission.id)
            },
            details: Some(json!({
                "submission": submission,
                "workspace": workspace,
            })),
        })
    }

    pub(super) fn handle_ci_status(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let selection = request.operands.first().cloned();
        let submissions = load_submissions(&workspace)?;
        let batches = load_batches(&workspace)?;

        let details = if let Some(selection) = selection {
            if let Some(submission) = find_submission(&submissions, &selection) {
                json!({ "submission": submission, "workspace": workspace })
            } else if let Some(batch) = batches.iter().find(|batch| batch.name == selection) {
                json!({ "batch": batch, "workspace": workspace })
            } else {
                return Err(CoreError::Operator(format!(
                    "no ci submission or batch named `{selection}` exists"
                )));
            }
        } else {
            let pending_count = submissions
                .iter()
                .filter(|submission| is_pending_submission(submission))
                .count();
            json!({
                "submissions": submissions,
                "batches": batches,
                "state_counts": state_counts(&load_submissions(&workspace)?),
                "pending_count": pending_count,
                "workspace": workspace,
            })
        };

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "reported current ci workspace state.".to_owned(),
            details: Some(details),
        })
    }

    pub(super) fn handle_ci_retry(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let submissions = load_submissions(&workspace)?;
        let selection = request.operands.first().cloned().ok_or_else(|| {
            CoreError::Operator("ci retry requires a submission id or package name".to_owned())
        })?;
        let mut submission = find_submission(&submissions, &selection).ok_or_else(|| {
            CoreError::Operator(format!("no ci submission matches `{selection}`"))
        })?;
        if request.dry_run {
            return Ok(CommandReport {
                area: "ci",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("ci retry would requeue `{}`.", submission.id),
                details: Some(json!({ "submission": submission, "workspace": workspace })),
            });
        }

        submission.state = "retry-requested".to_owned();
        submission.updated_at = current_unix_timestamp();
        submission.queued_at = Some(submission.updated_at);
        submission.completed_layers = 0;
        submission.completed_at = None;
        submission.last_error = None;
        save_submission(&workspace, &submission)?;

        let plan = plan_ci_work(self, &submission.requested_targets)?;
        let submission = process_submission(self, &workspace, &plan, submission)?;

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!("retried ci submission `{}`.", submission.id),
            details: Some(json!({ "submission": submission, "workspace": workspace })),
        })
    }

    pub(super) fn handle_ci_logs(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let submissions = load_submissions(&workspace)?;
        let selection = request.operands.first().cloned().ok_or_else(|| {
            CoreError::Operator("ci logs requires a submission id or package name".to_owned())
        })?;
        let submission = find_submission(&submissions, &selection).ok_or_else(|| {
            CoreError::Operator(format!("no ci submission matches `{selection}`"))
        })?;
        let content = fs::read_to_string(&submission.log_path)?;

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported logs for `{}`.", submission.id),
            details: Some(json!({
                "submission_id": submission.id,
                "state": submission.state,
                "attempts": submission.attempts,
                "log_path": submission.log_path,
                "content": content,
            })),
        })
    }
}
