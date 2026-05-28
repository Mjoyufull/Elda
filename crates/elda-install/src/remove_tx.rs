use std::fs;

use elda_db::{Database, InstallationMode};

use crate::archive_state::archive_current_state;
use crate::conffile::remove_conffile_entry;
use crate::fsops::{backup_file_for_restore, cleanup_transaction_root, map_manifest_path};
use crate::journal::{JournalState, TransactionJournal, ensure_no_pending_journals};
use crate::snapshot::{post_activation_snapshot, pre_activation_snapshot};
use crate::system_backend::{
    activate_staged_state, next_state_prefix, prepare_staged_remove,
    reconcile_system_state_after_remove, run_remove_triggers,
};
use crate::{InstallError, MutationPolicy, RemoveConffileMode, RemoveReport, next_state_id};

pub fn remove_package(
    database: &Database,
    package_name: &str,
    policy: &MutationPolicy,
) -> Result<RemoveReport, InstallError> {
    remove_package_internal(
        database,
        package_name,
        true,
        true,
        RemoveConffileMode::PreserveAsSave,
        policy,
    )
}

pub fn remove_package_purge_conffiles(
    database: &Database,
    package_name: &str,
    policy: &MutationPolicy,
) -> Result<RemoveReport, InstallError> {
    remove_package_internal(
        database,
        package_name,
        true,
        true,
        RemoveConffileMode::Purge,
        policy,
    )
}

pub fn remove_package_for_upgrade(
    database: &Database,
    package_name: &str,
    policy: &MutationPolicy,
) -> Result<RemoveReport, InstallError> {
    remove_package_internal(
        database,
        package_name,
        false,
        true,
        RemoveConffileMode::PreserveInPlaceForUpgrade,
        policy,
    )
}

pub(crate) fn remove_package_internal(
    database: &Database,
    package_name: &str,
    archive_state: bool,
    take_lock: bool,
    conffile_mode: RemoveConffileMode,
    policy: &MutationPolicy,
) -> Result<RemoveReport, InstallError> {
    if take_lock {
        database.bootstrap()?;
    } else {
        database.layout().ensure_exists()?;
    }
    let _lock = if take_lock {
        Some(database.acquire_mutation_lock()?)
    } else {
        None
    };
    ensure_no_pending_journals(database.layout())?;
    ensure_package_installed(database, package_name)?;

    let state_id = next_state_id(next_state_prefix(database.layout()));
    let transaction_root = database
        .layout()
        .tmp_dir
        .join("transactions")
        .join(format!("remove-{state_id}"));
    let mut journal = begin_remove_journal(database, package_name, &state_id, &transaction_root)?;
    if let Some(snapshot) = pre_activation_snapshot(database.layout(), &state_id, policy) {
        journal.snapshots.push(snapshot);
        journal.persist(database.layout())?;
    }
    let files = apply_remove_state(
        database,
        package_name,
        &state_id,
        &transaction_root,
        &mut journal,
        conffile_mode,
    )?;

    remove_manifest(database, package_name, &transaction_root, &mut journal)?;
    database.remove_package(package_name)?;
    reconcile_system_state_after_remove(database, package_name)?;
    let removed_paths = files
        .iter()
        .map(|record| record.path.clone())
        .collect::<Vec<_>>();
    run_remove_triggers(database, &removed_paths)?;
    database.set_current_state(&state_id)?;
    if let Some(snapshot) = post_activation_snapshot(database.layout(), &state_id, policy) {
        journal.snapshots.push(snapshot);
        journal.persist(database.layout())?;
    }
    if archive_state {
        archive_current_state(database, &state_id, &journal.snapshots)?;
    }
    journal.state = JournalState::DbCommitted;
    journal.persist(database.layout())?;
    cleanup_transaction_root(&transaction_root)?;
    let snapshots = journal.snapshots.clone();
    journal.remove(database.layout())?;

    Ok(RemoveReport {
        package_name: package_name.to_owned(),
        removed_paths: files.len(),
        snapshots,
    })
}
fn ensure_package_installed(database: &Database, package_name: &str) -> Result<(), InstallError> {
    if database.installed_package(package_name)?.is_none() {
        return Err(InstallError::NotInstalled(package_name.to_owned()));
    }
    Ok(())
}

fn begin_remove_journal(
    database: &Database,
    package_name: &str,
    state_id: &str,
    transaction_root: &std::path::Path,
) -> Result<TransactionJournal, InstallError> {
    let mut journal = TransactionJournal::new_remove(
        format!("txn-remove-{state_id}"),
        package_name.to_owned(),
        transaction_root.to_path_buf(),
    );
    journal.state_id = Some(state_id.to_owned());
    journal.persist(database.layout())?;
    fs::create_dir_all(transaction_root)?;
    Ok(journal)
}

fn remove_package_files(
    database: &Database,
    package_name: &str,
    transaction_root: &std::path::Path,
    journal: &mut TransactionJournal,
    conffile_mode: RemoveConffileMode,
) -> Result<Vec<elda_db::PackageFileRecord>, InstallError> {
    let mut files = database.package_files(package_name)?;
    files.sort_by(|left, right| right.path.cmp(&left.path));

    for (index, entry) in files.iter().enumerate() {
        let target_path = map_manifest_path(database.layout(), &entry.path)?;
        if !target_path.exists() && !target_path.is_symlink() {
            continue;
        }

        if entry.is_conffile
            && entry.path_kind == "file"
            && let Some(removed) = remove_conffile_entry(
                journal,
                transaction_root,
                &target_path,
                entry,
                index,
                conffile_mode,
            )?
        {
            if removed {
                journal.persist(database.layout())?;
            }
            continue;
        }

        remove_non_conffile_entry(journal, &target_path, entry, index, transaction_root)?;
        journal.persist(database.layout())?;
    }

    Ok(files)
}

fn apply_remove_state(
    database: &Database,
    package_name: &str,
    state_id: &str,
    transaction_root: &std::path::Path,
    journal: &mut TransactionJournal,
    conffile_mode: RemoveConffileMode,
) -> Result<Vec<elda_db::PackageFileRecord>, InstallError> {
    let files = database.package_files(package_name)?;
    if database.layout().mode == InstallationMode::System {
        let staged = prepare_staged_remove(
            database,
            package_name,
            state_id,
            transaction_root,
            journal,
            conffile_mode,
        )?;
        activate_staged_state(database, &staged, journal)?;
        journal.state = JournalState::FilesApplied;
        journal.persist(database.layout())?;
        return Ok(files);
    }

    remove_package_files(
        database,
        package_name,
        transaction_root,
        journal,
        conffile_mode,
    )
}

fn remove_non_conffile_entry(
    journal: &mut TransactionJournal,
    target_path: &std::path::Path,
    entry: &elda_db::PackageFileRecord,
    index: usize,
    transaction_root: &std::path::Path,
) -> Result<(), InstallError> {
    match entry.path_kind.as_str() {
        "directory" => {
            if fs::remove_dir(target_path).is_ok() {
                journal.backup_entries.push(crate::journal::BackupEntry {
                    original_path: target_path.to_path_buf(),
                    backup_path: None,
                    path_kind: entry.path_kind.clone(),
                    link_target: None,
                    mode: entry.mode,
                });
            }
        }
        "file" => {
            let backup_path = transaction_root.join(format!("backup-{index}"));
            backup_file_for_restore(journal, target_path, backup_path)?;
            fs::remove_file(target_path)?;
        }
        "symlink" => {
            let link_target = fs::read_link(target_path)?.display().to_string();
            journal.backup_entries.push(crate::journal::BackupEntry {
                original_path: target_path.to_path_buf(),
                backup_path: None,
                path_kind: entry.path_kind.clone(),
                link_target: Some(link_target),
                mode: entry.mode,
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

fn remove_manifest(
    database: &Database,
    package_name: &str,
    transaction_root: &std::path::Path,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    let manifest_target = database
        .layout()
        .manifests_dir
        .join(format!("{}.manifest", package_name));
    if manifest_target.exists() {
        backup_file_for_restore(
            journal,
            &manifest_target,
            transaction_root.join("package.manifest"),
        )?;
        fs::remove_file(&manifest_target)?;
    }
    journal.state = JournalState::FilesApplied;
    journal.persist(database.layout())?;
    Ok(())
}
