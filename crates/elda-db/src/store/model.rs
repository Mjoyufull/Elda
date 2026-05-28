use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootstrapReport {
    pub created_database: bool,
    pub schema_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledPackageRecord {
    pub pkgname: String,
    pub arch: Option<String>,
    pub version: String,
    pub package_kind: String,
    pub variant_id: Option<String>,
    pub install_reason: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub remote_name: Option<String>,
    pub state_id: Option<String>,
    pub repo_commit: Option<String>,
    pub payload_sha256: Option<String>,
    pub manifest_hash: Option<String>,
    pub pinned_version: Option<String>,
    pub held: bool,
    pub hold_source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledPackageDetails {
    pub pkgname: String,
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
    pub arch: Option<String>,
    pub package_kind: String,
    pub variant_id: Option<String>,
    pub install_reason: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub remote_name: Option<String>,
    pub state_id: Option<String>,
    pub activation_backend: Option<String>,
    pub repo_commit: Option<String>,
    pub payload_sha256: Option<String>,
    pub manifest_hash: Option<String>,
    pub pinned_version: Option<String>,
    pub held: bool,
    pub hold_source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageFileRecord {
    pub pkgname: String,
    pub arch: Option<String>,
    pub path: String,
    pub path_kind: String,
    pub sha256: Option<String>,
    pub size: u64,
    pub mode: u32,
    pub link_target: Option<String>,
    pub is_conffile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageDependencyRecord {
    pub pkgname: String,
    pub dependency_name: String,
    pub dependency_kind: String,
    pub raw_expr: String,
    pub is_weak: bool,
    pub provider_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReverseDependencyRecord {
    pub pkgname: String,
    pub dependency_kind: String,
    pub raw_expr: String,
    pub is_weak: bool,
    pub provider_group: Option<String>,
    pub install_reason: String,
    pub pinned_version: Option<String>,
    pub held: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRecord {
    pub pkgname: String,
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
    pub arch: Option<String>,
    pub package_kind: String,
    pub variant_id: Option<String>,
    pub install_reason: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub remote_name: Option<String>,
    pub channel: Option<String>,
    pub state_id: Option<String>,
    pub activation_backend: Option<String>,
    pub repo_commit: Option<String>,
    pub payload_sha256: Option<String>,
    pub manifest_hash: Option<String>,
    pub pinned_version: Option<String>,
    pub held: bool,
    pub hold_source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateSnapshot {
    pub schema_version: u32,
    pub active_state: Option<String>,
    pub world: Vec<String>,
    pub installed_packages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthReport {
    pub schema_version: u32,
    pub installed_packages: usize,
    pub world_anchors: usize,
    pub pending_journals: Vec<String>,
    pub issues: Vec<String>,
}
