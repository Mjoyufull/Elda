mod plan;
mod restore;

use elda_db::Database;

use crate::journal::ensure_no_pending_journals;
use crate::system_backend::capture_active_system_state;
use crate::{InstallError, RollbackReport};

pub use plan::rollback_plan;
use plan::{build_rollback_plan, resolve_target_archive};
pub use restore::recover_pending_transactions;
use restore::{remove_rollback_packages, restore_rollback_packages};

pub fn rollback_state(
    database: &Database,
    target_state: Option<&str>,
) -> Result<RollbackReport, InstallError> {
    database.bootstrap()?;
    let _lock = database.acquire_mutation_lock()?;
    ensure_no_pending_journals(database.layout())?;

    let current_state = database.state_snapshot()?.active_state;
    let target = resolve_target_archive(database, current_state.as_deref(), target_state)?;
    let plan = build_rollback_plan(database, current_state.clone(), &target)?;

    remove_rollback_packages(database, &plan.removed_packages)?;
    let restored_packages = restore_rollback_packages(database, &target.packages)?;
    capture_active_system_state(database, &target.state_id)?;
    database.set_current_state(&target.state_id)?;

    Ok(RollbackReport {
        from_state: current_state,
        to_state: target.state_id,
        removed_packages: plan.removed_packages,
        restored_packages,
        untouched_packages: plan.untouched_packages,
    })
}
