use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::CoreError;

use super::version::parse_foreign_version;
use super::{ForeignPackage, sort_packages};

pub(crate) fn read_apt_packages(root: &Path) -> Result<Vec<ForeignPackage>, CoreError> {
    let status_path = root.join("var/lib/dpkg/status");
    if !status_path.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for paragraph in split_debian_paragraphs(&fs::read_to_string(status_path)?) {
        let fields = parse_debian_fields(paragraph);
        if fields
            .get("Status")
            .is_none_or(|status| !status.contains(" installed"))
        {
            continue;
        }
        let name = required_debian_field(&fields, "Package")?;
        packages.push(ForeignPackage {
            source_pm: "apt".to_owned(),
            name: name.clone(),
            version: parse_foreign_version(&required_debian_field(&fields, "Version")?),
            arch: fields.get("Architecture").cloned(),
            files: read_dpkg_file_list(root, &name)?,
            dependencies: parse_debian_dependencies(fields.get("Depends")),
            source_repo: None,
            source_channel: None,
        });
    }
    sort_packages(&mut packages);
    Ok(packages)
}

fn read_dpkg_file_list(root: &Path, package_name: &str) -> Result<Vec<String>, CoreError> {
    let path = root
        .join("var/lib/dpkg/info")
        .join(format!("{package_name}.list"));
    if !path.exists() {
        return Ok(Vec::new());
    }
    Ok(fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/') && *line != "/.")
        .map(ToOwned::to_owned)
        .collect())
}

fn split_debian_paragraphs(content: &str) -> impl Iterator<Item = &str> {
    content
        .split("\n\n")
        .filter(|paragraph| !paragraph.trim().is_empty())
}

fn parse_debian_fields(paragraph: &str) -> BTreeMap<String, String> {
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut current_key: Option<String> = None;
    for line in paragraph.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(key) = &current_key
                && let Some(value) = fields.get_mut(key)
            {
                value.push('\n');
                value.push_str(line.trim());
            }
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_owned();
        fields.insert(key.clone(), value.trim().to_owned());
        current_key = Some(key);
    }
    fields
}

fn required_debian_field(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<String, CoreError> {
    fields
        .get(key)
        .cloned()
        .ok_or_else(|| CoreError::Operator(format!("dpkg status record missing `{key}`")))
}

fn parse_debian_dependencies(value: Option<&String>) -> Vec<String> {
    value
        .map(|text| {
            text.split(',')
                .filter_map(|entry| entry.split('|').next())
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| entry.replace(" (", "").replace(')', ""))
                .collect()
        })
        .unwrap_or_default()
}
