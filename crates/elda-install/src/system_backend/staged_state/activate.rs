use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use elda_db::{Database, InstallationMode, PackageFileRecord, StateLayout};

use super::StagedSystemState;
use super::util::{copy_existing_path, live_path};
use crate::InstallError;
use crate::fsops::{backup_existing_path, remove_existing_path, sidecar_path, strip_leading_slash};
use crate::journal::{BackupEntry, TransactionJournal};
use crate::system_backend::active_system_paths;

pub(crate) fn activate_staged_state(
    database: &Database,
    staged: &StagedSystemState,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(());
    }

    let current_paths = collect_current_live_paths(database)?;
    let stale_paths = current_paths
        .difference(&staged.tracked_paths)
        .cloned()
        .collect::<Vec<_>>();
    for path in stale_paths.iter().rev() {
        remove_live_path(database.layout(), path, journal)?;
        journal.persist(database.layout())?;
    }

    let mut desired_paths = staged.tracked_paths.iter().cloned().collect::<Vec<_>>();
    desired_paths.sort_by(|left, right| {
        path_kind_rank(&staged.stage_root, left)
            .cmp(&path_kind_rank(&staged.stage_root, right))
            .then_with(|| left.cmp(right))
    });
    for path in desired_paths {
        sync_live_path(database.layout(), &staged.stage_root, &path, journal)?;
        journal.persist(database.layout())?;
    }

    Ok(())
}

pub(crate) fn capture_active_system_state(
    database: &Database,
    state_id: &str,
) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(());
    }

    let stage_dir = database.layout().states_dir.join(state_id);
    let stage_root = stage_dir.join("root");
    if stage_dir.exists() {
        fs::remove_dir_all(&stage_dir)?;
    }
    fs::create_dir_all(&stage_root)?;

    for path in collect_current_live_paths(database)? {
        let live = live_path(&database.layout().root_dir, &path)?;
        if live.exists() || live.is_symlink() {
            copy_existing_path(&live, &live_path(&stage_root, &path)?)?;
        }
    }

    Ok(())
}

pub(crate) fn collect_stage_paths(stage_root: &Path) -> Result<BTreeSet<String>, InstallError> {
    let mut paths = BTreeSet::new();
    collect_stage_paths_recursive(stage_root, stage_root, &mut paths)?;
    Ok(paths)
}

fn collect_current_live_paths(database: &Database) -> Result<BTreeSet<String>, InstallError> {
    let mut paths = BTreeSet::new();
    for installed in database.list_installed_packages()? {
        for record in database.package_files(&installed.pkgname)? {
            if record.is_conffile {
                maybe_insert_sidecar_path(database.layout(), &mut paths, &record, ".eldanew")?;
            }
            paths.insert(record.path);
        }
    }
    paths.extend(active_system_paths(database.layout())?);

    Ok(paths)
}

fn maybe_insert_sidecar_path(
    layout: &StateLayout,
    paths: &mut BTreeSet<String>,
    record: &PackageFileRecord,
    suffix: &str,
) -> Result<(), InstallError> {
    let sidecar = sidecar_path(&live_path(&layout.root_dir, &record.path)?, suffix);
    if sidecar.exists() || sidecar.is_symlink() {
        paths.insert(format!("/{}", strip_leading_slash(&record.path)?).to_owned() + suffix);
    }
    Ok(())
}

fn collect_stage_paths_recursive(
    stage_root: &Path,
    current: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<(), InstallError> {
    for entry in fs::read_dir(current)? {
        let path = entry?.path();
        let relative = path
            .strip_prefix(stage_root)
            .map_err(|error| InstallError::Unsupported(error.to_string()))?;
        paths.insert(format!("/{}", relative.display()));
        if fs::symlink_metadata(&path)?.is_dir() {
            collect_stage_paths_recursive(stage_root, &path, paths)?;
        }
    }

    Ok(())
}

fn path_kind_rank(stage_root: &Path, path: &str) -> u8 {
    let Ok(source) = live_path(stage_root, path) else {
        return 1;
    };
    match fs::symlink_metadata(source) {
        Ok(metadata) if metadata.is_dir() => 0,
        _ => 1,
    }
}

fn remove_live_path(
    layout: &StateLayout,
    path: &str,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    let target = live_path(&layout.root_dir, path)?;
    if !target.exists() && !target.is_symlink() {
        return Ok(());
    }
    let metadata = fs::symlink_metadata(&target)?;
    if metadata.is_dir() {
        if fs::remove_dir(&target).is_ok() {
            journal.backup_entries.push(BackupEntry {
                original_path: target,
                backup_path: None,
                path_kind: "directory".to_owned(),
                link_target: None,
                mode: metadata.permissions().mode(),
            });
        }
        return Ok(());
    }

    backup_existing_path(
        journal,
        &target,
        next_backup_path(&journal.transaction_root, journal),
    )?;
    fs::remove_file(target)?;
    Ok(())
}

fn sync_live_path(
    layout: &StateLayout,
    stage_root: &Path,
    path: &str,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    let source = live_path(stage_root, path)?;
    let target = live_path(&layout.root_dir, path)?;
    let source_metadata = fs::symlink_metadata(&source)?;
    if source_metadata.is_dir() {
        if target.exists() && !fs::symlink_metadata(&target)?.is_dir() {
            backup_existing_path(
                journal,
                &target,
                next_backup_path(&journal.transaction_root, journal),
            )?;
            remove_existing_path(&target)?;
        }
        if !target.exists() {
            fs::create_dir_all(&target)?;
            journal.created_paths.push(target);
        }
        return Ok(());
    }
    if same_live_entry(&source, &target)? {
        return Ok(());
    }
    let switch_path = next_switch_path(&target, journal);
    materialize_switch_path(&source, &switch_path)?;
    journal.created_paths.push(switch_path.clone());
    if target.exists() || target.is_symlink() {
        backup_and_remove_existing(&target, journal)?;
    }
    fs::rename(&switch_path, &target)?;
    journal.created_paths.push(target);

    Ok(())
}

fn same_live_entry(source: &Path, target: &Path) -> Result<bool, InstallError> {
    if !target.exists() && !target.is_symlink() {
        return Ok(false);
    }
    let source_meta = fs::symlink_metadata(source)?;
    let target_meta = fs::symlink_metadata(target)?;
    if source_meta.file_type().is_symlink() != target_meta.file_type().is_symlink() {
        return Ok(false);
    }
    if source_meta.is_dir() && target_meta.is_dir() {
        return Ok(true);
    }
    if source_meta.file_type().is_symlink() {
        return Ok(fs::read_link(source)? == fs::read_link(target)?);
    }
    if !source_meta.is_file() || !target_meta.is_file() {
        return Ok(false);
    }
    Ok(fs::read(source)? == fs::read(target)?
        && source_meta.permissions().mode() == target_meta.permissions().mode())
}

fn next_backup_path(transaction_root: &Path, journal: &TransactionJournal) -> PathBuf {
    transaction_root.join(format!("activate-backup-{}", journal.backup_entries.len()))
}

fn next_switch_path(target: &Path, journal: &TransactionJournal) -> PathBuf {
    let mut path = target.as_os_str().to_os_string();
    path.push(format!(".elda-switch-{}", journal.created_paths.len()));
    PathBuf::from(path)
}

fn materialize_switch_path(source: &Path, switch_path: &Path) -> Result<(), InstallError> {
    copy_existing_path(source, switch_path)
}

fn backup_and_remove_existing(
    target: &Path,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    let metadata = fs::symlink_metadata(target)?;
    if metadata.is_dir() {
        if fs::remove_dir(target).is_ok() {
            journal.backup_entries.push(BackupEntry {
                original_path: target.to_path_buf(),
                backup_path: None,
                path_kind: "directory".to_owned(),
                link_target: None,
                mode: metadata.permissions().mode(),
            });
        }
        return Ok(());
    }

    backup_existing_path(
        journal,
        target,
        next_backup_path(&journal.transaction_root, journal),
    )?;
    remove_existing_path(target)?;
    Ok(())
}
