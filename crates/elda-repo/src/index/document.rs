use super::*;
use crate::model::DEFAULT_REMOTE_CHANNEL;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum RemoteIndexDocument {
    Envelope { packages: Vec<RemotePackageInput> },
    Array(Vec<RemotePackageInput>),
}

#[derive(Debug, Deserialize)]
pub(super) struct RemotePackageInput {
    #[serde(default)]
    pkgname: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    asset_url: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    payload_sig: Option<String>,
    #[serde(default)]
    sbom_url: Option<String>,
    #[serde(default)]
    attestation_url: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    source_ref: Option<String>,
    #[serde(default)]
    fallback_git_url: Option<String>,
    #[serde(default)]
    repo_commit: Option<String>,
    #[serde(default)]
    release_tag: Option<String>,
    #[serde(default)]
    variant_id: Option<String>,
    pkg_lua: String,
}

pub(super) fn parse_remote_packages_from_content(
    remote: &RemoteDocument,
    content: &str,
) -> Result<Vec<SyncedPackageRecord>, RepoError> {
    let document = parse_index_document(content)?;
    let inputs = match document {
        RemoteIndexDocument::Envelope { packages } | RemoteIndexDocument::Array(packages) => {
            packages
        }
    };

    let packages = inputs
        .into_iter()
        .map(|input| normalize_remote_package(remote, input))
        .collect::<Result<Vec<_>, _>>()?;

    filter_packages_for_channel(remote, packages)
}

fn normalize_remote_package(
    remote: &RemoteDocument,
    input: RemotePackageInput,
) -> Result<SyncedPackageRecord, RepoError> {
    let synthetic_name = if input.pkgname.is_empty() {
        "pkg"
    } else {
        input.pkgname.as_str()
    };
    let synthetic_path =
        PathBuf::from(format!("remote://{}/{synthetic_name}/pkg.lua", remote.name));
    let recipe = parse_pkg_lua(&synthetic_path, &input.pkg_lua)
        .map_err(|error| RepoError::Parse(format!("remote `{}`: {error}", remote.name)))?;
    let pkgname = if input.pkgname.is_empty() {
        recipe.package.name.clone()
    } else {
        input.pkgname
    };

    if pkgname != recipe.package.name {
        return Err(RepoError::Parse(format!(
            "remote `{}` record `{pkgname}` does not match pkg.lua package name `{}`",
            remote.name, recipe.package.name
        )));
    }

    Ok(SyncedPackageRecord {
        remote_name: remote.name.clone(),
        remote_priority: remote.priority,
        pkgname,
        epoch: recipe.package.epoch,
        pkgver: recipe.package.version.clone(),
        pkgrel: recipe.package.rel,
        arch: recipe.package.arch.clone(),
        package_kind: recipe.package.kind.clone(),
        variant_id: input.variant_id,
        summary: input.summary,
        description: input.description,
        homepage: input.homepage,
        license: input.license,
        channel: input.channel,
        asset_url: input.asset_url,
        sha256: input.sha256,
        size: input.size,
        payload_sig: input.payload_sig,
        sbom_url: input.sbom_url,
        attestation_url: input.attestation_url,
        source_kind: input.source_kind,
        source_ref: input.source_ref,
        fallback_git_url: input.fallback_git_url,
        repo_commit: input.repo_commit,
        release_tag: input.release_tag,
        pkg_lua: input.pkg_lua,
    })
}

fn filter_packages_for_channel(
    remote: &RemoteDocument,
    packages: Vec<SyncedPackageRecord>,
) -> Result<Vec<SyncedPackageRecord>, RepoError> {
    if packages.is_empty() {
        return Ok(packages);
    }

    let selected = remote.channel.as_str();
    let filtered = packages
        .into_iter()
        .filter(|package| package_matches_channel(package, selected))
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        return Err(RepoError::Parse(format!(
            "remote `{}` does not publish selected channel `{selected}`",
            remote.name
        )));
    }

    Ok(filtered)
}

fn package_matches_channel(package: &SyncedPackageRecord, selected: &str) -> bool {
    match package.channel.as_deref() {
        Some(channel) => channel == selected,
        None => selected == DEFAULT_REMOTE_CHANNEL,
    }
}

fn parse_index_document(content: &str) -> Result<RemoteIndexDocument, RepoError> {
    let trimmed = content.trim_start();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && let Ok(document) = serde_json::from_str(trimmed)
    {
        return Ok(document);
    }

    toml::from_str(trimmed).map_err(RepoError::from)
}
