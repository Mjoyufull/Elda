use std::io::{self, Write};

use super::preflight::{missing_release_trust_keys, temporary_build_dependency_names};
use super::report::install_execution_decision;
use crate::app::{AppContext, PlannedInstallAction};
use crate::app_confirm::{ConfirmResponse, interactive_session, prompt_yn, prompt_yne};
use crate::config::Config;
use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};

const SOURCE_BUILD_RESERVE_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) fn confirm_install_execution(
    app: &AppContext,
    request: &CommandRequest,
    plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    if !interactive_session(request) {
        enforce_noninteractive_install_policy(app, request, plan)?;
        return Ok(());
    }

    confirm_temporary_build_dependencies(plan)?;
    confirm_release_trust_keys(app, request, plan)?;
    Ok(())
}

fn enforce_noninteractive_install_policy(
    app: &AppContext,
    request: &CommandRequest,
    plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    if request.output_mode == OutputMode::Human {
        return Ok(());
    }
    let missing = missing_release_trust_keys(app, plan);
    if missing.is_empty() {
        return Ok(());
    }
    Err(CoreError::Operator(format!(
        "install requires release trust keys {:?} in [trust].release_keys; import them interactively or add them to config before a non-interactive install",
        missing
    )))
}

fn confirm_temporary_build_dependencies(plan: &[PlannedInstallAction]) -> Result<(), CoreError> {
    let deps = temporary_build_dependency_names(plan);
    if deps.is_empty() {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(
        stdout,
        "Temporary build dependencies planned for cleanup after success:"
    )?;
    for name in deps.iter().take(16) {
        writeln!(stdout, "  {name}")?;
    }
    if deps.len() > 16 {
        writeln!(stdout, "  … {} more", deps.len() - 16)?;
    }
    if !prompt_yn(
        "Remove temporary build dependencies after a successful source build?",
        true,
    )? {
        return Err(CoreError::Operator(
            "install cancelled: operator declined temporary build-dependency cleanup policy"
                .to_owned(),
        ));
    }
    Ok(())
}

fn confirm_release_trust_keys(
    app: &AppContext,
    _request: &CommandRequest,
    plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    let missing = missing_release_trust_keys(app, plan);
    if missing.is_empty() {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(
        stdout,
        "Release verification requires trust keys not yet in Elda config:"
    )?;
    for key in &missing {
        writeln!(stdout, "  {key}")?;
    }

    loop {
        match prompt_yne("Import listed release keys into Elda config trust.release_keys?")? {
            ConfirmResponse::Accept => {
                let layout = app.database.layout();
                Config::append_release_keys(&layout.root_dir, &missing)?;
                return Ok(());
            }
            ConfirmResponse::Decline => {
                return Err(CoreError::Operator(
                    "install cancelled: required release trust keys were not accepted".to_owned(),
                ));
            }
            ConfirmResponse::Edit => {
                writeln!(
                    stdout,
                    "Add keys under [trust].release_keys in config.toml, then retry."
                )?;
            }
            ConfirmResponse::Invalid => {}
        }
    }
}

pub(crate) fn estimate_post_build_bytes(
    app: &AppContext,
    plan: &[PlannedInstallAction],
) -> (u64, &'static str) {
    let (cached, method) = super::preflight::estimate_planned_payload_bytes(app, plan);
    let (source_trees, source_method) =
        super::preflight::estimate_cached_source_tree_bytes(app, plan);
    let combined = cached.saturating_add(source_trees);
    if combined > 0 {
        let method = match (cached > 0, source_trees > 0) {
            (true, true) => "cached-payload-and-source-tree-sizes",
            (true, false) => method,
            (false, true) => source_method,
            (false, false) => "no-estimate",
        };
        return (combined, method);
    }
    let source_builds = plan
        .iter()
        .filter(|action| install_execution_decision(action).needs_change)
        .filter(|action| action.resolved.selected_lane == "source")
        .count();
    if source_builds == 0 {
        return (0, "no-source-builds");
    }
    (
        SOURCE_BUILD_RESERVE_BYTES.saturating_mul(source_builds as u64),
        "heuristic-source-build-reserve",
    )
}
