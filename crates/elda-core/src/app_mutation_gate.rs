//! Dispatch-level confirmation for mutating commands that do not run their own gate.

use crate::CommandRequest;
use crate::app_confirm::confirm_mutation;
use crate::error::CoreError;

const SELF_CONFIRMED_PREFIXES: &[&[&str]] = &[
    &["a"],
    &["add"],
    &["i"],
    &["ig"],
    &["ib"],
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

pub(crate) fn confirm_dispatch_mutation(request: &CommandRequest) -> Result<(), CoreError> {
    if request.dry_run || !requires_dispatch_confirmation(&request.command_path) {
        return Ok(());
    }
    let summary = mutation_summary(request);
    confirm_mutation(request, &summary)
}

fn requires_dispatch_confirmation(path: &[String]) -> bool {
    if path.is_empty() || is_read_only(path) || has_dedicated_confirm(path) {
        return false;
    }
    matches!(
        path.first().map(String::as_str),
        Some(
            "a" | "add"
                | "rm"
                | "u"
                | "sync"
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

fn is_read_only(path: &[String]) -> bool {
    match path {
        [command] => matches!(
            command.as_str(),
            "ls" | "check"
                | "doctor"
                | "version"
                | "init"
                | "recover"
                | "autoremove"
                | "fix-triggers"
        ),
        [namespace, command] => match (namespace.as_str(), command.as_str()) {
            (
                "search" | "info" | "verify" | "reverify" | "diff" | "why" | "rdeps" | "versions"
                | "files",
                _,
            ) => true,
            ("review", _) => true,
            ("git", "tags" | "releases" | "versions") => true,
            ("rmt", "ls" | "info" | "preview" | "trust") => true,
            (
                "host",
                "scan-tree" | "test-tree" | "diff-tree" | "client-bundle" | "status" | "doctor"
                | "init-ci" | "print-cache-config",
            ) => true,
            ("publish", "plan" | "diff" | "finalize" | "sign") => true,
            ("rc", "ls" | "show" | "diff" | "check" | "publish-ready") => true,
            ("config", "pending" | "diff") => true,
            ("trigger", "ls" | "info" | "diff") => true,
            ("maint", "check") => true,
            ("pf", "show") => true,
            ("fl", "check" | "diff") => true,
            ("cache", "ls") => true,
            ("ext", "ls") => true,
            ("daemon", "status") => true,
            ("qa", _) => true,
            ("forge", "search" | "browse") => true,
            ("mg", "report") => true,
            _ => false,
        },
        _ => false,
    }
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
