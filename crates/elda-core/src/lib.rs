#![forbid(unsafe_code)]

mod app;
mod app_appimage;
mod app_ci;
mod app_config_queue;
mod app_confirm;
mod app_doctor;
mod app_ext;
mod app_failure;
mod app_flags;
mod app_fs;
mod app_git;
mod app_host;
mod app_init;
mod app_install;
mod app_maint;
mod app_metadata_add;
mod app_migration;
mod app_model;
mod app_mutation_gate;
mod app_parse;
mod app_policy;
mod app_profile;
mod app_publish;
mod app_recipe;
mod app_recipe_metadata;
mod app_recipe_show;
mod app_render;
mod app_render_appimage;
mod app_render_ci;
mod app_render_extended;
mod app_render_git;
mod app_render_host;
mod app_render_install;
mod app_render_migration;
mod app_render_misc;
mod app_render_state;
mod app_render_support;
mod app_render_tree;
mod app_repo;
mod app_review;
mod app_review_memory;
mod app_state;
mod app_upgrade;
mod app_vendor;
mod app_version;
mod cache_policy;
mod config;
mod editor;
mod error;
mod flags;
mod host_config;
mod privilege;
mod progress;
mod progress_live;
mod progress_live_json;
mod recipe_catalog;
mod run_log;
mod version;

pub use app::run_from_root;
pub use app_failure::{process_exit_code, report_frontend_failure, report_runtime_failure};
pub use app_render::render_human;
pub use config::process_root_dir;
pub use elda_types::{CommandReport, CrateBoundary, ExitStatus, NamespaceSpec, OutputMode};
pub use error::CoreError;
pub use privilege::{PrivilegeProvider, PrivilegeRequest};
pub use version::{cli_long_version, cli_version_line, version_details};

const ROOT_COMMANDS: &[&str] = &[
    "a",
    "add",
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
    "versions",
    "pin",
    "unpin",
    "hold",
    "unhold",
    "adopt",
    "downgrade",
    "diff",
    "check",
    "doctor",
    "version",
    "init",
    "recover",
    "rollback",
    "fix-triggers",
    "autoremove",
];

const CLI_NAMESPACES: &[NamespaceSpec] = &[
    NamespaceSpec::new("(root)", ROOT_COMMANDS),
    NamespaceSpec::new(
        "rmt",
        &[
            "add",
            "add-from-bundle",
            "ls",
            "info",
            "preview",
            "trust",
            "enable",
            "disable",
            "set-priority",
            "rm",
        ],
    ),
    NamespaceSpec::new(
        "host",
        &[
            "scan-tree",
            "test-tree",
            "diff-tree",
            "push-recipes",
            "client-bundle",
            "init-ci",
            "doctor",
            "status",
            "link",
            "print-cache-config",
        ],
    ),
    NamespaceSpec::new(
        "publish",
        &["plan", "run", "finalize", "diff", "promote", "sign"],
    ),
    NamespaceSpec::new(
        "rc",
        &[
            "add",
            "show",
            "diff",
            "publish-ready",
            "edit",
            "check",
            "ls",
            "rm",
        ],
    ),
    NamespaceSpec::new(
        "ci",
        &["sub", "run", "status", "pr", "retry", "logs", "batch"],
    ),
    NamespaceSpec::new("vendor", &["add", "import", "export"]),
    NamespaceSpec::new("forge", &["search", "browse"]),
    NamespaceSpec::new("git", &["tags", "releases"]),
    NamespaceSpec::new("appimage", &["inspect"]),
    NamespaceSpec::new(
        "pf",
        &[
            "apply",
            "add",
            "rm",
            "show",
            "set-init",
            "clear-init",
            "set-arch",
            "add-foreign-arch",
            "remove-foreign-arch",
        ],
    ),
    NamespaceSpec::new("fl", &["check", "diff"]),
    NamespaceSpec::new("mg", &["from", "lock", "unlock"]),
    NamespaceSpec::new("review", &["ls", "info", "forget", "diff"]),
    NamespaceSpec::new("maint", &["check", "fix"]),
    NamespaceSpec::new("trigger", &["ls", "info", "run", "diff"]),
    NamespaceSpec::new("config", &["pending", "diff", "apply", "keep"]),
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
    elda_git::BOUNDARY,
    elda_appimage::BOUNDARY,
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
    pub log_level: Option<u8>,
    pub accept_rotated_keys: Vec<String>,
    pub no_stream: bool,
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
            log_level: None,
            accept_rotated_keys: Vec::new(),
            no_stream: false,
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
    pub fn with_log_level(mut self, log_level: Option<u8>) -> Self {
        self.log_level = log_level;
        self
    }

    #[must_use]
    pub fn with_accepted_rotated_keys(mut self, accept_rotated_keys: Vec<String>) -> Self {
        self.accept_rotated_keys = accept_rotated_keys;
        self
    }

    #[must_use]
    pub fn with_no_stream(mut self, no_stream: bool) -> Self {
        self.no_stream = no_stream;
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
