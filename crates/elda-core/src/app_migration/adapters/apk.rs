use std::fs;
use std::path::Path;

use crate::error::CoreError;

use super::version::parse_foreign_version;
use super::{ForeignPackage, normalize_foreign_path, sort_packages};

pub(crate) fn read_apk_packages(root: &Path) -> Result<Vec<ForeignPackage>, CoreError> {
    let installed = root.join("lib/apk/db/installed");
    if !installed.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for paragraph in fs::read_to_string(installed)?.split("\n\n") {
        let mut name = None;
        let mut version = None;
        let mut arch = None;
        let mut dependencies = Vec::new();
        let mut files = Vec::new();
        let mut current_dir = String::new();
        for line in paragraph.lines() {
            if let Some(value) = line.strip_prefix("P:") {
                name = Some(value.to_owned());
            } else if let Some(value) = line.strip_prefix("V:") {
                version = Some(value.to_owned());
            } else if let Some(value) = line.strip_prefix("A:") {
                arch = Some(value.to_owned());
            } else if let Some(value) = line.strip_prefix("D:") {
                dependencies.extend(value.split_whitespace().map(ToOwned::to_owned));
            } else if let Some(value) = line.strip_prefix("F:") {
                current_dir = normalize_foreign_path(value);
            } else if let Some(value) = line.strip_prefix("R:") {
                files.push(join_foreign_path(&current_dir, value));
            }
        }
        if let (Some(name), Some(raw_version)) = (name, version) {
            packages.push(ForeignPackage {
                source_pm: "apk".to_owned(),
                name,
                version: parse_foreign_version(&raw_version),
                arch,
                files,
                dependencies,
                source_repo: None,
                source_channel: None,
            });
        }
    }
    sort_packages(&mut packages);
    Ok(packages)
}

fn join_foreign_path(dir: &str, file: &str) -> String {
    let normalized_file = file.trim().trim_start_matches('/');
    if dir == "/" {
        format!("/{normalized_file}")
    } else {
        format!("{}/{normalized_file}", dir.trim_end_matches('/'))
    }
}
