mod document;
mod fetch;
mod select;
mod state;
mod sync;
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

pub use select::{load_remote_payload_trust, resolve_package, search_packages};
pub use sync::{SyncOptions, load_snapshot, sync_remotes};
