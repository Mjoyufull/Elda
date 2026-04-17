use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VendorRecipeReport {
    pub package_name: String,
    pub recipe_dir: PathBuf,
    pub source_kind: String,
    pub source_url: String,
    pub sha256: String,
    pub asset: Option<String>,
    pub binary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VendorImportReport {
    pub source_path: PathBuf,
    pub format: String,
    pub packages: Vec<VendorRecipeReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VendorExportReport {
    pub output_path: PathBuf,
    pub format: String,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendorLockFile {
    pub version: u32,
    pub entries: Vec<VendorLockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendorLockEntry {
    pub package_name: String,
    pub source_kind: String,
    pub url: Option<String>,
    pub repo: Option<String>,
    pub tag: Option<String>,
    pub asset: Option<String>,
    pub sha256: String,
    pub binary: Option<String>,
    pub rename: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedVendorManifestLine {
    pub(super) package_name: String,
    pub(super) source: String,
    pub(super) binary: Option<String>,
    pub(super) asset: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GitHubReleaseSpec<'a> {
    pub(super) repo: &'a str,
    pub(super) tag_or_latest: &'a str,
}

#[derive(Debug, Clone)]
pub(super) enum ResolvedVendorSource {
    UrlArchive {
        url: String,
        sha256: String,
        binary: Option<String>,
        rename: Option<String>,
    },
    GitHubRelease {
        repo: String,
        tag: String,
        asset: String,
        sha256: String,
        binary: Option<String>,
        rename: Option<String>,
    },
}

impl ResolvedVendorSource {
    pub(super) fn source_kind(&self) -> &'static str {
        match self {
            Self::UrlArchive { .. } => "url_archive",
            Self::GitHubRelease { .. } => "github_release",
        }
    }

    pub(super) fn source_url(&self) -> String {
        match self {
            Self::UrlArchive { url, .. } => url.clone(),
            Self::GitHubRelease {
                repo, tag, asset, ..
            } => {
                format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
            }
        }
    }

    pub(super) fn sha256(&self) -> &str {
        match self {
            Self::UrlArchive { sha256, .. } | Self::GitHubRelease { sha256, .. } => sha256,
        }
    }

    pub(super) fn asset_name(&self) -> Option<&str> {
        match self {
            Self::UrlArchive { .. } => None,
            Self::GitHubRelease { asset, .. } => Some(asset.as_str()),
        }
    }

    pub(super) fn binary_name(&self) -> Option<&str> {
        match self {
            Self::UrlArchive { binary, .. } | Self::GitHubRelease { binary, .. } => {
                binary.as_deref()
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GitHubReleaseResponse {
    pub(super) tag_name: String,
    pub(super) assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GitHubReleaseAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
}
