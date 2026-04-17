use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use elda_build::{ManifestEntry, ManifestEntryKind};
use elda_db::{InstallationMode, StateLayout};

use crate::conffile::apply_conffile_entry;
use crate::journal::{BackupEntry, TransactionJournal};
use crate::{InstallConffileMode, InstallError};

#[derive(Debug, Default)]
pub(crate) struct AppliedEntry {
    pub(crate) created_paths: Vec<PathBuf>,
}

pub(crate) fn apply_entry(
    layout: &StateLayout,
    transaction_root: &Path,
    entry: &ManifestEntry,
    is_conffile: bool,
    conffile_mode: InstallConffileMode,
) -> Result<AppliedEntry, InstallError> {
    let relative = strip_leading_slash(&entry.path)?;
    let source_path = transaction_root.join(relative);
    let target_path = map_manifest_path(layout, &entry.path)?;

    match entry.kind {
        ManifestEntryKind::Directory => apply_directory(&target_path),
        ManifestEntryKind::RegularFile => apply_file(
            &source_path,
            &target_path,
            entry,
            is_conffile,
            conffile_mode,
        ),
        ManifestEntryKind::Symlink => apply_symlink(&target_path, entry),
    }
}

pub(crate) fn unpack_payload(payload_path: &Path, destination: &Path) -> Result<(), InstallError> {
    let file = fs::File::open(payload_path)?;
    let decoder = zstd::Decoder::new(file)?;
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(destination)?;

    Ok(())
}

pub(crate) fn map_manifest_path(
    layout: &StateLayout,
    manifest_path: &str,
) -> Result<PathBuf, InstallError> {
    let relative = strip_leading_slash(manifest_path)?;
    if layout.mode == InstallationMode::System {
        return Ok(layout.root_dir.join(relative));
    }

    let prefix_string = layout.prefix.to_string_lossy();
    let prefix_root = strip_leading_slash(&prefix_string)?;
    let mapped = if let Some(suffix) = manifest_path.strip_prefix("/usr/") {
        layout.root_dir.join(prefix_root).join(suffix)
    } else if manifest_path == "/usr" {
        layout.root_dir.join(prefix_root)
    } else if let Some(suffix) = manifest_path.strip_prefix("/etc/") {
        layout.root_dir.join(prefix_root).join("etc").join(suffix)
    } else if manifest_path == "/etc" {
        layout.root_dir.join(prefix_root).join("etc")
    } else if let Some(suffix) = manifest_path.strip_prefix("/var/") {
        layout.root_dir.join(prefix_root).join("var").join(suffix)
    } else if manifest_path == "/var" {
        layout.root_dir.join(prefix_root).join("var")
    } else {
        return Err(InstallError::Unsupported(format!(
            "prefix activation cannot map `{manifest_path}`"
        )));
    };

    Ok(mapped)
}

pub(crate) fn cleanup_paths(paths: &[PathBuf]) -> Result<(), InstallError> {
    let mut cleanup = paths.to_vec();
    cleanup.sort_by(|left, right| right.cmp(left));

    for path in cleanup {
        if path.is_symlink() || path.is_file() {
            if path.exists() || path.is_symlink() {
                fs::remove_file(path)?;
            }
            continue;
        }
        if path.is_dir() {
            let _ = fs::remove_dir(path);
        }
    }

    Ok(())
}

pub(crate) fn restore_backups(entries: &[BackupEntry]) -> Result<(), InstallError> {
    let mut restore = entries.to_vec();
    restore.sort_by(|left, right| left.original_path.cmp(&right.original_path));

    for entry in restore {
        match entry.path_kind.as_str() {
            "directory" => restore_directory(&entry)?,
            "file" => restore_file(&entry)?,
            "symlink" => restore_symlink(&entry)?,
            other => {
                return Err(InstallError::Unsupported(format!(
                    "cannot restore unsupported manifest kind `{other}`"
                )));
            }
        }
    }

    Ok(())
}

pub(crate) fn backup_file_for_restore(
    journal: &mut TransactionJournal,
    original_path: &Path,
    backup_path: PathBuf,
) -> Result<(), InstallError> {
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(original_path, &backup_path)?;
    journal.backup_entries.push(BackupEntry {
        original_path: original_path.to_path_buf(),
        backup_path: Some(backup_path),
        path_kind: "file".to_owned(),
        link_target: None,
        mode: fs::metadata(original_path)?.permissions().mode(),
    });

    Ok(())
}

pub(crate) fn cleanup_transaction_root(path: &Path) -> Result<(), InstallError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[must_use]
pub(crate) fn manifest_kind_label(kind: ManifestEntryKind) -> String {
    match kind {
        ManifestEntryKind::Directory => "directory",
        ManifestEntryKind::RegularFile => "file",
        ManifestEntryKind::Symlink => "symlink",
    }
    .to_owned()
}

pub(crate) fn strip_leading_slash(path: &str) -> Result<&str, InstallError> {
    path.strip_prefix('/')
        .ok_or_else(|| InstallError::Unsupported(format!("manifest path `{path}` is not absolute")))
}

pub(crate) fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

pub(crate) fn remove_existing_path(path: &Path) -> Result<(), InstallError> {
    if path.is_symlink() || path.is_file() {
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

pub(crate) fn backup_existing_path(
    journal: &mut TransactionJournal,
    original_path: &Path,
    backup_path: PathBuf,
) -> Result<(), InstallError> {
    let metadata = fs::symlink_metadata(original_path)?;
    if metadata.file_type().is_symlink() {
        journal.backup_entries.push(BackupEntry {
            original_path: original_path.to_path_buf(),
            backup_path: None,
            path_kind: "symlink".to_owned(),
            link_target: Some(fs::read_link(original_path)?.display().to_string()),
            mode: metadata.permissions().mode(),
        });
        return Ok(());
    }
    if metadata.is_dir() {
        return Err(InstallError::Unsupported(format!(
            "cannot preserve conffile sidecar at directory path `{}`",
            original_path.display()
        )));
    }

    backup_file_for_restore(journal, original_path, backup_path)
}

fn apply_directory(target_path: &Path) -> Result<AppliedEntry, InstallError> {
    fs::create_dir_all(target_path)?;
    Ok(AppliedEntry {
        created_paths: vec![target_path.to_path_buf()],
    })
}

fn apply_file(
    source_path: &Path,
    target_path: &Path,
    entry: &ManifestEntry,
    is_conffile: bool,
    conffile_mode: InstallConffileMode,
) -> Result<AppliedEntry, InstallError> {
    if is_conffile && (target_path.exists() || target_path.is_symlink()) {
        return apply_conffile_entry(source_path, target_path, entry, conffile_mode);
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_path, target_path)?;
    fs::set_permissions(target_path, fs::Permissions::from_mode(entry.mode))?;

    Ok(AppliedEntry {
        created_paths: vec![target_path.to_path_buf()],
    })
}

fn apply_symlink(target_path: &Path, entry: &ManifestEntry) -> Result<AppliedEntry, InstallError> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = entry.link_target.as_ref().ok_or_else(|| {
        InstallError::Unsupported(format!("symlink `{}` is missing a target", entry.path))
    })?;
    symlink(target, target_path)?;

    Ok(AppliedEntry {
        created_paths: vec![target_path.to_path_buf()],
    })
}

fn restore_directory(entry: &BackupEntry) -> Result<(), InstallError> {
    fs::create_dir_all(&entry.original_path)?;
    fs::set_permissions(&entry.original_path, fs::Permissions::from_mode(entry.mode))?;
    Ok(())
}

fn restore_file(entry: &BackupEntry) -> Result<(), InstallError> {
    if let Some(parent) = entry.original_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let backup_path = entry.backup_path.as_ref().ok_or_else(|| {
        InstallError::Journal(format!(
            "missing backup file for {}",
            entry.original_path.display()
        ))
    })?;
    fs::copy(backup_path, &entry.original_path)?;
    fs::set_permissions(&entry.original_path, fs::Permissions::from_mode(entry.mode))?;
    Ok(())
}

fn restore_symlink(entry: &BackupEntry) -> Result<(), InstallError> {
    if let Some(parent) = entry.original_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = entry.link_target.as_deref().ok_or_else(|| {
        InstallError::Journal(format!(
            "missing symlink target for {}",
            entry.original_path.display()
        ))
    })?;
    if entry.original_path.exists() || entry.original_path.is_symlink() {
        fs::remove_file(&entry.original_path)?;
    }
    symlink(target, &entry.original_path)?;
    Ok(())
}
