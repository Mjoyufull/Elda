//! Dispatch-level confirmation for mutating commands that do not run their own gate.

use crate::CommandRequest;
use crate::app_confirm::confirm_mutation;
use crate::app_dispatch_confirm::{
    clear_dispatch_confirmation, dispatch_confirmation_matches, write_dispatch_confirmation,
};
use crate::error::CoreError;

/// Commands that render their own, better-informed gate. The dispatch gate must
/// stay out of their way: asking `Proceed with \`u\`?` before the upgrade plan
/// exists makes the operator confirm something they have not been shown yet.
const SELF_CONFIRMED_PREFIXES: &[&[&str]] = &[
    &["i"],
    &["ig"],
    &["ib"],
    &["u"],
    &["rm"],
    &["rmt", "add"],
    &["rmt", "rm"],
    &["rmt", "remove"],
    &["config", "apply"],
    &["config", "keep"],
    &["maint", "fix"],
    &["trigger", "run"],
    &["rollback"],
];

pub(crate) fn confirm_dispatch_mutation(
    data_dir: &std::path::Path,
    request: &CommandRequest,
) -> Result<(), CoreError> {
    if request.dry_run || !requires_dispatch_confirmation(&request.command_path) {
        return Ok(());
    }
    if dispatch_confirmation_matches(data_dir, request)? {
        clear_dispatch_confirmation(data_dir)?;
        return Ok(());
    }
    let summary = mutation_summary(request);
    confirm_mutation(request, &summary)?;
    write_dispatch_confirmation(data_dir, request)
}

fn requires_dispatch_confirmation(path: &[String]) -> bool {
    if path.is_empty()
        || crate::command_class::is_read_only_command(path)
        || has_dedicated_confirm(path)
    {
        return false;
    }
    matches!(
        path.first().map(String::as_str),
        Some(
            "a" | "add"
                | "rm"
                | "u"
                // `sync` refreshes the local snapshot of remote indexes. It
                // destroys nothing and is the one command an operator runs
                // reflexively; `pacman -Sy` does not prompt either.
                | "pin"
                | "unpin"
                | "hold"
                | "unhold"
                | "downgrade"
                | "recover"
                | "fix-triggers"
                | "autoremove"
                | "adopt"
                | "rc"
                | "vendor"
                | "cache"
                | "pf"
                | "rmt"
                | "mg"
                | "state"
                | "ci"
                | "forge"
                | "appimage"
                | "host"
                | "publish"
        )
    )
}

fn has_dedicated_confirm(path: &[String]) -> bool {
    SELF_CONFIRMED_PREFIXES
        .iter()
        .any(|prefix| path_starts_with(path, prefix))
}

fn path_starts_with(path: &[String], prefix: &[&str]) -> bool {
    path.len() >= prefix.len()
        && prefix
            .iter()
            .zip(path.iter())
            .all(|(expected, actual)| *expected == actual.as_str())
}

fn mutation_summary(request: &CommandRequest) -> String {
    let command = request.command_path.join(" ");
    let targets = request
        .operands
        .iter()
        .filter(|operand| !operand.starts_with('-'))
        .take(3)
        .cloned()
        .collect::<Vec<_>>();
    if targets.is_empty() {
        format!("Proceed with `{command}`?")
    } else if request.operands.len() > targets.len() {
        format!("Proceed with `{command}` for {} target(s)?", targets.len())
    } else {
        format!("Proceed with `{command}` for `{}`?", targets.join("`, `"))
    }
}
