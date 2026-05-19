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
mod snapshot;
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
pub use snapshot::SnapshotRecord;
pub use system_backend::{
    ActivationBackendStatus, BootStatusReport, PendingTriggerRecord, ProfileInitReconciliation,
    SystemBackendStatus, TriggerDetailReport, TriggerInspectionReport, TriggerRepairReport,
    inspect_trigger, inspect_triggers, load_applied_profile_init, load_installed_system_metadata,
    load_system_backend_status, pending_triggers, plan_profile_init_reconciliation,
    reconcile_profile_init, reconcile_provider_assets, repair_triggers, run_named_trigger,
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
    pub activation_backend: String,
    pub installed_paths: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<SnapshotRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoveReport {
    pub package_name: String,
    pub removed_paths: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<SnapshotRecord>,
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MutationPolicy {
    pub snapshot_tool: Option<String>,
}

#[must_use]
pub(crate) fn next_state_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("{prefix}-{millis}")
}
