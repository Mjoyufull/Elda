use std::fs;
use std::path::Path;

use crate::error::CoreError;

use super::version::parse_foreign_version;
use super::{ForeignPackage, sort_packages};

pub(crate) fn read_xbps_packages(root: &Path) -> Result<Vec<ForeignPackage>, CoreError> {
    let db = root.join("var/db/xbps/pkgdb-0.38.plist");
    if !db.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(db)?;
    let mut packages = Vec::new();
    for block in content.split("<dict>").skip(1) {
        let Some(name) = plist_string(block, "pkgname") else {
            continue;
        };
        let raw_version = plist_string(block, "version").unwrap_or_else(|| "0-1".to_owned());
        packages.push(ForeignPackage {
            source_pm: "xbps".to_owned(),
            name,
            version: parse_foreign_version(&raw_version),
            arch: plist_string(block, "architecture"),
            files: Vec::new(),
            dependencies: Vec::new(),
            source_repo: plist_string(block, "repository"),
            source_channel: None,
        });
    }
    sort_packages(&mut packages);
    Ok(packages)
}

fn plist_string(block: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let start = block.find(&marker)? + marker.len();
    let rest = &block[start..];
    let open = rest.find("<string>")? + "<string>".len();
    let rest = &rest[open..];
    let close = rest.find("</string>")?;
    Some(rest[..close].trim().to_owned())
}
