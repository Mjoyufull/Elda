use serde::Serialize;
use thiserror::Error;

mod classify;
mod github;
mod target;
#[cfg(test)]
mod tests;

pub(crate) use classify::classify_release_asset;
use github::{ReleaseResponse, fetch_provider_releases};
pub use target::{ReleaseTarget, parse_github_repo_target, parse_release_target};

use crate::tags::{VersionConfidence, normalize_tag_version};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitReleaseReport {
    pub target: String,
    pub repo: String,
    pub source: GitReleaseSource,
    pub releases: Vec<GitReleaseEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitReleaseSource {
    GithubApi,
    GitlabApi,
    GiteaApi,
    ForgejoApi,
    SourcehutApi,
    DirectManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitReleaseEntry {
    pub tag: String,
    pub name: Option<String>,
    pub normalized_version: Option<String>,
    pub version_confidence: VersionConfidence,
    pub prerelease: bool,
    pub draft: bool,
    pub published_at: Option<String>,
    pub recommended_asset: Option<String>,
    pub assets: Vec<GitReleaseAssetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitReleaseAssetEntry {
    pub name: String,
    pub url: String,
    pub kind: AssetKind,
    pub format: AssetFormat,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub libc: Option<String>,
    pub compatibility: AssetCompatibility,
    pub score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetKind {
    Payload,
    Checksum,
    Signature,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetFormat {
    TarGz,
    TarXz,
    TarZst,
    Zip,
    AppImage,
    Deb,
    Rpm,
    Apk,
    PacmanPackage,
    RawBinary,
    Checksum,
    Signature,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetCompatibility {
    NativeExact,
    NativePartial,
    Foreign,
    Unknown,
    Sidecar,
}

#[derive(Debug, Error)]
pub enum ReleaseInspectError {
    #[error(
        "unsupported release target `{0}`; use a GitHub/GitLab/Gitea repository URL or owner/repo"
    )]
    UnsupportedTarget(String),
    #[error("{provider} release lookup failed for `{repo}`: {detail}")]
    LookupFailed {
        provider: String,
        repo: String,
        detail: String,
    },
    #[error("{provider} release response for `{repo}` could not be decoded: {source}")]
    Decode {
        provider: String,
        repo: String,
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseProvider {
    Github,
    Gitlab,
    Gitea,
    Forgejo,
    Sourcehut,
    Direct,
}

pub fn inspect_releases(
    target: &str,
    max_releases: usize,
) -> Result<GitReleaseReport, ReleaseInspectError> {
    let release_target = parse_release_target(target)
        .ok_or_else(|| ReleaseInspectError::UnsupportedTarget(target.to_owned()))?;
    if max_releases == 0 {
        return Ok(GitReleaseReport {
            target: target.to_owned(),
            repo: release_target.repo.clone(),
            source: release_target.source(),
            releases: Vec::new(),
        });
    }

    let releases = fetch_provider_releases(&release_target, max_releases)?
        .into_iter()
        .map(release_entry)
        .collect();
    Ok(GitReleaseReport {
        target: target.to_owned(),
        repo: release_target.repo.clone(),
        source: release_target.source(),
        releases,
    })
}

pub fn inspect_github_releases(
    target: &str,
    max_releases: usize,
) -> Result<GitReleaseReport, ReleaseInspectError> {
    inspect_releases(target, max_releases)
}

fn release_entry(release: ReleaseResponse) -> GitReleaseEntry {
    let (normalized_version, version_confidence) = normalize_tag_version(&release.tag_name);
    let assets = release
        .assets
        .into_iter()
        .map(classify_release_asset)
        .collect::<Vec<_>>();
    let recommended_asset = assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::Payload)
        .max_by_key(|asset| asset.score)
        .filter(|asset| asset.score > 0)
        .map(|asset| asset.name.clone());

    GitReleaseEntry {
        tag: release.tag_name,
        name: release.name,
        normalized_version,
        version_confidence,
        prerelease: release.prerelease,
        draft: release.draft,
        published_at: release.published_at,
        recommended_asset,
        assets,
    }
}
