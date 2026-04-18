#![forbid(unsafe_code)]

mod archive_state;
mod cached_archive;
mod conffile;
mod downgrade;
mod error;
mod fsops;
mod install_tx;
mod journal;
mod remove_tx;
mod rollback;
mod system_backend;
#[cfg(test)]
mod tests;
mod verify;

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use elda_types::CrateBoundary;

pub use downgrade::{DowngradeCandidate, downgrade_to_candidate, list_downgrade_candidates};
pub use error::InstallError;
pub use install_tx::{install_built_package, install_upgraded_package};
pub use journal::{RecoveredJournal, RecoveryReport};
pub use remove_tx::{remove_package, remove_package_for_upgrade, remove_package_purge_conffiles};
pub use rollback::{recover_pending_transactions, rollback_plan, rollback_state};
pub use system_backend::{
    PendingTriggerRecord, TriggerRepairReport, load_installed_system_metadata, pending_triggers,
    repair_triggers,
};
pub use verify::{VerifyIssue, VerifyIssueKind, VerifyReport, verify_packages};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-install",
    "Conflict checks, global mutation lock, and transaction execution.",
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallReport {
    pub package_name: String,
    pub state_id: String,
    pub installed_paths: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoveReport {
    pub package_name: String,
    pub removed_paths: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RollbackPlan {
    pub from_state: Option<String>,
    pub to_state: String,
    pub removed_packages: Vec<String>,
    pub restored_packages: Vec<String>,
    pub untouched_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RollbackReport {
    pub from_state: Option<String>,
    pub to_state: String,
    pub removed_packages: Vec<String>,
    pub restored_packages: Vec<InstallReport>,
    pub untouched_packages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstallExecution {
    pub(crate) archive_state: bool,
    pub(crate) take_lock: bool,
    pub(crate) conffile_mode: InstallConffileMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallConffileMode {
    FirstOwnership,
    Upgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoveConffileMode {
    PreserveAsSave,
    Purge,
    PreserveInPlaceForUpgrade,
}

#[must_use]
pub(crate) fn next_state_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("{prefix}-{millis}")
}
