use std::path::PathBuf;

use elda_recipe::{RecipeDocument, parse_pkg_lua};
use serde::{Deserialize, Serialize};

use crate::error::RepoError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TrustMode {
    #[default]
    Tofu,
    Pinned,
    Insecure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDocument {
    pub name: String,
    pub index_url: String,
    #[serde(default)]
    pub metadata_url: Option<String>,
    #[serde(default)]
    pub signature_url: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub trust: TrustMode,
    #[serde(default)]
    pub trusted_keys: Vec<String>,
    #[serde(default)]
    pub allow_stale: bool,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustedPublicKey {
    pub key_id: String,
    pub fingerprint: String,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePayloadTrust {
    pub remote_name: String,
    pub trust: TrustMode,
    pub verified: bool,
    pub trusted_public_keys: Vec<TrustedPublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheDocument {
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncedIndexSnapshot {
    pub schema_version: u32,
    pub generated_at: u64,
    pub offline: bool,
    pub remotes: Vec<SyncedRemoteRecord>,
    pub packages: Vec<SyncedPackageRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncedRemoteRecord {
    pub name: String,
    pub index_url: String,
    pub priority: u32,
    pub package_count: usize,
    pub trust: TrustMode,
    pub verified: bool,
    pub stale: bool,
    pub source: String,
    pub selected_key: Option<String>,
    pub last_sync_unix: Option<u64>,
    pub last_verified_unix: Option<u64>,
    pub issue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncedPackageRecord {
    pub remote_name: String,
    pub remote_priority: u32,
    pub pkgname: String,
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
    pub arch: Vec<String>,
    pub package_kind: String,
    pub variant_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub channel: Option<String>,
    pub asset_url: Option<String>,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub payload_sig: Option<String>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub fallback_git_url: Option<String>,
    pub repo_commit: Option<String>,
    pub release_tag: Option<String>,
    pub pkg_lua: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncReport {
    pub snapshot_path: PathBuf,
    pub offline: bool,
    pub remote_count: usize,
    pub package_count: usize,
    pub verified_remote_count: usize,
    pub stale_remote_count: usize,
    pub failed_remote_count: usize,
    pub remotes: Vec<SyncedRemoteRecord>,
}

impl SyncedPackageRecord {
    #[must_use]
    pub fn version_string(&self) -> String {
        format!("{}:{}-{}", self.epoch, self.pkgver, self.pkgrel)
    }

    pub fn parse_recipe(&self) -> Result<RecipeDocument, RepoError> {
        let synthetic_path = PathBuf::from(format!(
            "remote://{}/{}/pkg.lua",
            self.remote_name, self.pkgname
        ));

        parse_pkg_lua(&synthetic_path, &self.pkg_lua)
            .map_err(|error| RepoError::Parse(error.to_string()))
    }
}

const fn default_enabled() -> bool {
    true
}

const fn default_priority() -> u32 {
    100
}
