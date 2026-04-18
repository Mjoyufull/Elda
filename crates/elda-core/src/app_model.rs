use serde::{Deserialize, Serialize};

use crate::config::default_native_arch;
use elda_repo::RemoteDocument;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DesiredStateDocument {
    pub(crate) format_version: u32,
    pub(crate) exported_at: String,
    pub(crate) installation_mode: String,
    pub(crate) prefix: String,
    pub(crate) profile: DesiredStateProfile,
    pub(crate) remotes: Vec<RemoteDocument>,
    pub(crate) world: Vec<String>,
    pub(crate) installed: Vec<DesiredStatePackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DesiredStateProfile {
    #[serde(default)]
    pub(crate) active_profiles: Vec<String>,
    #[serde(default)]
    pub(crate) base: String,
    #[serde(default = "default_native_arch")]
    pub(crate) native_arch: String,
    #[serde(default)]
    pub(crate) foreign_arches: Vec<String>,
    #[serde(default)]
    pub(crate) init: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedProfileState {
    pub(crate) active_profiles: Vec<String>,
    pub(crate) native_arch: String,
    pub(crate) foreign_arches: Vec<String>,
    pub(crate) init: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DesiredStatePackage {
    pub(crate) pkgname: String,
    pub(crate) version: String,
    pub(crate) install_reason: String,
    pub(crate) package_kind: String,
    #[serde(default)]
    pub(crate) variant_id: Option<String>,
    pub(crate) source_kind: String,
    pub(crate) remote_name: Option<String>,
    pub(crate) pinned_version: Option<String>,
    pub(crate) held: bool,
    pub(crate) hold_source: Option<String>,
}
