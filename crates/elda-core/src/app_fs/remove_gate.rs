use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::app::AppContext;
use crate::app_confirm::{
    ConfirmResponse, interactive_session, read_yne_after_frame, require_interactive_confirmation,
};
use crate::app_parse::installed_version;
use crate::app_render_remove::render_remove_plan_frame;
use crate::error::CoreError;
use crate::render_style::highlight_operator_frame;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RemoveConfirmStamp {
    plan_hash: String,
    command_path: Vec<String>,
    operands: Vec<String>,
}

pub(super) fn confirm_remove_transaction(
    app: &AppContext,
    request: &CommandRequest,
    packages: &[String],
    cascade: bool,
    purge_conffiles: bool,
) -> Result<(), CoreError> {
    if request.dry_run || request.output_mode != OutputMode::Human {
        return Ok(());
    }
    if !interactive_session(request) {
        if cfg!(test) {
            return Ok(());
        }
        return require_interactive_confirmation(request, "remove");
    }

    let data_dir = app.database.layout().data_dir.clone();
    let plan_hash = remove_plan_hash(packages, cascade, purge_conffiles);
    if remove_confirmation_matches(&data_dir, request, &plan_hash)? {
        return Ok(());
    }

    let plan_report = build_remove_plan_report(app, request, packages, cascade, purge_conffiles)?;
    if let Some(rendered) = render_remove_plan_frame(&plan_report) {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(stdout, "{}", highlight_operator_frame(&rendered))?;
        stdout.flush()?;
    }

    loop {
        match read_yne_after_frame()? {
            ConfirmResponse::Accept => {
                write_remove_confirmation(&data_dir, request, &plan_hash)?;
                return Ok(());
            }
            ConfirmResponse::Decline => {
                return Err(CoreError::Operator(
                    "remove cancelled at transaction plan gate".to_owned(),
                ));
            }
            ConfirmResponse::Edit => {
                return Err(CoreError::Operator(
                    "remove plan has no editable recipe gate; adjust operands or flags and retry"
                        .to_owned(),
                ));
            }
            ConfirmResponse::Invalid => {}
        }
    }
}

pub(super) fn build_remove_plan_report(
    app: &AppContext,
    request: &CommandRequest,
    packages: &[String],
    cascade: bool,
    purge_conffiles: bool,
) -> Result<CommandReport, CoreError> {
    let bootstrap = app.database.bootstrap()?;
    let actions = packages
        .iter()
        .map(|package| remove_action_json(app, package))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CommandReport {
        area: "plan",
        status: "planned",
        exit_status: ExitStatus::Success,
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
        output_mode: request.output_mode,
        dry_run: false,
        summary: format!("remove plan contains {} package action(s).", actions.len()),
        details: Some(json!({
            "schema_version": bootstrap.schema_version,
            "layout": app.database.layout(),
            "plan": {
                "kind": "remove",
                "cascade": cascade,
                "purge_conffiles": purge_conffiles,
                "actions": actions,
            },
        })),
    })
}

fn remove_action_json(app: &AppContext, package: &str) -> Result<Value, CoreError> {
    let installed = app.ensure_installed(package)?;
    let paths = app.database.package_files(package)?.len();
    Ok(json!({
        "target": package,
        "package": package,
        "action": "remove",
        "version": installed_version(&installed),
        "installed_version": installed_version(&installed),
        "selected_lane": installed.activation_backend.clone().unwrap_or_else(|| "unknown".to_owned()),
        "selected_source_kind": installed.source_kind,
        "persisted_source_kind": installed.source_kind,
        "source_ref": installed.source_ref,
        "remote_name": installed.remote_name,
        "installed_paths": paths,
        "install_reason": installed.install_reason,
        "held": installed.held,
        "pinned_version": installed.pinned_version,
    }))
}

fn remove_plan_hash(packages: &[String], cascade: bool, purge_conffiles: bool) -> String {
    let mut hasher = Sha256::new();
    for package in packages {
        hasher.update(package.as_bytes());
    }
    hasher.update([u8::from(cascade)]);
    hasher.update([u8::from(purge_conffiles)]);
    format!("{:x}", hasher.finalize())
}

fn remove_confirm_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("remove-confirm.json")
}

fn remove_confirmation_matches(
    data_dir: &Path,
    request: &CommandRequest,
    plan_hash: &str,
) -> Result<bool, CoreError> {
    let path = remove_confirm_path(data_dir);
    if !path.is_file() {
        return Ok(false);
    }
    let stamp: RemoveConfirmStamp = serde_json::from_slice(&std::fs::read(&path)?)?;
    Ok(stamp.plan_hash == plan_hash
        && stamp.command_path == request.command_path
        && stamp.operands == request.operands)
}

fn write_remove_confirmation(
    data_dir: &Path,
    request: &CommandRequest,
    plan_hash: &str,
) -> Result<(), CoreError> {
    let path = remove_confirm_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stamp = RemoveConfirmStamp {
        plan_hash: plan_hash.to_owned(),
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&stamp)?)?;
    Ok(())
}

pub(super) fn clear_remove_confirmation(data_dir: &Path) -> Result<(), CoreError> {
    let path = remove_confirm_path(data_dir);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
