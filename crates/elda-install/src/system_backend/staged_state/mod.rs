mod activate;
mod compose;
mod util;

use std::collections::BTreeSet;
use std::path::PathBuf;

pub(crate) use activate::{activate_staged_state, capture_active_system_state};
pub(crate) use compose::{prepare_staged_install, prepare_staged_remove};

pub(crate) struct StagedSystemState {
    pub(crate) stage_root: PathBuf,
    pub(crate) tracked_paths: BTreeSet<String>,
}
