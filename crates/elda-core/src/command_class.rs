//! Single source of truth for "does this command mutate the root?".
//!
//! Two things depend on this answer and they must never disagree:
//!
//! * **Privilege escalation.** A read-only command on a live host root must not
//!   re-exec under `sudo`/`doas`. `elda ls` asking for a password is the single
//!   most obnoxious stopping point in the CLI.
//! * **Dispatch confirmation.** Only mutations get the generic `Proceed?` gate.
//!
//! The default for an unrecognised path is **mutating**. Being wrong in that
//! direction costs an unnecessary escalation; being wrong the other way means a
//! mutation runs without the gate.

/// True when the command only reads state and never writes to the managed root.
#[must_use]
pub(crate) fn is_read_only_command(path: &[String]) -> bool {
    match path {
        [command] => is_read_only_root_command(command),
        [namespace, command, ..] => is_read_only_namespaced(namespace, command),
        [] => true,
    }
}

fn is_read_only_root_command(command: &str) -> bool {
    matches!(
        command,
        "ls" | "list"
            | "search"
            | "info"
            | "files"
            | "verify"
            | "reverify"
            | "why"
            | "rdeps"
            | "versions"
            | "diff"
            | "check"
            | "version"
    )
    // `doctor` deliberately stays out of this list: it reports *and prepares*
    // bootstrap readiness, so it is allowed to create the layout it checks.
}

fn is_read_only_namespaced(namespace: &str, command: &str) -> bool {
    match namespace {
        // `qa` runs lint/build/smoke inside the workspace, never against the root.
        "qa" => true,
        "files" => matches!(command, "owner" | "search"),
        "git" => matches!(command, "tags" | "releases" | "versions"),
        "appimage" => command == "inspect",
        "rmt" => matches!(command, "ls" | "info" | "preview" | "trust"),
        "rc" => matches!(command, "ls" | "show" | "diff" | "check" | "publish-ready"),
        "config" => matches!(command, "pending" | "diff"),
        "review" => matches!(command, "ls" | "info" | "diff"),
        "trigger" => matches!(command, "ls" | "info" | "diff"),
        "maint" => command == "check",
        "pf" => command == "show",
        "fl" => matches!(command, "ls" | "check" | "diff"),
        "cache" => command == "ls",
        "ext" => command == "ls",
        "daemon" => command == "status",
        "forge" => matches!(command, "search" | "browse"),
        "mg" => command == "report",
        "state" => matches!(command, "show" | "export"),
        "ci" => matches!(command, "status" | "logs"),
        // `finalize`, `sign`, and `promote` all write; only planning is read-only.
        "publish" => matches!(command, "plan" | "diff"),
        "host" => matches!(
            command,
            "scan-tree"
                | "test-tree"
                | "diff-tree"
                | "client-bundle"
                | "status"
                | "doctor"
                | "init-ci"
                | "print-cache-config"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_read_only_command;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    #[test]
    fn bare_query_commands_are_read_only() {
        for command in ["ls", "list", "search", "info", "why", "rdeps", "files"] {
            assert!(
                is_read_only_command(&path(&[command])),
                "`{command}` must not trigger privilege escalation"
            );
        }
    }

    #[test]
    fn mutating_root_commands_are_not_read_only() {
        for command in [
            "i",
            "ig",
            "ib",
            "rm",
            "u",
            "sync",
            "init",
            "recover",
            "rollback",
            "autoremove",
            "fix-triggers",
            "adopt",
            "downgrade",
            "pin",
            "hold",
        ] {
            assert!(
                !is_read_only_command(&path(&[command])),
                "`{command}` mutates and must keep its gate"
            );
        }
    }

    #[test]
    fn namespaced_reads_and_writes_are_separated() {
        assert!(is_read_only_command(&path(&["rmt", "ls"])));
        assert!(is_read_only_command(&path(&["appimage", "inspect"])));
        assert!(is_read_only_command(&path(&["publish", "plan"])));
        assert!(is_read_only_command(&path(&["state", "export"])));

        assert!(!is_read_only_command(&path(&["rmt", "add"])));
        assert!(!is_read_only_command(&path(&["publish", "finalize"])));
        assert!(!is_read_only_command(&path(&["publish", "sign"])));
        assert!(!is_read_only_command(&path(&["state", "import"])));
        assert!(!is_read_only_command(&path(&["review", "forget"])));
        assert!(!is_read_only_command(&path(&["maint", "fix"])));
    }

    #[test]
    fn unknown_paths_default_to_mutating() {
        assert!(!is_read_only_command(&path(&["totally-new-command"])));
        assert!(!is_read_only_command(&path(&["rmt", "brand-new-verb"])));
    }
}
