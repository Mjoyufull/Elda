use std::io::{self, Write};

use super::preflight::missing_release_trust_keys;
use super::report::install_execution_decision;
use crate::app::{AppContext, PlannedInstallAction};
use crate::app_confirm::interactive_session;
use crate::config::Config;
use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};

const SOURCE_BUILD_RESERVE_BYTES: u64 = 256 * 1024 * 1024;

/// Apply the decisions the operator already made at the transaction gate.
///
/// One execution is one decision. The plan frame lists the temporary
/// build-dependency policy and every release key the plan will import, so
/// accepting that single gate accepts both. Re-asking here would be the
/// gate-stacking the CLI is meant to be rid of.
pub(crate) fn confirm_install_execution(
    app: &AppContext,
    request: &CommandRequest,
    plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    if !interactive_session(request) {
        enforce_noninteractive_install_policy(app, request, plan)?;
        return Ok(());
    }

    import_release_trust_keys(app, plan)
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

/// Persist the release keys the plan frame listed under `trust keys::`.
fn import_release_trust_keys(
    app: &AppContext,
    plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    let missing = missing_release_trust_keys(app, plan);
    if missing.is_empty() {
        return Ok(());
    }

    let layout = app.database.layout();
    Config::append_release_keys(&layout.root_dir, &missing)?;

    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    writeln!(
        stderr,
        "imported {} release trust key(s) into [trust].release_keys",
        missing.len()
    )?;
    Ok(())
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
