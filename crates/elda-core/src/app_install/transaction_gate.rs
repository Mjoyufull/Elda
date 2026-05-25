use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::install_confirm::confirm_install_execution;
use super::preflight::install_preflight_report;
use super::progress::planned_activation_backend;
use super::report::{install_execution_decision, planned_install_action_json};
use crate::app::{AppContext, PlannedInstallAction};
use crate::app_confirm::{
    ConfirmResponse, interactive_session, read_yne_after_frame, require_interactive_confirmation,
};
use crate::app_render_install::render_install_plan_frame;
use crate::editor::open_path_in_editor;
use crate::error::CoreError;
use crate::render_style::highlight_operator_frame;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionConfirmStamp {
    plan_hash: String,
    command_path: Vec<String>,
    operands: Vec<String>,
}

pub(super) fn confirm_install_transaction(
    app: &AppContext,
    request: &CommandRequest,
    install_plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    if request.dry_run {
        return Ok(());
    }
    if request.output_mode != OutputMode::Human {
        return Ok(());
    }
    if !interactive_session(request) {
        if cfg!(test) {
            return confirm_install_execution(app, request, install_plan);
        }
        return require_interactive_confirmation(request, "install");
    }

    let plan_report = build_install_plan_report(app, request, install_plan)?;
    if let Some(rendered) = render_install_plan_frame(&plan_report) {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(stdout, "{}", highlight_operator_frame(&rendered))?;
        stdout.flush()?;
    }

    loop {
        match read_yne_after_frame()? {
            ConfirmResponse::Accept => {
                let data_dir = app.database.layout().data_dir.clone();
                let plan_hash = plan_confirmation_hash(install_plan);
                write_transaction_confirmation(&data_dir, request, &plan_hash)?;
                return confirm_install_execution(app, request, install_plan);
            }
            ConfirmResponse::Decline => {
                return Err(CoreError::Operator(
                    "install cancelled at transaction plan gate".to_owned(),
                ));
            }
            ConfirmResponse::Edit => {
                edit_install_plan_recipes(install_plan)?;
            }
            ConfirmResponse::Invalid => {}
        }
    }
}

pub(super) fn build_install_plan_report(
    app: &AppContext,
    request: &CommandRequest,
    install_plan: &[PlannedInstallAction],
) -> Result<CommandReport, CoreError> {
    let bootstrap = app.database.bootstrap()?;
    let preflight = install_preflight_report(app, request, install_plan)?;
    let activation_backend = planned_activation_backend(app.database.layout().mode);
    let actions = install_plan
        .iter()
        .map(|action| {
            planned_install_action_json(
                action,
                activation_backend,
                app.config.metadata.link_option_mode,
            )
        })
        .collect::<Vec<_>>();

    Ok(CommandReport {
        area: "plan",
        status: "planned",
        exit_status: ExitStatus::Success,
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
        output_mode: request.output_mode,
        dry_run: false,
        summary: format!(
            "install plan contains {} package action(s) across {} requested target(s).",
            actions.len(),
            request.operands.len(),
        ),
        details: Some(json!({
            "schema_version": bootstrap.schema_version,
            "created_database": bootstrap.created_database,
            "layout": app.database.layout(),
            "preflight": preflight,
            "plan": {
                "kind": "install",
                "link_option_mode": app.config.metadata.link_option_mode,
                "actions": actions,
            },
        })),
    })
}

fn edit_install_plan_recipes(install_plan: &[PlannedInstallAction]) -> Result<(), CoreError> {
    let mut opened = false;
    for action in install_plan {
        if install_execution_decision(action).needs_change {
            open_path_in_editor(recipe_edit_target(&action.resolved.recipe.path))?;
            opened = true;
        }
    }
    if !opened {
        return Err(CoreError::Operator(
            "no changing package recipes to edit in this install plan".to_owned(),
        ));
    }
    Ok(())
}

fn recipe_edit_target(recipe_path: &Path) -> &Path {
    recipe_path.parent().unwrap_or(recipe_path)
}

fn plan_confirmation_hash(install_plan: &[PlannedInstallAction]) -> String {
    let mut hasher = Sha256::new();
    for action in install_plan {
        let decision = install_execution_decision(action);
        hasher.update(action.package_name.as_bytes());
        hasher.update(
            format!(
                "{}:{}-{}",
                action.resolved.recipe.package.epoch,
                action.resolved.recipe.package.version,
                action.resolved.recipe.package.rel,
            )
            .as_bytes(),
        );
        hasher.update(action.resolved.selected_lane.as_bytes());
        hasher.update(action.resolved.selected_source_kind.as_bytes());
        hasher.update([u8::from(decision.needs_change)]);
        if let Ok(digest) = crate::app_review_memory::hash_file(&action.resolved.recipe.path) {
            hasher.update(digest.as_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

fn transaction_confirm_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("transaction-confirm.json")
}

fn write_transaction_confirmation(
    data_dir: &Path,
    request: &CommandRequest,
    plan_hash: &str,
) -> Result<(), CoreError> {
    let path = transaction_confirm_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stamp = TransactionConfirmStamp {
        plan_hash: plan_hash.to_owned(),
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&stamp)?)?;
    Ok(())
}
