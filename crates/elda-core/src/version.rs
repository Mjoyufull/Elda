use serde_json::{Value, json};

pub const ELDA_VERSION: &str = env!("CARGO_PKG_VERSION");

#[must_use]
pub fn codename() -> Option<&'static str> {
    ELDA_VERSION.split_once('-').map(|(_, name)| name)
}

#[must_use]
pub fn release_number() -> &'static str {
    ELDA_VERSION
        .split_once('-')
        .map_or(ELDA_VERSION, |(n, _)| n)
}

#[must_use]
pub fn version_details() -> Value {
    json!({
        "elda_version": ELDA_VERSION,
        "release": release_number(),
        "codename": codename(),
        "license": env!("CARGO_PKG_LICENSE"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
        "build": build_metadata(),
        "components": component_versions(),
        "schemas": schema_versions(),
    })
}

#[must_use]
pub fn version_details_human_lines() -> Vec<String> {
    let details = version_details();
    let mut lines = vec![
        format!("Version: {}", json_str(&details, "elda_version")),
        format!("Release: {}", json_str(&details, "release")),
    ];
    if let Some(name) = details.get("codename").and_then(Value::as_str) {
        lines.push(format!("Codename: {name}"));
    }
    lines.push(format!("License: {}", json_str(&details, "license")));
    lines.push(format!(
        "Minimum Rust: {}",
        json_str(&details, "rust_version")
    ));

    if let Some(build) = details.get("build").and_then(Value::as_object) {
        lines.push(String::new());
        lines.push("Build".to_owned());
        if let Some(profile) = build.get("profile").and_then(Value::as_str) {
            lines.push(format!("  Profile: {profile}"));
        }
        if let Some(target) = build.get("target").and_then(Value::as_str) {
            lines.push(format!("  Target: {target}"));
        }
        if let Some(date) = build.get("date").and_then(Value::as_str) {
            if !date.is_empty() {
                lines.push(format!("  Built: {date}"));
            }
        }
        if let Some(commit) = build.get("git_commit").and_then(Value::as_str) {
            if !commit.is_empty() {
                lines.push(format!("  Git commit: {commit}"));
            }
        }
    }

    lines.push(String::new());
    lines.push("Components".to_owned());
    if let Some(components) = details.get("components").and_then(Value::as_array) {
        for entry in components {
            if let (Some(name), Some(version)) = (
                entry.get("name").and_then(Value::as_str),
                entry.get("version").and_then(Value::as_str),
            ) {
                lines.push(format!("  {name}: {version}"));
            }
        }
    }

    lines.push(String::new());
    lines.push("Schemas".to_owned());
    if let Some(schemas) = details.get("schemas").and_then(Value::as_object) {
        for (key, value) in schemas {
            lines.push(format!("  {key}: {value}"));
        }
    }

    lines
}

#[must_use]
pub fn cli_version_line() -> String {
    match codename() {
        Some(name) => format!("elda {ELDA_VERSION} ({name})"),
        None => format!("elda {ELDA_VERSION}"),
    }
}

#[must_use]
pub fn cli_long_version() -> String {
    version_details_human_lines().join("\n")
}

fn build_metadata() -> Value {
    json!({
        "profile": option_env!("ELDA_BUILD_PROFILE").unwrap_or("unknown"),
        "target": option_env!("ELDA_BUILD_TARGET").unwrap_or("unknown"),
        "date": option_env!("ELDA_BUILD_DATE").unwrap_or(""),
        "git_commit": option_env!("ELDA_GIT_COMMIT").unwrap_or(""),
    })
}

fn component_versions() -> Value {
    json!([
        { "name": "elda-cli", "version": ELDA_VERSION },
        { "name": "elda-core", "version": ELDA_VERSION },
        { "name": "elda-build", "version": ELDA_VERSION },
        { "name": "elda-install", "version": ELDA_VERSION },
        { "name": "elda-recipe", "version": ELDA_VERSION },
        { "name": "elda-repo", "version": ELDA_VERSION },
        { "name": "elda-db", "version": ELDA_VERSION },
        { "name": "elda-git", "version": ELDA_VERSION },
        { "name": "elda-types", "version": ELDA_VERSION },
        { "name": "elda-appimage", "version": ELDA_VERSION },
        { "name": "elda-populate", "version": ELDA_VERSION },
        { "name": "elda-linux", "version": ELDA_VERSION },
    ])
}

fn schema_versions() -> Value {
    json!({
        "sqlite_user_version": 3,
        "state_export_format_version": 1,
        "published_index_format_version": 1,
        "ci_workspace_format_version": 1,
        "cache_seed_manifest_schema_version": 1,
    })
}

fn json_str(details: &Value, key: &str) -> String {
    details
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned()
}
