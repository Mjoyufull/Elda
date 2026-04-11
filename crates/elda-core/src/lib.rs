#![forbid(unsafe_code)]

pub use elda_types::{CommandReport, CrateBoundary, ExitStatus, NamespaceSpec, OutputMode};

const ROOT_COMMANDS: &[&str] = &[
    "i",
    "rm",
    "u",
    "sync",
    "ls",
    "search",
    "info",
    "files",
    "verify",
    "reverify",
    "why",
    "rdeps",
    "pin",
    "unpin",
    "hold",
    "unhold",
    "adopt",
    "downgrade",
    "diff",
    "check",
    "recover",
    "rollback",
    "fix-triggers",
    "autoremove",
];

const CLI_NAMESPACES: &[NamespaceSpec] = &[
    NamespaceSpec::new("(root)", ROOT_COMMANDS),
    NamespaceSpec::new("rmt", &["add"]),
    NamespaceSpec::new("rc", &["add", "edit", "check"]),
    NamespaceSpec::new(
        "ci",
        &["sub", "run", "status", "pr", "retry", "logs", "batch"],
    ),
    NamespaceSpec::new("vendor", &["add", "import", "export"]),
    NamespaceSpec::new("forge", &["search", "browse"]),
    NamespaceSpec::new("pf", &["apply", "show", "set-init"]),
    NamespaceSpec::new("fl", &["check", "diff"]),
    NamespaceSpec::new("mg", &["from", "lock", "unlock"]),
    NamespaceSpec::new("state", &["show", "export", "import"]),
    NamespaceSpec::new("cache", &["add", "ls"]),
    NamespaceSpec::new("daemon", &["run", "status", "refresh"]),
    NamespaceSpec::new("ext", &["ls"]),
    NamespaceSpec::new("qa", &["lint", "build", "smoke", "stack", "repro", "diff"]),
];

const WORKSPACE_BOUNDARIES: &[CrateBoundary] = &[
    CrateBoundary::new(
        "elda-cli",
        "CLI surface only, output formatting, and command wiring.",
    ),
    CrateBoundary::new(
        "elda-core",
        "Shared domain types, config skeleton, and app context.",
    ),
    CrateBoundary::new(
        "elda-db",
        "SQLite state, manifests, journals, and world tracking.",
    ),
    CrateBoundary::new(
        "elda-repo",
        "Remote definitions, index sync, and verification.",
    ),
    CrateBoundary::new(
        "elda-fetch",
        "HTTP fetch, cache access, and checksum plumbing.",
    ),
    CrateBoundary::new("elda-git", "Git source fetch and inspection."),
    CrateBoundary::new(
        "elda-recipe",
        "Recipe loading, validation, and legacy import.",
    ),
    CrateBoundary::new(
        "elda-build",
        "Build orchestration, staging, and payload assembly.",
    ),
    CrateBoundary::new("elda-install", "Conflict checks, locks, and transactions."),
    CrateBoundary::new(
        "elda-unix",
        "Unix host traits for activation and build execution.",
    ),
    CrateBoundary::new(
        "elda-linux",
        "Linux-only activation, multilib, and namespace backend.",
    ),
    CrateBoundary::new("elda-ext", "Extension protocol and adapter discovery."),
    CrateBoundary::new(
        "elda-types",
        "Shared serializable command and boundary types.",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub command_path: Vec<String>,
    pub operands: Vec<String>,
    pub output_mode: OutputMode,
    pub dry_run: bool,
}

impl CommandRequest {
    #[must_use]
    pub fn new(
        command_path: Vec<String>,
        operands: Vec<String>,
        output_mode: OutputMode,
        dry_run: bool,
    ) -> Self {
        Self {
            command_path,
            operands,
            output_mode,
            dry_run,
        }
    }
}

#[must_use]
pub fn cli_surface() -> &'static [NamespaceSpec] {
    CLI_NAMESPACES
}

#[must_use]
pub fn workspace_boundaries() -> &'static [CrateBoundary] {
    WORKSPACE_BOUNDARIES
}

#[must_use]
pub fn run(request: CommandRequest) -> CommandReport {
    let command_label = if request.command_path.is_empty() {
        "help".to_owned()
    } else {
        request.command_path.join(" ")
    };
    let status = if request.dry_run { "planned" } else { "stub" };
    let summary = if request.operands.is_empty() {
        format!(
            "phase 0 skeleton: `{command_label}` is wired into the CLI surface; backend behavior is not implemented yet."
        )
    } else {
        format!(
            "phase 0 skeleton: `{command_label}` accepted operands [{}]; backend behavior is not implemented yet.",
            request.operands.join(", ")
        )
    };

    CommandReport {
        phase: "phase-0",
        status,
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandRequest, cli_surface, run};
    use crate::OutputMode;

    #[test]
    fn cli_surface_contains_all_spec_namespaces() {
        let namespace_names = cli_surface()
            .iter()
            .map(|namespace| namespace.name)
            .collect::<Vec<_>>();

        assert!(namespace_names.contains(&"(root)"));
        assert!(namespace_names.contains(&"rmt"));
        assert!(namespace_names.contains(&"rc"));
        assert!(namespace_names.contains(&"ci"));
        assert!(namespace_names.contains(&"vendor"));
        assert!(namespace_names.contains(&"forge"));
        assert!(namespace_names.contains(&"pf"));
        assert!(namespace_names.contains(&"fl"));
        assert!(namespace_names.contains(&"mg"));
        assert!(namespace_names.contains(&"state"));
        assert!(namespace_names.contains(&"cache"));
        assert!(namespace_names.contains(&"daemon"));
        assert!(namespace_names.contains(&"ext"));
        assert!(namespace_names.contains(&"qa"));
    }

    #[test]
    fn dry_run_requests_report_planned_status() {
        let report = run(CommandRequest::new(
            vec!["i".to_owned()],
            vec!["ripgrep".to_owned()],
            OutputMode::Human,
            true,
        ));

        assert_eq!(report.status, "planned");
        assert!(report.summary.contains("ripgrep"));
    }
}
