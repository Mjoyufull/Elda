use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::CoreError;

use super::version::parse_foreign_version;
use super::{ForeignPackage, normalize_foreign_path, sort_packages};

pub(crate) fn read_pacman_packages(root: &Path) -> Result<Vec<ForeignPackage>, CoreError> {
    let local = root.join("var/lib/pacman/local");
    if !local.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for entry in fs::read_dir(local)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let desc = entry.path().join("desc");
        if !desc.exists() {
            continue;
        }
        let parsed = parse_pacman_desc(&fs::read_to_string(desc)?)?;
        packages.push(ForeignPackage {
            source_pm: "pacman".to_owned(),
            name: required_field(&parsed, "NAME")?,
            version: parse_foreign_version(&required_field(&parsed, "VERSION")?),
            arch: parsed
                .get("ARCH")
                .and_then(|values| values.first())
                .cloned(),
            files: parsed
                .get("FILES")
                .into_iter()
                .flatten()
                .map(|path| normalize_foreign_path(path))
                .collect(),
            dependencies: parsed.get("DEPENDS").cloned().unwrap_or_default(),
            source_repo: parsed
                .get("REPOSITORY")
                .and_then(|values| values.first())
                .cloned(),
            source_channel: None,
        });
    }
    sort_packages(&mut packages);
    Ok(packages)
}

fn parse_pacman_desc(content: &str) -> Result<BTreeMap<String, Vec<String>>, CoreError> {
    let mut map = BTreeMap::new();
    let mut current_key: Option<String> = None;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') && trimmed.ends_with('%') && trimmed.len() > 2 {
            let key = trimmed.trim_matches('%').to_owned();
            map.entry(key.clone()).or_insert_with(Vec::new);
            current_key = Some(key);
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        let key = current_key.as_ref().ok_or_else(|| {
            CoreError::Operator("invalid pacman desc: value before key".to_owned())
        })?;
        map.entry(key.clone())
            .or_insert_with(Vec::new)
            .push(trimmed.to_owned());
    }
    Ok(map)
}

fn required_field(map: &BTreeMap<String, Vec<String>>, key: &str) -> Result<String, CoreError> {
    map.get(key)
        .and_then(|values| values.first())
        .cloned()
        .ok_or_else(|| CoreError::Operator(format!("foreign package record missing `{key}`")))
}
