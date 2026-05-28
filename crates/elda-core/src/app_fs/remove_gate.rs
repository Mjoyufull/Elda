use std::io::{self, Write};
use std::path::Path;

use serde_json::{Value, json};

use crate::app::AppContext;
use crate::app_confirm::{
    ConfirmResponse, interactive_session, read_yne_after_frame, require_interactive_confirmation,
};
use crate::app_parse::installed_version;
use crate::app_render_remove::render_remove_plan_frame;
use crate::error::CoreError;
use crate::render_style::highlight_operator_frame;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};

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
    clear_remove_confirmation(&data_dir)?;

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

fn remove_confirm_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("remove-confirm.json")
}

pub(super) fn clear_remove_confirmation(data_dir: &Path) -> Result<(), CoreError> {
    let path = remove_confirm_path(data_dir);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{clear_remove_confirmation, remove_confirm_path};

    #[test]
    fn clear_remove_confirmation_deletes_stale_stamp() {
        let tempdir = tempfile::TempDir::new().expect("tempdir");
        let stamp_path = remove_confirm_path(tempdir.path());
        std::fs::write(&stamp_path, "{}").expect("stale stamp should be written");

        clear_remove_confirmation(tempdir.path()).expect("stale stamp should clear");

        assert!(!stamp_path.exists());
    }
}
