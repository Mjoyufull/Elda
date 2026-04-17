use crate::model::{
    IssueSeverity, PackageDefinition, SourceLaneDefinition, ValidationIssue, infer_lane_name,
};

pub(super) fn validate_source(
    package: &PackageDefinition,
    issues: &mut Vec<ValidationIssue>,
    canonical_arches: &[&str],
    source_kinds: &[&str],
) {
    let source = &package.source;
    if source.is_multi_lane() {
        if !source.kind.is_empty()
            || !source.fields.is_empty()
            || !source.github_release_assets.is_empty()
        {
            issues.push(error(
                "source.lanes cannot be combined with single-lane source fields".to_owned(),
            ));
        }
        if source.lanes.is_empty() {
            issues.push(error(
                "source.lanes must contain at least one declared lane".to_owned(),
            ));
            return;
        }

        if let Some(default_lane) = &source.default_lane
            && !source.lanes.contains_key(default_lane)
        {
            issues.push(error(format!(
                "source.default_lane `{default_lane}` does not match a declared lane"
            )));
        }

        for (lane_name, lane) in &source.lanes {
            if lane_name != "source" && lane_name != "binary" {
                issues.push(error(format!(
                    "source.lanes supports only `source` and `binary`, found `{lane_name}`"
                )));
            }

            validate_source_lane(lane, &package.arch, issues, canonical_arches, source_kinds);

            match (lane_name.as_str(), infer_lane_name(&lane.kind)) {
                ("source", Some("binary")) => issues.push(warning(format!(
                    "source.lanes.source uses binary-style source kind `{}`",
                    lane.kind
                ))),
                ("binary", Some("source")) => issues.push(warning(format!(
                    "source.lanes.binary uses source-build-style source kind `{}`",
                    lane.kind
                ))),
                _ => {}
            }
        }

        return;
    }

    if source.default_lane.is_some() {
        issues.push(error(
            "source.default_lane is only valid when source.lanes is present".to_owned(),
        ));
    }

    validate_source_lane(
        &SourceLaneDefinition {
            kind: source.kind.clone(),
            fields: source.fields.clone(),
            github_release_assets: source.github_release_assets.clone(),
        },
        &package.arch,
        issues,
        canonical_arches,
        source_kinds,
    );
}

fn validate_source_lane(
    source: &SourceLaneDefinition,
    package_arches: &[String],
    issues: &mut Vec<ValidationIssue>,
    canonical_arches: &[&str],
    source_kinds: &[&str],
) {
    if !source_kinds.contains(&source.kind.as_str()) {
        issues.push(error(format!(
            "source.kind must be one of: {}",
            source_kinds.join(", ")
        )));
        return;
    }

    if source.kind != "github_release" && !source.github_release_assets.is_empty() {
        issues.push(error(format!(
            "source.kind `{}` does not support `assets = {{ ... }}`",
            source.kind
        )));
    }

    match source.kind.as_str() {
        "url_archive" => require_fields(source, &["url", "sha256"], issues),
        "github_release" => {
            require_fields(source, &["repo"], issues);

            let has_asset = source.fields.contains_key("asset");
            let has_sha256 = source.fields.contains_key("sha256");
            if has_asset != has_sha256 {
                issues.push(error(
                    "github_release top-level `asset` and `sha256` must either both be set or both be omitted".to_owned(),
                ));
            }

            if !has_asset && source.github_release_assets.is_empty() {
                issues.push(error(
                    "github_release source requires either top-level `asset` + `sha256` or `assets = { <arch> = { ... } }`".to_owned(),
                ));
            }

            validate_github_release_assets(source, package_arches, issues, canonical_arches);

            let has_tag = source.fields.contains_key("tag");
            let has_release = source.fields.contains_key("release");
            if !has_tag && !has_release {
                issues.push(error(
                    "github_release source requires either `tag` or `release`".to_owned(),
                ));
            }
        }
        "git" => {
            require_fields(source, &["url"], issues);
            let refs = ["rev", "tag", "branch"]
                .into_iter()
                .filter(|field| source.fields.contains_key(*field))
                .count();
            if refs != 1 {
                issues.push(error(
                    "git source requires exactly one of `rev`, `tag`, or `branch`".to_owned(),
                ));
            }
        }
        "nix_flake" => require_fields(source, &["url"], issues),
        "gentoo_overlay" => require_fields(source, &["url", "package"], issues),
        _ => {}
    }
}

fn validate_github_release_assets(
    source: &SourceLaneDefinition,
    package_arches: &[String],
    issues: &mut Vec<ValidationIssue>,
    canonical_arches: &[&str],
) {
    if source.github_release_assets.is_empty() {
        return;
    }

    for package_arch in package_arches {
        if !source.github_release_assets.contains_key(package_arch) {
            issues.push(error(format!(
                "github_release source is missing an `assets.{package_arch}` entry for package arch `{package_arch}`"
            )));
        }
    }

    for (arch, asset) in &source.github_release_assets {
        if !canonical_arches.contains(&arch.as_str()) {
            issues.push(error(format!(
                "github_release assets key `{arch}` is not a canonical architecture label"
            )));
        }
        if asset.asset.trim().is_empty() {
            issues.push(error(format!(
                "github_release assets.{arch}.asset must not be empty"
            )));
        }
        if asset.sha256.trim().is_empty() {
            issues.push(error(format!(
                "github_release assets.{arch}.sha256 must not be empty"
            )));
        }
        if asset
            .binary
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "github_release assets.{arch}.binary must not be empty"
            )));
        }
        if asset
            .subdir
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "github_release assets.{arch}.subdir must not be empty"
            )));
        }
        if asset
            .rename
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "github_release assets.{arch}.rename must not be empty"
            )));
        }
        if asset.strip_components.is_some_and(|value| value < 0) {
            issues.push(error(format!(
                "github_release assets.{arch}.strip_components cannot be negative"
            )));
        }
    }
}

fn require_fields(
    source: &SourceLaneDefinition,
    required: &[&str],
    issues: &mut Vec<ValidationIssue>,
) {
    for field in required {
        if !source.fields.contains_key(*field) {
            issues.push(error(format!(
                "{} source is missing required field `{field}`",
                source.kind
            )));
        }
    }
}

fn error(message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: IssueSeverity::Error,
        message: message.into(),
    }
}

fn warning(message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: IssueSeverity::Warning,
        message: message.into(),
    }
}
