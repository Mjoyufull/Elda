use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use elda_build::{DeclarativeAsset, ManifestEntry, ManifestEntryKind};

use crate::InstallError;
use crate::fsops::{remove_existing_path, sidecar_path, strip_leading_slash};

pub(super) fn copy_packaged_entry(
    unpack_root: &Path,
    target: &Path,
    entry: &ManifestEntry,
) -> Result<(), InstallError> {
    let source = unpack_root.join(strip_leading_slash(&entry.path)?);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    match entry.kind {
        ManifestEntryKind::RegularFile => {
            fs::copy(source, target)?;
            fs::set_permissions(target, fs::Permissions::from_mode(entry.mode))?;
        }
        ManifestEntryKind::Symlink => {
            symlink(
                entry.link_target.as_deref().ok_or_else(|| {
                    InstallError::Unsupported(format!(
                        "symlink `{}` is missing a target",
                        entry.path
                    ))
                })?,
                target,
            )?;
        }
        ManifestEntryKind::Directory => {}
    }

    Ok(())
}

pub(super) fn copy_existing_path(source: &Path, target: &Path) -> Result<(), InstallError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        if target.exists() || target.is_symlink() {
            remove_existing_path(target)?;
        }
        symlink(fs::read_link(source)?, target)?;
        return Ok(());
    }
    if metadata.is_dir() {
        fs::create_dir_all(target)?;
        fs::set_permissions(
            target,
            fs::Permissions::from_mode(metadata.permissions().mode()),
        )?;
        return Ok(());
    }

    fs::copy(source, target)?;
    fs::set_permissions(
        target,
        fs::Permissions::from_mode(metadata.permissions().mode()),
    )?;
    Ok(())
}

pub(super) fn copy_sidecar_if_present(
    source: &Path,
    target: &Path,
    suffix: &str,
) -> Result<(), InstallError> {
    let sidecar = sidecar_path(source, suffix);
    if sidecar.exists() || sidecar.is_symlink() {
        copy_existing_path(&sidecar, &sidecar_path(target, suffix))?;
    }
    Ok(())
}

pub(super) fn write_asset(
    stage_root: &Path,
    asset: Option<&DeclarativeAsset>,
) -> Result<(), InstallError> {
    let Some(asset) = asset else {
        return Ok(());
    };
    let target = live_path(stage_root, &asset.path)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, &asset.content)?;
    Ok(())
}

pub(super) fn live_path(root: &Path, manifest_path: &str) -> Result<PathBuf, InstallError> {
    Ok(root.join(strip_leading_slash(manifest_path)?))
}

pub(super) fn resolve_absolute_target(root: &Path, target: &str) -> PathBuf {
    if root == Path::new("/") {
        PathBuf::from(target)
    } else {
        root.join(strip_leading_slash(target).unwrap_or(target))
    }
}
