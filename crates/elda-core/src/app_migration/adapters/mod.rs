mod apk;
mod apt;
mod pacman;
mod portage;
mod version;
mod xbps;

use std::path::Path;

use crate::error::CoreError;
pub(crate) use version::ForeignVersion;

#[derive(Debug, Clone)]
pub(crate) struct ForeignPackage {
    pub(crate) source_pm: String,
    pub(crate) name: String,
    pub(crate) version: ForeignVersion,
    pub(crate) arch: Option<String>,
    pub(crate) files: Vec<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) source_repo: Option<String>,
    pub(crate) source_channel: Option<String>,
}

pub(crate) fn read_foreign_package(
    root: &Path,
    source_pm: &str,
    package_name: &str,
) -> Result<Option<ForeignPackage>, CoreError> {
    Ok(read_foreign_packages(root, source_pm)?
        .into_iter()
        .find(|package| package.name == package_name))
}

pub(crate) fn read_foreign_packages(
    root: &Path,
    source_pm: &str,
) -> Result<Vec<ForeignPackage>, CoreError> {
    match source_pm {
        "pacman" => pacman::read_pacman_packages(root),
        "apt" => apt::read_apt_packages(root),
        "apk" => apk::read_apk_packages(root),
        "portage" => portage::read_portage_packages(root),
        "xbps" => xbps::read_xbps_packages(root),
        other => Err(CoreError::Operator(format!(
            "unsupported migration adapter `{other}`; expected pacman, apt, apk, xbps, or portage"
        ))),
    }
}

pub(crate) fn normalize_foreign_path(path: &str) -> String {
    let trimmed = path.trim().trim_start_matches("./");
    if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    }
}

pub(crate) fn sort_packages(packages: &mut [ForeignPackage]) {
    packages.sort_by(|left, right| left.name.cmp(&right.name));
}
