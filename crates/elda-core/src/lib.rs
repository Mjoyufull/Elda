#![forbid(unsafe_code)]

mod app;
mod app_flags;
mod app_fs;
mod app_install;
mod app_parse;
mod app_policy;
mod app_profile;
mod app_recipe;
mod app_render;
mod app_render_support;
mod app_repo;
mod app_state;
mod app_upgrade;
mod cache_policy;
mod config;
mod error;
mod flags;
mod privilege;

pub use app::run_from_root;
pub use app_render::render_human;
pub use config::process_root_dir;
pub use elda_types::{CommandReport, CrateBoundary, ExitStatus, NamespaceSpec, OutputMode};
pub use error::CoreError;
pub use privilege::{PrivilegeProvider, PrivilegeRequest};

const ROOT_COMMANDS: &[&str] = &[
    "i",
    "ig",
    "ib",
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
        "Shared domain types, config skeleton, app context, and privilege policy.",
    ),
    CrateBoundary::new(
        "elda-db",
        "SQLite state, manifests, journals, layout bootstrap, and world tracking.",
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
        "Shared serializable command, identity, and version types.",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub command_path: Vec<String>,
    pub operands: Vec<String>,
    pub output_mode: OutputMode,
    pub dry_run: bool,
    pub system_mode: bool,
    pub offline: bool,
    pub accept_rotated_keys: Vec<String>,
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
            system_mode: false,
            offline: false,
            accept_rotated_keys: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_system_mode(mut self, system_mode: bool) -> Self {
        self.system_mode = system_mode;
        self
    }

    #[must_use]
    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    #[must_use]
    pub fn with_accepted_rotated_keys(mut self, accept_rotated_keys: Vec<String>) -> Self {
        self.accept_rotated_keys = accept_rotated_keys;
        self
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

pub fn run(request: CommandRequest) -> Result<CommandReport, CoreError> {
    run_from_root(process_root_dir(), request)
}

#[cfg(test)]
mod tests;
