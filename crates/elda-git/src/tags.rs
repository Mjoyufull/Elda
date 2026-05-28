use std::process::Command;
use std::str::FromStr;

use elda_types::PackageVersion;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitTagReport {
    pub target: String,
    pub tags: Vec<GitTagEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitTagEntry {
    pub tag: String,
    pub object: String,
    pub normalized_version: Option<String>,
    pub version_confidence: VersionConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VersionConfidence {
    StableSemver,
    SemverPrerelease,
    DateLike,
    Raw,
}

#[derive(Debug, Error)]
pub enum GitInspectError {
    #[error("git tag lookup failed for `{target}`: {detail}")]
    LookupFailed { target: String, detail: String },
    #[error("git command failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitTagOptions {
    pub max_tags: usize,
    pub include_prereleases: bool,
    pub strip_v_prefix: bool,
    pub allow_date_versions: bool,
    pub allow_raw_versions: bool,
}

impl Default for GitTagOptions {
    fn default() -> Self {
        Self {
            max_tags: 50,
            include_prereleases: true,
            strip_v_prefix: true,
            allow_date_versions: true,
            allow_raw_versions: true,
        }
    }
}

pub fn list_remote_tags(target: &str, max_tags: usize) -> Result<GitTagReport, GitInspectError> {
    list_remote_tags_with_options(
        target,
        GitTagOptions {
            max_tags,
            ..GitTagOptions::default()
        },
    )
}

pub fn list_remote_tags_with_options(
    target: &str,
    options: GitTagOptions,
) -> Result<GitTagReport, GitInspectError> {
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--refs", target])
        .output()?;
    if !output.status.success() {
        return Err(GitInspectError::LookupFailed {
            target: target.to_owned(),
            detail: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tags = stdout
        .lines()
        .filter_map(|line| parse_ls_remote_tag_line_with_options(line, &options))
        .filter(|tag| {
            options.include_prereleases
                || tag.version_confidence != VersionConfidence::SemverPrerelease
        })
        .filter(|tag| {
            options.allow_date_versions || tag.version_confidence != VersionConfidence::DateLike
        })
        .filter(|tag| {
            options.allow_raw_versions || tag.version_confidence != VersionConfidence::Raw
        })
        .collect::<Vec<_>>();
    tags.sort_by(compare_tags_newest_first);
    if options.max_tags > 0 {
        tags.truncate(options.max_tags);
    }

    Ok(GitTagReport {
        target: target.to_owned(),
        tags,
    })
}

#[cfg(test)]
pub(crate) fn parse_ls_remote_tag_line(line: &str) -> Option<GitTagEntry> {
    parse_ls_remote_tag_line_with_options(line, &GitTagOptions::default())
}

pub(crate) fn parse_ls_remote_tag_line_with_options(
    line: &str,
    options: &GitTagOptions,
) -> Option<GitTagEntry> {
    let (object, reference) = line.split_once('\t')?;
    let tag = reference.strip_prefix("refs/tags/")?;
    let (normalized_version, version_confidence) = normalize_tag_version_with_options(tag, options);
    Some(GitTagEntry {
        tag: tag.to_owned(),
        object: object.to_owned(),
        normalized_version,
        version_confidence,
    })
}

pub(crate) fn normalize_tag_version(tag: &str) -> (Option<String>, VersionConfidence) {
    normalize_tag_version_with_options(tag, &GitTagOptions::default())
}

pub(crate) fn normalize_tag_version_with_options(
    tag: &str,
    options: &GitTagOptions,
) -> (Option<String>, VersionConfidence) {
    let stripped = if options.strip_v_prefix {
        tag.strip_prefix('v').unwrap_or(tag)
    } else {
        tag
    };
    let candidate = normalize_version_text(stripped);
    if candidate.is_empty() {
        return (None, VersionConfidence::Raw);
    }

    let confidence = if is_stable_semver(stripped) {
        VersionConfidence::StableSemver
    } else if is_prerelease_semver(stripped) {
        VersionConfidence::SemverPrerelease
    } else if is_date_like(stripped) {
        VersionConfidence::DateLike
    } else {
        VersionConfidence::Raw
    };

    if confidence == VersionConfidence::DateLike && !options.allow_date_versions {
        return (None, confidence);
    }
    if confidence == VersionConfidence::Raw && !options.allow_raw_versions {
        return (None, confidence);
    }

    let normalized = format!("0:{candidate}-1");
    if PackageVersion::from_str(&normalized).is_ok() {
        (Some(normalized), confidence)
    } else {
        (None, VersionConfidence::Raw)
    }
}

fn normalize_version_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '+' | '~'))
        .collect()
}

fn is_stable_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    matches!(parts.as_slice(), [major, minor, patch] if numeric(major) && numeric(minor) && numeric(patch))
}

fn is_prerelease_semver(value: &str) -> bool {
    let Some((base, suffix)) = value.split_once('-') else {
        return false;
    };
    is_stable_semver(base) && !suffix.trim().is_empty()
}

fn is_date_like(value: &str) -> bool {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    digits >= 6 && value.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
}

fn numeric(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn compare_tags_newest_first(left: &GitTagEntry, right: &GitTagEntry) -> std::cmp::Ordering {
    match (&left.normalized_version, &right.normalized_version) {
        (Some(left_version), Some(right_version)) => {
            let left_parsed = PackageVersion::from_str(left_version);
            let right_parsed = PackageVersion::from_str(right_version);
            match (left_parsed, right_parsed) {
                (Ok(left), Ok(right)) => right.cmp(&left),
                _ => left.tag.cmp(&right.tag),
            }
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.tag.cmp(&right.tag),
    }
}

#[cfg(test)]
mod tests {
    use super::{VersionConfidence, parse_ls_remote_tag_line};

    #[test]
    fn parses_ls_remote_tag_and_normalizes_stable_semver() {
        let tag = parse_ls_remote_tag_line("0123456789abcdef\trefs/tags/v1.2.3")
            .expect("tag should parse");

        assert_eq!(tag.tag, "v1.2.3");
        assert_eq!(tag.object, "0123456789abcdef");
        assert_eq!(tag.normalized_version.as_deref(), Some("0:1.2.3-1"));
        assert_eq!(tag.version_confidence, VersionConfidence::StableSemver);
    }

    #[test]
    fn prerelease_tags_are_visible_with_lower_confidence() {
        let tag = parse_ls_remote_tag_line("0123456789abcdef\trefs/tags/v1.2.3-rc1")
            .expect("tag should parse");

        assert_eq!(tag.normalized_version.as_deref(), Some("0:1.2.3rc1-1"));
        assert_eq!(tag.version_confidence, VersionConfidence::SemverPrerelease);
    }

    #[test]
    fn non_tag_refs_are_ignored() {
        assert!(parse_ls_remote_tag_line("abc\trefs/heads/main").is_none());
    }
}
