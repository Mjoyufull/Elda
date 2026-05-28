use std::path::{Path, PathBuf};

use elda_db::StateLayout;

use crate::InstallError;

pub(super) fn resolve_symlink_target(layout: &StateLayout, target: &str) -> PathBuf {
    if layout.root_dir == Path::new("/") {
        PathBuf::from(target)
    } else {
        layout
            .root_dir
            .join(strip_leading_slash(target).unwrap_or(target))
    }
}

pub(super) fn package_metadata_dir(layout: &StateLayout) -> PathBuf {
    layout.state_dir.join("system-backend").join("packages")
}

pub(super) fn package_metadata_path(layout: &StateLayout, package_name: &str) -> PathBuf {
    package_metadata_dir(layout).join(format!("{package_name}.json"))
}

pub(super) fn alternative_registry_path(layout: &StateLayout) -> PathBuf {
    layout
        .state_dir
        .join("system-backend")
        .join("alternatives.json")
}

pub(super) fn strip_leading_slash(path: &str) -> Result<&str, InstallError> {
    path.strip_prefix('/')
        .ok_or_else(|| InstallError::Unsupported(format!("expected absolute path `{path}`")))
}
