mod metadata;
mod staged_state;
mod triggers;

use elda_db::{InstallationMode, StateLayout};
use elda_linux::activation_backend_for_system_mode;

pub use metadata::load_installed_system_metadata;
pub(crate) use metadata::{
    active_system_paths, reconcile_system_state_after_install, reconcile_system_state_after_remove,
};
pub(crate) use staged_state::{
    activate_staged_state, capture_active_system_state, prepare_staged_install,
    prepare_staged_remove,
};
pub use triggers::{PendingTriggerRecord, TriggerRepairReport, pending_triggers, repair_triggers};
pub(crate) use triggers::{run_install_triggers, run_remove_triggers};

#[must_use]
pub(crate) fn activation_backend_name(layout: &StateLayout) -> &'static str {
    activation_backend_for_system_mode(layout.mode == InstallationMode::System).name()
}

#[must_use]
pub(crate) fn next_state_prefix(layout: &StateLayout) -> &'static str {
    activation_backend_for_system_mode(layout.mode == InstallationMode::System).state_prefix()
}
