use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::RepoError;
use crate::model::{
    InteremotePreview, InteremotePreviewPackage, RemoteDocument, SyncedPackageRecord,
};

#[path = "interemote_metadata.rs"]
mod interemote_metadata;
#[path = "interemote_render.rs"]
mod interemote_render;

use interemote_metadata::{
    candidate_metadata, candidate_metadata_path, metadata_fields, package_record, scan_candidates,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InteremoteKind {
    GentooOverlay,
    XbpsSrc,
}

#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub(super) name: String,
    pub(super) rel_path: PathBuf,
    pub(super) metadata_path: PathBuf,
    pub(super) kind: InteremoteKind,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PackageMetadata {
    pub(super) description: Option<String>,
    pub(super) homepage: Option<String>,
    pub(super) license: Vec<String>,
    pub(super) version: Option<String>,
    pub(super) rel: u64,
    pub(super) depends: Vec<String>,
    pub(super) makedepends: Vec<String>,
    pub(super) checkdepends: Vec<String>,
    pub(super) flags_default: Vec<String>,
    pub(super) flags_allowed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InteremoteSyncResult {
    pub packages: Vec<SyncedPackageRecord>,
    pub preview: InteremotePreview,
}

pub(super) fn should_try_interemote(remote: &RemoteDocument) -> bool {
    let url = remote.index_url.trim_end_matches('/');
    !(url.ends_with(".toml") || url.ends_with(".json") || url.ends_with(".idx"))
}

pub(super) fn sync_interemote_with_preview(
    remote: &RemoteDocument,
    allowed_git_protocols: &[String],
) -> Result<InteremoteSyncResult, RepoError> {
    let checkout = checkout_interemote(remote, allowed_git_protocols)?;
    let result = (|| {
        let commit = git_commit(&checkout)?;
        let kind = detect_interemote_kind(remote, &checkout)?;
        let candidates = scan_candidates(&checkout, kind)?;
        let preview = interemote_preview(remote, kind, commit.clone(), candidates.clone())?;
        let packages = package_records(remote, &candidates, commit.as_deref());

        Ok(InteremoteSyncResult { packages, preview })
    })();
    finish_with_cleanup(&checkout, result)
}

pub fn preview_interemote(remote: &RemoteDocument) -> Result<InteremotePreview, RepoError> {
    preview_interemote_with_protocols(
        remote,
        &["https".to_owned(), "ssh".to_owned(), "file".to_owned()],
    )
}

pub fn preview_interemote_with_protocols(
    remote: &RemoteDocument,
    allowed_git_protocols: &[String],
) -> Result<InteremotePreview, RepoError> {
    let checkout = checkout_interemote(remote, allowed_git_protocols)?;
    let result = (|| {
        let commit = git_commit(&checkout)?;
        let kind = detect_interemote_kind(remote, &checkout)?;
        let candidates = scan_candidates(&checkout, kind)?;
        interemote_preview(remote, kind, commit, candidates)
    })();
    finish_with_cleanup(&checkout, result)
}

fn checkout_interemote(
    remote: &RemoteDocument,
    allowed_git_protocols: &[String],
) -> Result<PathBuf, RepoError> {
    ensure_git_protocol_allowed(&remote.index_url, allowed_git_protocols)?;
    let checkout = std::env::temp_dir().join(format!(
        "elda-interemote-{}-{}",
        remote.name,
        current_unix_timestamp()
    ));
    remove_dir_if_exists(&checkout)?;

    let mut command = Command::new("git");
    command.args(["clone", "--depth", "1", &remote.index_url]);
    command.arg(&checkout);
    let output = command.output()?;
    if output.status.success() {
        return Ok(checkout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(RepoError::Http(format!(
        "failed to clone interemote `{}` from `{}`: {stderr}",
        remote.name, remote.index_url
    )))
}

fn ensure_git_protocol_allowed(
    location: &str,
    allowed_protocols: &[String],
) -> Result<(), RepoError> {
    let protocol = classify_git_protocol(location);
    if allowed_protocols
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(protocol))
    {
        return Ok(());
    }

    Err(RepoError::Trust(format!(
        "interemote `{location}` uses `{protocol}` transport, which is not allowed by [git].allowed_protocols"
    )))
}

fn classify_git_protocol(location: &str) -> &'static str {
    if location.starts_with("file://") || Path::new(location).exists() {
        return "file";
    }
    if location.starts_with("https://") {
        return "https";
    }
    if location.starts_with("http://") {
        return "http";
    }
    if location.starts_with("ssh://") || looks_like_scp_git_url(location) {
        return "ssh";
    }
    if location.starts_with("git://") {
        return "git";
    }

    "unknown"
}

fn looks_like_scp_git_url(location: &str) -> bool {
    let Some((user_host, path)) = location.split_once(':') else {
        return false;
    };
    !user_host.contains('/') && user_host.contains('@') && !path.is_empty()
}

fn detect_kind(root: &Path) -> Option<InteremoteKind> {
    if root.join("profiles/repo_name").is_file() {
        return Some(InteremoteKind::GentooOverlay);
    }
    if root.join("srcpkgs").is_dir() {
        return Some(InteremoteKind::XbpsSrc);
    }
    None
}

fn detect_interemote_kind(
    remote: &RemoteDocument,
    checkout: &Path,
) -> Result<InteremoteKind, RepoError> {
    detect_kind(checkout).ok_or_else(|| {
        RepoError::Parse(format!(
            "remote `{}` is not an Elda index and does not look like a Gentoo overlay or Void srcpkgs tree",
            remote.name
        ))
    })
}

fn interemote_preview(
    remote: &RemoteDocument,
    kind: InteremoteKind,
    commit: Option<String>,
    candidates: Vec<Candidate>,
) -> Result<InteremotePreview, RepoError> {
    let discovered_count = candidates.len();
    let matched_excludes = candidates
        .iter()
        .filter(|candidate| is_excluded(remote, &candidate.name))
        .map(|candidate| candidate.name.clone())
        .collect::<Vec<_>>();
    let included = candidates
        .into_iter()
        .filter(|candidate| !is_excluded(remote, &candidate.name))
        .collect::<Vec<_>>();
    let included_count = included.len();
    let preview_limit = 12;
    let issue_limit = 32;
    let mut parseable_count = 0;
    let mut packages = Vec::new();
    let mut issues = Vec::new();

    for (index, candidate) in included.iter().enumerate() {
        let metadata = candidate_metadata(candidate);
        let (version, summary, issue) = match metadata {
            Ok(metadata) => {
                parseable_count += 1;
                (metadata.version, metadata.description, None)
            }
            Err(error) => (None, None, Some(error.to_string())),
        };
        let package = InteremotePreviewPackage {
            name: candidate.name.clone(),
            package_path: candidate.rel_path.to_string_lossy().into_owned(),
            metadata_path: candidate_metadata_path(candidate),
            version,
            summary,
            issue,
        };
        if package.issue.is_some() && issues.len() < issue_limit {
            issues.push(package.clone());
        }
        if index < preview_limit {
            packages.push(package);
        }
    }

    Ok(InteremotePreview {
        remote_name: remote.name.clone(),
        index_url: remote.index_url.clone(),
        kind: kind_label(kind).to_owned(),
        parser: parser_label(kind).to_owned(),
        source_kind: source_kind_label(kind).to_owned(),
        commit,
        discovered_count,
        included_count,
        excluded_count: matched_excludes.len(),
        parseable_count,
        preview_limit,
        configured_excludes: remote.exclude.clone(),
        matched_excludes,
        metadata_fields: metadata_fields(kind),
        packages,
        issues,
    })
}

fn package_records(
    remote: &RemoteDocument,
    candidates: &[Candidate],
    commit: Option<&str>,
) -> Vec<SyncedPackageRecord> {
    candidates
        .iter()
        .filter(|candidate| !is_excluded(remote, &candidate.name))
        .filter_map(|candidate| package_record(remote, candidate, commit).ok())
        .collect()
}

fn is_excluded(remote: &RemoteDocument, name: &str) -> bool {
    remote.exclude.iter().any(|excluded| excluded == name)
}

fn kind_label(kind: InteremoteKind) -> &'static str {
    match kind {
        InteremoteKind::GentooOverlay => "gentoo_overlay",
        InteremoteKind::XbpsSrc => "xbps_src",
    }
}

fn source_kind_label(kind: InteremoteKind) -> &'static str {
    match kind {
        InteremoteKind::GentooOverlay => "gentoo_overlay",
        InteremoteKind::XbpsSrc => "xbps_template",
    }
}

fn parser_label(kind: InteremoteKind) -> &'static str {
    match kind {
        InteremoteKind::GentooOverlay => "bounded ebuild metadata parser",
        InteremoteKind::XbpsSrc => "bounded XBPS template parser",
    }
}

fn git_commit(repo: &Path) -> Result<Option<String>, RepoError> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!value.is_empty()).then_some(value))
}

fn finish_with_cleanup<T>(checkout: &Path, result: Result<T, RepoError>) -> Result<T, RepoError> {
    let cleanup = remove_dir_if_exists(checkout);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), RepoError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
