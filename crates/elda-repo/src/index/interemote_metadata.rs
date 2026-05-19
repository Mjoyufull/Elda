use std::fs;
use std::path::{Path, PathBuf};

use crate::error::RepoError;
use crate::model::{RemoteDocument, SyncedPackageRecord};

use super::interemote_render::render_pkg_lua;
use super::{Candidate, InteremoteKind, PackageMetadata};

pub(super) fn candidate_metadata(candidate: &Candidate) -> Result<PackageMetadata, RepoError> {
    match candidate.kind {
        InteremoteKind::GentooOverlay => gentoo_metadata(candidate),
        InteremoteKind::XbpsSrc => xbps_metadata(candidate),
    }
}

pub(super) fn candidate_metadata_path(candidate: &Candidate) -> String {
    candidate
        .metadata_path
        .file_name()
        .map(|file_name| {
            candidate
                .rel_path
                .join(file_name)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| candidate.metadata_path.to_string_lossy().into_owned())
}

pub(super) fn metadata_fields(kind: InteremoteKind) -> Vec<String> {
    let fields: &[&str] = match kind {
        InteremoteKind::GentooOverlay => &[
            "DESCRIPTION",
            "HOMEPAGE",
            "LICENSE",
            "RDEPEND",
            "BDEPEND",
            "IUSE",
        ],
        InteremoteKind::XbpsSrc => &[
            "pkgname",
            "version",
            "revision",
            "short_desc",
            "homepage",
            "license",
            "depends",
            "makedepends",
            "checkdepends",
        ],
    };
    fields.iter().map(|field| (*field).to_owned()).collect()
}

pub(super) fn scan_candidates(
    root: &Path,
    kind: InteremoteKind,
) -> Result<Vec<Candidate>, RepoError> {
    match kind {
        InteremoteKind::GentooOverlay => scan_gentoo_candidates(root),
        InteremoteKind::XbpsSrc => scan_xbps_candidates(root),
    }
}

fn scan_gentoo_candidates(root: &Path) -> Result<Vec<Candidate>, RepoError> {
    let mut candidates = Vec::new();
    for category in fs::read_dir(root)?.flatten().map(|entry| entry.path()) {
        if !category.is_dir() || is_skipped_gentoo_category(&category) {
            continue;
        }
        for package in fs::read_dir(&category)?.flatten().map(|entry| entry.path()) {
            if !package.is_dir() {
                continue;
            }
            let Some(ebuild) = first_file_with_extension(&package, "ebuild")? else {
                continue;
            };
            let Some(name) = package.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let rel_path = package.strip_prefix(root).unwrap_or(&package).to_path_buf();
            candidates.push(Candidate {
                name: name.to_owned(),
                rel_path,
                metadata_path: ebuild,
                kind: InteremoteKind::GentooOverlay,
            });
        }
    }
    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(candidates)
}

fn scan_xbps_candidates(root: &Path) -> Result<Vec<Candidate>, RepoError> {
    let mut candidates = Vec::new();
    let srcpkgs = root.join("srcpkgs");
    for package in fs::read_dir(srcpkgs)?.flatten().map(|entry| entry.path()) {
        if !package.is_dir() || !package.join("template").is_file() {
            continue;
        }
        let Some(name) = package.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let rel_path = package.strip_prefix(root).unwrap_or(&package).to_path_buf();
        candidates.push(Candidate {
            name: name.to_owned(),
            rel_path,
            metadata_path: package.join("template"),
            kind: InteremoteKind::XbpsSrc,
        });
    }
    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(candidates)
}

pub(super) fn package_record(
    remote: &RemoteDocument,
    candidate: &Candidate,
    commit: Option<&str>,
) -> Result<SyncedPackageRecord, RepoError> {
    let metadata = candidate_metadata(candidate)?;
    let pkg_lua = render_pkg_lua(remote, candidate, &metadata, commit);

    Ok(SyncedPackageRecord {
        remote_name: remote.name.clone(),
        remote_priority: remote.priority,
        pkgname: candidate.name.clone(),
        epoch: 0,
        pkgver: metadata.version.unwrap_or_else(|| "0.1.0".to_owned()),
        pkgrel: metadata.rel,
        arch: vec!["amd64".to_owned()],
        package_kind: "normal".to_owned(),
        variant_id: None,
        summary: metadata.description.clone(),
        description: metadata.description,
        homepage: metadata.homepage,
        license: metadata.license.first().cloned(),
        channel: Some(remote.channel.clone()),
        asset_url: None,
        sha256: None,
        size: None,
        payload_sig: None,
        sbom_url: None,
        attestation_url: None,
        source_kind: Some("interemote".to_owned()),
        source_ref: commit.map(ToOwned::to_owned),
        fallback_git_url: Some(remote.index_url.clone()),
        repo_commit: commit.map(ToOwned::to_owned),
        release_tag: None,
        pkg_lua,
    })
}

fn gentoo_metadata(candidate: &Candidate) -> Result<PackageMetadata, RepoError> {
    let contents = fs::read_to_string(&candidate.metadata_path)?;
    let iuse = split_words(assignment_value(&contents, "IUSE").as_deref());
    let (flags_default, flags_allowed) = gentoo_flags(&iuse);
    Ok(PackageMetadata {
        description: assignment_value(&contents, "DESCRIPTION"),
        homepage: assignment_value(&contents, "HOMEPAGE"),
        license: split_words(assignment_value(&contents, "LICENSE").as_deref()),
        version: gentoo_version(&candidate.metadata_path),
        rel: 1,
        depends: dependency_tokens(assignment_value(&contents, "RDEPEND").as_deref()),
        makedepends: dependency_tokens(assignment_value(&contents, "BDEPEND").as_deref()),
        checkdepends: Vec::new(),
        flags_default,
        flags_allowed,
    })
}

fn xbps_metadata(candidate: &Candidate) -> Result<PackageMetadata, RepoError> {
    let contents = fs::read_to_string(&candidate.metadata_path)?;
    Ok(PackageMetadata {
        description: assignment_value(&contents, "short_desc"),
        homepage: assignment_value(&contents, "homepage"),
        license: split_words(assignment_value(&contents, "license").as_deref()),
        version: assignment_value(&contents, "version"),
        rel: assignment_value(&contents, "revision")
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(1),
        depends: dependency_tokens(assignment_value(&contents, "depends").as_deref()),
        makedepends: dependency_tokens(assignment_value(&contents, "makedepends").as_deref()),
        checkdepends: dependency_tokens(assignment_value(&contents, "checkdepends").as_deref()),
        flags_default: Vec::new(),
        flags_allowed: Vec::new(),
    })
}

fn assignment_value(contents: &str, key: &str) -> Option<String> {
    let mut lines = contents.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || !trimmed.starts_with(key) {
            continue;
        }
        let value = trimmed
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim();
        let mut collected = value.to_owned();
        if starts_multiline(value) && !ends_multiline(value) {
            for next in lines.by_ref() {
                collected.push('\n');
                collected.push_str(next.trim());
                if ends_multiline(next.trim()) {
                    break;
                }
            }
        }
        return Some(clean_scalar(&collected));
    }
    None
}

fn starts_multiline(value: &str) -> bool {
    value.starts_with('"') || value.starts_with('\'')
}

fn ends_multiline(value: &str) -> bool {
    (value.ends_with('"') && !value.ends_with("\\\""))
        || (value.ends_with('\'') && !value.ends_with("\\'"))
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('(')
        .trim_matches(')')
        .trim()
        .to_owned()
}

fn split_words(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(clean_scalar)
        .filter(|entry| !entry.is_empty() && *entry != "(" && *entry != ")")
        .collect()
}

fn dependency_tokens(value: Option<&str>) -> Vec<String> {
    split_words(value)
        .into_iter()
        .filter(|token| {
            !token.starts_with("${")
                && token != "||"
                && token != "("
                && token != ")"
                && !token.ends_with('?')
        })
        .collect()
}

fn gentoo_flags(iuse: &[String]) -> (Vec<String>, Vec<String>) {
    let mut defaults = Vec::new();
    let mut allowed = Vec::new();
    for raw in iuse {
        let flag = raw.trim_start_matches(['+', '-']);
        if flag.is_empty() {
            continue;
        }
        allowed.push(flag.to_owned());
        if raw.starts_with('+') {
            defaults.push(flag.to_owned());
        }
    }
    allowed.sort();
    allowed.dedup();
    defaults.sort();
    defaults.dedup();
    (defaults, allowed)
}

fn gentoo_version(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    stem.rsplit_once('-').map(|(_, version)| version.to_owned())
}

fn first_file_with_extension(dir: &Path, extension: &str) -> Result<Option<PathBuf>, RepoError> {
    let mut entries = fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(extension))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries.into_iter().next())
}

fn is_skipped_gentoo_category(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("profiles" | "metadata" | "scripts" | "eclass" | "licenses")
    )
}
