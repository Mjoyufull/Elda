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

    let release_asset_kind = source.kind == "github_release"
        || source.kind == "release_asset"
        || source.kind == "appimage";
    if !release_asset_kind && !source.github_release_assets.is_empty() {
        issues.push(error(format!(
            "source.kind `{}` does not support `assets = {{ ... }}`",
            source.kind
        )));
    }

    match source.kind.as_str() {
        "url_archive" => require_fields(source, &["url", "sha256"], issues),
        "github_release" | "release_asset" => {
            validate_release_asset_source(source, package_arches, issues, canonical_arches);
        }
        "appimage" => validate_appimage_source(source, package_arches, issues, canonical_arches),
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
        "aur_pkgbuild" | "xbps_template" => require_fields(source, &["url"], issues),
        _ => {}
    }
}

fn validate_appimage_source(
    source: &SourceLaneDefinition,
    package_arches: &[String],
    issues: &mut Vec<ValidationIssue>,
    canonical_arches: &[&str],
) {
    validate_appimage_lane_constraints(source, issues);

    if scalar_string(source, "url").is_some() {
        for key in ["repo", "tag", "release", "provider", "host"] {
            if source.fields.contains_key(key) {
                issues.push(error(format!("appimage url source must not set `{key}`")));
            }
        }
        if !source.github_release_assets.is_empty() {
            issues.push(error(
                "appimage url sources cannot use `assets = { ... }`".to_owned(),
            ));
        }
        require_fields(source, &["url", "sha256", "binary"], issues);
        validate_appimage_integration(source, issues);
        validate_appimage_binary_names(source, issues);
        return;
    }

    validate_release_asset_source(source, package_arches, issues, canonical_arches);
    validate_appimage_binary_requirements(source, package_arches, issues);
    validate_appimage_integration(source, issues);
    validate_appimage_binary_names(source, issues);
}

fn validate_appimage_lane_constraints(
    source: &SourceLaneDefinition,
    issues: &mut Vec<ValidationIssue>,
) {
    for key in ["strip_components", "subdir", "rename"] {
        if source.fields.contains_key(key) {
            issues.push(error(format!(
                "appimage source must not set `{key}` (managed AppImages are not extracted from archives)"
            )));
        }
    }
    for (arch, asset) in &source.github_release_assets {
        if asset.strip_components.is_some() || asset.subdir.is_some() || asset.rename.is_some() {
            issues.push(error(format!(
                "appimage assets.{arch} must not set strip_components, subdir, or rename"
            )));
        }
    }
}

fn validate_appimage_integration(source: &SourceLaneDefinition, issues: &mut Vec<ValidationIssue>) {
    let Some(value) = scalar_string(source, "integration") else {
        return;
    };
    if !matches!(value, "desktop" | "none") {
        issues.push(error(format!(
            "appimage source `integration` must be `desktop` or `none`, got `{value}`"
        )));
    }
}

fn validate_appimage_binary_requirements(
    source: &SourceLaneDefinition,
    package_arches: &[String],
    issues: &mut Vec<ValidationIssue>,
) {
    if !source.github_release_assets.is_empty() {
        for package_arch in package_arches {
            let Some(asset) = source.github_release_assets.get(package_arch) else {
                continue;
            };
            if asset
                .binary
                .as_ref()
                .map(|bin| bin.trim().is_empty())
                .unwrap_or(true)
            {
                issues.push(error(format!(
                    "appimage source requires assets.{package_arch}.binary"
                )));
            }
        }
    } else {
        require_fields(source, &["binary"], issues);
    }
}

fn validate_appimage_binary_names(
    source: &SourceLaneDefinition,
    issues: &mut Vec<ValidationIssue>,
) {
    if let Some(bin) = scalar_string(source, "binary")
        && (bin.contains('/') || bin.contains(".."))
    {
        issues.push(error(
            "appimage `binary` must be a single filename (launcher in /usr/bin)".to_owned(),
        ));
    }
    for (arch, asset) in &source.github_release_assets {
        if let Some(bin) = asset.binary.as_deref()
            && (bin.contains('/') || bin.contains(".."))
        {
            issues.push(error(format!(
                "appimage assets.{arch}.binary must be a single filename"
            )));
        }
    }
}

fn validate_release_asset_source(
    source: &SourceLaneDefinition,
    package_arches: &[String],
    issues: &mut Vec<ValidationIssue>,
    canonical_arches: &[&str],
) {
    if source.kind == "release_asset" {
        require_fields(source, &["provider"], issues);
        if let Some(provider) = scalar_string(source, "provider")
            && !supported_release_provider(provider)
        {
            issues.push(error(format!(
                "release_asset provider `{provider}` is not implemented by the current build slice"
            )));
        }
    }
    require_fields(source, &["repo"], issues);
    validate_release_host_field(source, issues);

    let has_asset = source.fields.contains_key("asset");
    let has_sha256 = source.fields.contains_key("sha256");
    if has_asset != has_sha256 {
        issues.push(error(format!(
            "{} top-level `asset` and `sha256` must either both be set or both be omitted",
            source.kind
        )));
    }

    if !has_asset && source.github_release_assets.is_empty() {
        issues.push(error(format!(
            "{} source requires either top-level `asset` + `sha256` or `assets = {{ <arch> = {{ ... }} }}`",
            source.kind
        )));
    }

    validate_top_level_release_signature(source, issues);
    validate_github_release_assets(source, package_arches, issues, canonical_arches);

    let has_tag = source.fields.contains_key("tag");
    let has_release = source.fields.contains_key("release");
    if !has_tag && !has_release {
        issues.push(error(format!(
            "{} source requires either `tag` or `release`",
            source.kind
        )));
    }
}

fn validate_release_host_field(source: &SourceLaneDefinition, issues: &mut Vec<ValidationIssue>) {
    let Some(host) = scalar_string(source, "host") else {
        return;
    };
    if source.kind != "release_asset" && source.kind != "appimage" {
        issues.push(error(
            "source.host is only valid for release_asset or appimage sources".to_owned(),
        ));
        return;
    }
    if host.contains('/') || host.contains(':') || host.trim().is_empty() {
        issues.push(error(
            "release_asset host must be a bare forge host such as `gitlab.example.org`".to_owned(),
        ));
    }
}

fn supported_release_provider(provider: &str) -> bool {
    matches!(
        provider,
        "github" | "gitlab" | "gitea" | "forgejo" | "sourcehut" | "direct"
    )
}

fn scalar_string<'a>(source: &'a SourceLaneDefinition, key: &str) -> Option<&'a str> {
    match source.fields.get(key)? {
        crate::model::ScalarValue::String(value) => Some(value.as_str()),
        _ => None,
    }
}

fn validate_top_level_release_signature(
    source: &SourceLaneDefinition,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(signature) = scalar_string(source, "signature") else {
        return;
    };
    validate_release_signature_value(source.kind.as_str(), "signature", signature, issues);
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
                "{} source is missing an `assets.{package_arch}` entry for package arch `{package_arch}`",
                source.kind
            )));
        }
    }

    for (arch, asset) in &source.github_release_assets {
        if !canonical_arches.contains(&arch.as_str()) {
            issues.push(error(format!(
                "release asset assets key `{arch}` is not a canonical architecture label"
            )));
        }
        if asset.asset.trim().is_empty() {
            issues.push(error(format!(
                "release asset assets.{arch}.asset must not be empty"
            )));
        }
        if asset.sha256.trim().is_empty() {
            issues.push(error(format!(
                "release asset assets.{arch}.sha256 must not be empty"
            )));
        }
        if let Some(signature) = asset.signature.as_deref() {
            validate_release_signature_value(
                source.kind.as_str(),
                &format!("assets.{arch}.signature"),
                signature,
                issues,
            );
        }
        if asset
            .binary
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "release asset assets.{arch}.binary must not be empty"
            )));
        }
        if asset
            .subdir
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "release asset assets.{arch}.subdir must not be empty"
            )));
        }
        if asset
            .rename
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            issues.push(error(format!(
                "release asset assets.{arch}.rename must not be empty"
            )));
        }
        if asset.strip_components.is_some_and(|value| value < 0) {
            issues.push(error(format!(
                "release asset assets.{arch}.strip_components cannot be negative"
            )));
        }
    }
}

fn validate_release_signature_value(
    source_kind: &str,
    field: &str,
    signature: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let signature = signature.trim();
    if signature.is_empty() {
        issues.push(error(format!(
            "{source_kind} source `{field}` must not be empty"
        )));
        return;
    }
    if signature.contains("..") {
        issues.push(error(format!(
            "{source_kind} source `{field}` must not contain parent-directory traversal"
        )));
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
