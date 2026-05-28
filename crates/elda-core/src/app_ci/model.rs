use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CiBatchRecord {
    pub(crate) name: String,
    pub(crate) packages: Vec<String>,
    pub(crate) last_submission_id: Option<String>,
    pub(crate) state: String,
    pub(crate) updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CiSubmissionRecord {
    pub(crate) id: String,
    pub(crate) requested_targets: Vec<String>,
    pub(crate) packages: Vec<String>,
    pub(crate) branch_name: String,
    #[serde(default)]
    pub(crate) target_branch: String,
    pub(crate) mode: String,
    pub(crate) state: String,
    pub(crate) immediate: bool,
    pub(crate) batch_name: Option<String>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
    #[serde(default)]
    pub(crate) attempts: u32,
    #[serde(default)]
    pub(crate) planned_layers: u32,
    #[serde(default)]
    pub(crate) completed_layers: u32,
    #[serde(default)]
    pub(crate) queued_at: Option<u64>,
    #[serde(default)]
    pub(crate) started_at: Option<u64>,
    #[serde(default)]
    pub(crate) completed_at: Option<u64>,
    #[serde(default)]
    pub(crate) last_error: Option<String>,
    pub(crate) issues: Vec<String>,
    pub(crate) published_packages: Vec<PublishedPackageRecord>,
    pub(crate) lock_path: Option<PathBuf>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) signature_path: Option<PathBuf>,
    pub(crate) log_path: PathBuf,
    pub(crate) packages_repo_path: PathBuf,
    pub(crate) trusted_key_fingerprint: Option<String>,
    pub(crate) repo_commit: Option<String>,
    pub(crate) remote_name: Option<String>,
    pub(crate) remote_url: Option<String>,
    pub(crate) pushed_ref: Option<String>,
    pub(crate) pushed_commit: Option<String>,
    pub(crate) pushed_at: Option<u64>,
    pub(crate) review_url: Option<String>,
    pub(crate) review_kind: Option<String>,
    pub(crate) review_id: Option<String>,
    pub(crate) review_created_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PublishedPackageRecord {
    pub(crate) pkgname: String,
    pub(crate) epoch: u64,
    pub(crate) pkgver: String,
    pub(crate) pkgrel: u64,
    pub(crate) arch: String,
    pub(crate) variant_id: Option<String>,
    pub(crate) payload_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) payload_sha256: String,
    pub(crate) manifest_hash: String,
    pub(crate) payload_signature: String,
    #[serde(default)]
    pub(crate) signature_path: PathBuf,
    #[serde(default)]
    pub(crate) sbom_path: PathBuf,
    #[serde(default)]
    pub(crate) attestation_path: PathBuf,
    pub(crate) repo_commit: Option<String>,
    pub(crate) published_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CiLockDocument {
    pub(crate) format_version: u32,
    pub(crate) generated_at: u64,
    pub(crate) packages: Vec<CiLockPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CiLockPackage {
    pub(crate) pkgname: String,
    pub(crate) epoch: u64,
    pub(crate) pkgver: String,
    pub(crate) pkgrel: u64,
    pub(crate) arch: String,
    pub(crate) source_ref: Option<String>,
    pub(crate) runtime_depends: Vec<String>,
    pub(crate) makedepends: Vec<String>,
    pub(crate) checkdepends: Vec<String>,
    pub(crate) provides: Vec<String>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) build_profile: String,
    pub(crate) ci_policy: String,
    pub(crate) layer: u32,
    pub(crate) artifact_name: Option<String>,
    pub(crate) artifact_sha256: Option<String>,
    pub(crate) repo_commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ForgeBrowseRecord {
    pub(crate) package: String,
    pub(crate) local_recipe_path: Option<PathBuf>,
    pub(crate) packages_repo_path: Option<PathBuf>,
    pub(crate) index_path: Option<PathBuf>,
    pub(crate) pkg_lua: Option<String>,
    pub(crate) published: Option<PublishedPackageRecord>,
    pub(crate) channel: Option<String>,
}
