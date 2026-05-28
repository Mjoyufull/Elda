mod document;
mod fetch;
mod interemote;
mod select;
mod state;
mod sync;
mod sync_cache;
mod sync_delta;
mod sync_failure;
mod trust;

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use regex::Regex;
use serde::Deserialize;

use elda_recipe::parse_pkg_lua;
use elda_types::PackageVersion;

use crate::error::RepoError;
use crate::model::{
    RemoteDocument, SyncReport, SyncedIndexSnapshot, SyncedPackageRecord, SyncedRemoteRecord,
};
use crate::store::list_remotes;

pub use interemote::{preview_interemote, preview_interemote_with_protocols};
pub use select::{
    inspect_remote_trust, load_remote_payload_trust, resolve_package, search_packages,
};
pub use sync::{RemoteSyncEvent, SyncOptions, load_snapshot, sync_remotes};
