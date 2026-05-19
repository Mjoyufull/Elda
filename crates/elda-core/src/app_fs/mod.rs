mod inspect;
mod recovery;
mod remove;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::app::AppContext;
use crate::app_parse::installed_version;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_build::ManifestEntryKind;
use elda_install::{
    inspect_trigger, inspect_triggers, recover_pending_transactions, remove_package,
    remove_package_purge_conffiles, repair_triggers, rollback_plan, rollback_state,
    run_named_trigger, verify_packages,
};

fn manifest_kind(kind: ManifestEntryKind) -> &'static str {
    match kind {
        ManifestEntryKind::Directory => "directory",
        ManifestEntryKind::RegularFile => "file",
        ManifestEntryKind::Symlink => "symlink",
    }
}
