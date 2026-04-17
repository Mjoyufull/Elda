use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use sha2::{Digest, Sha256};

use elda_build::ManifestEntry;
use elda_db::PackageFileRecord;

use crate::fsops::{
    AppliedEntry, backup_existing_path, backup_file_for_restore, remove_existing_path, sidecar_path,
};
use crate::journal::{BackupEntry, TransactionJournal};
use crate::{InstallConffileMode, InstallError, RemoveConffileMode};

pub(crate) fn apply_conffile_entry(
    source_path: &Path,
    target_path: &Path,
    entry: &ManifestEntry,
    conffile_mode: InstallConffileMode,
) -> Result<AppliedEntry, InstallError> {
    match conffile_mode {
        InstallConffileMode::FirstOwnership => {
            install_conffile_sidecar(source_path, target_path, entry)
        }
        InstallConffileMode::Upgrade => apply_upgraded_conffile(source_path, target_path, entry),
    }
}

pub(crate) fn remove_conffile_entry(
    journal: &mut TransactionJournal,
    transaction_root: &Path,
    target_path: &Path,
    entry: &PackageFileRecord,
    index: usize,
    conffile_mode: RemoveConffileMode,
) -> Result<Option<bool>, InstallError> {
    let modified = !record_matches_live_file(entry, target_path)?;
    match conffile_mode {
        RemoveConffileMode::PreserveInPlaceForUpgrade if modified => Ok(Some(false)),
        RemoveConffileMode::PreserveAsSave if modified => {
            preserve_conffile_as_save(journal, transaction_root, target_path, entry, index)?;
            Ok(Some(true))
        }
        RemoveConffileMode::PreserveInPlaceForUpgrade
        | RemoveConffileMode::PreserveAsSave
        | RemoveConffileMode::Purge => {
            remove_record_path(
                journal,
                transaction_root,
                target_path,
                &entry.path_kind,
                entry.mode,
                index,
            )?;
            Ok(Some(true))
        }
    }
}

pub(crate) fn preserve_conffile_as_save(
    journal: &mut TransactionJournal,
    transaction_root: &Path,
    target_path: &Path,
    entry: &PackageFileRecord,
    index: usize,
) -> Result<(), InstallError> {
    let save_path = sidecar_path(target_path, ".eldasave");
    let backup_path = transaction_root.join(format!("backup-{index}"));
    backup_file_for_restore(journal, target_path, backup_path)?;
    if save_path.exists() || save_path.is_symlink() {
        backup_existing_path(
            journal,
            &save_path,
            transaction_root.join(format!("backup-save-{index}")),
        )?;
    } else {
        journal.created_paths.push(save_path.clone());
    }
    fs::copy(target_path, &save_path)?;
    fs::set_permissions(&save_path, fs::Permissions::from_mode(entry.mode))?;
    fs::remove_file(target_path)?;
    Ok(())
}

pub(crate) fn record_matches_live_file(
    entry: &PackageFileRecord,
    target_path: &Path,
) -> Result<bool, InstallError> {
    if entry.path_kind != "file" {
        return Ok(false);
    }
    let expected = entry.sha256.as_deref().ok_or_else(|| {
        InstallError::Unsupported(format!(
            "managed file `{}` is missing its recorded checksum",
            entry.path
        ))
    })?;
    file_matches_sha256(target_path, expected)
}

pub(crate) fn file_matches_sha256(path: &Path, expected: &str) -> Result<bool, InstallError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Ok(false);
    }
    Ok(sha256_file(path)? == expected)
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, InstallError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn apply_upgraded_conffile(
    source_path: &Path,
    target_path: &Path,
    entry: &ManifestEntry,
) -> Result<AppliedEntry, InstallError> {
    let expected = entry.sha256.as_deref().ok_or_else(|| {
        InstallError::Unsupported(format!(
            "conffile `{}` is missing a packaged checksum",
            entry.path
        ))
    })?;
    if file_matches_sha256(target_path, expected)? {
        return Ok(AppliedEntry::default());
    }

    install_conffile_sidecar(source_path, target_path, entry)
}

fn install_conffile_sidecar(
    source_path: &Path,
    target_path: &Path,
    entry: &ManifestEntry,
) -> Result<AppliedEntry, InstallError> {
    let destination = sidecar_path(target_path, ".eldanew");
    remove_existing_path(&destination)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_path, &destination)?;
    fs::set_permissions(&destination, fs::Permissions::from_mode(entry.mode))?;

    Ok(AppliedEntry {
        created_paths: vec![destination],
    })
}

fn remove_record_path(
    journal: &mut TransactionJournal,
    transaction_root: &Path,
    target_path: &Path,
    path_kind: &str,
    mode: u32,
    index: usize,
) -> Result<(), InstallError> {
    match path_kind {
        "directory" => {
            if fs::remove_dir(target_path).is_ok() {
                journal.backup_entries.push(BackupEntry {
                    original_path: target_path.to_path_buf(),
                    backup_path: None,
                    path_kind: path_kind.to_owned(),
                    link_target: None,
                    mode,
                });
            }
        }
        "file" => {
            backup_file_for_restore(
                journal,
                target_path,
                transaction_root.join(format!("backup-{index}")),
            )?;
            fs::remove_file(target_path)?;
        }
        "symlink" => {
            let link_target = fs::read_link(target_path)?.display().to_string();
            journal.backup_entries.push(BackupEntry {
                original_path: target_path.to_path_buf(),
                backup_path: None,
                path_kind: path_kind.to_owned(),
                link_target: Some(link_target),
                mode,
            });
            fs::remove_file(target_path)?;
        }
        other => {
            return Err(InstallError::Unsupported(format!(
                "cannot remove unsupported manifest kind `{other}`"
            )));
        }
    }

    Ok(())
}
