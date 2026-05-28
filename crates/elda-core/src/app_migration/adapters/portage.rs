use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CoreError;

use super::version::parse_foreign_version;
use super::{ForeignPackage, sort_packages};

pub(crate) fn read_portage_packages(root: &Path) -> Result<Vec<ForeignPackage>, CoreError> {
    let db = root.join("var/db/pkg");
    if !db.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for category in fs::read_dir(db)? {
        let category = category?;
        if !category.file_type()?.is_dir() {
            continue;
        }
        for package_dir in fs::read_dir(category.path())? {
            let package_dir = package_dir?;
            if !package_dir.file_type()?.is_dir() {
                continue;
            }
            let path = package_dir.path();
            let package_name = read_optional_trimmed(path.join("PN"))?
                .unwrap_or_else(|| infer_portage_name(&package_dir.file_name().to_string_lossy()));
            let raw_version = read_optional_trimmed(path.join("PVR"))?
                .or_else(|| read_optional_trimmed(path.join("PV")).ok().flatten())
                .unwrap_or_else(|| {
                    infer_portage_version(&package_dir.file_name().to_string_lossy(), &package_name)
                });
            packages.push(ForeignPackage {
                source_pm: "portage".to_owned(),
                name: package_name,
                version: parse_foreign_version(&raw_version),
                arch: None,
                files: read_portage_contents(path.join("CONTENTS"))?,
                dependencies: parse_portage_dependencies(read_optional_trimmed(
                    path.join("RDEPEND"),
                )?),
                source_repo: Some(category.file_name().to_string_lossy().into_owned()),
                source_channel: None,
            });
        }
    }
    sort_packages(&mut packages);
    Ok(packages)
}

fn read_optional_trimmed(path: PathBuf) -> Result<Option<String>, CoreError> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(path)?.trim().to_owned()))
}

fn read_portage_contents(path: PathBuf) -> Result<Vec<String>, CoreError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(fs::read_to_string(path)?
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter(|path| path.starts_with('/'))
        .map(ToOwned::to_owned)
        .collect())
}

fn parse_portage_dependencies(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .filter(|token| token.chars().any(|c| c.is_ascii_alphabetic()))
        .map(|token| token.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']')))
        .filter(|token| !token.is_empty() && !token.starts_with('!'))
        .map(ToOwned::to_owned)
        .collect()
}

fn infer_portage_name(package_dir: &str) -> String {
    package_dir
        .rsplit_once('-')
        .map(|(name, _)| name.to_owned())
        .unwrap_or_else(|| package_dir.to_owned())
}

fn infer_portage_version(package_dir: &str, package_name: &str) -> String {
    package_dir
        .strip_prefix(package_name)
        .and_then(|tail| tail.strip_prefix('-'))
        .unwrap_or("0-1")
        .to_owned()
}
