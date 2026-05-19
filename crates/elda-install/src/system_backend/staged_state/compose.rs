use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use elda_build::{BuiltPackage, ManifestEntry, ManifestEntryKind, SystemPackageMetadata};
use elda_db::{Database, StateLayout};

use super::StagedSystemState;
use super::activate::collect_stage_paths;
use super::util::{
    copy_existing_path, copy_packaged_entry, copy_sidecar_if_present, live_path,
    resolve_absolute_target, write_asset,
};
use crate::InstallError;
use crate::cached_archive::built_package_from_installed;
use crate::conffile::{apply_conffile_entry, record_matches_live_file};
use crate::fsops::{sidecar_path, unpack_payload};
use crate::install_tx::InstallPathDecisions;
use crate::journal::TransactionJournal;
use crate::system_backend::{active_provider_families, provider_assets};
use crate::{InstallConffileMode, RemoveConffileMode};

pub(crate) fn prepare_staged_install(
    database: &Database,
    package: &BuiltPackage,
    state_id: &str,
    transaction_root: &Path,
    journal: &mut TransactionJournal,
    conffile_mode: InstallConffileMode,
    path_decisions: &InstallPathDecisions,
) -> Result<StagedSystemState, InstallError> {
    let stage_root = create_stage_root(database.layout(), state_id, journal)?;
    let mut system_packages =
        materialize_current_packages(database, transaction_root, &stage_root)?;
    materialize_incoming_package(
        database.layout(),
        transaction_root,
        &stage_root,
        package,
        conffile_mode,
        path_decisions,
    )?;
    system_packages.insert(
        package.package_name.clone(),
        package.system_metadata.clone(),
    );
    materialize_system_assets(
        database.layout(),
        &stage_root,
        &database.layout().root_dir,
        &system_packages,
    )?;

    Ok(StagedSystemState {
        tracked_paths: collect_stage_paths(&stage_root)?,
        stage_root,
    })
}

pub(crate) fn prepare_staged_remove(
    database: &Database,
    package_name: &str,
    state_id: &str,
    transaction_root: &Path,
    journal: &mut TransactionJournal,
    conffile_mode: RemoveConffileMode,
) -> Result<StagedSystemState, InstallError> {
    let stage_root = create_stage_root(database.layout(), state_id, journal)?;
    let mut system_packages = BTreeMap::new();
    for installed in database.list_installed_packages()? {
        if installed.pkgname == package_name {
            continue;
        }
        let package = built_package_from_installed(database, &installed.pkgname)?;
        materialize_current_package(database.layout(), transaction_root, &stage_root, &package)?;
        system_packages.insert(
            package.package_name.clone(),
            package.system_metadata.clone(),
        );
    }
    materialize_removed_conffiles(database, &stage_root, package_name, conffile_mode)?;
    materialize_system_assets(
        database.layout(),
        &stage_root,
        &database.layout().root_dir,
        &system_packages,
    )?;

    Ok(StagedSystemState {
        tracked_paths: collect_stage_paths(&stage_root)?,
        stage_root,
    })
}

fn create_stage_root(
    layout: &StateLayout,
    state_id: &str,
    journal: &mut TransactionJournal,
) -> Result<PathBuf, InstallError> {
    let stage_dir = layout.states_dir.join(state_id);
    let stage_root = stage_dir.join("root");
    if stage_dir.exists() {
        fs::remove_dir_all(&stage_dir)?;
    }
    fs::create_dir_all(&stage_root)?;
    journal.created_paths.push(stage_dir);

    Ok(stage_root)
}

fn materialize_current_packages(
    database: &Database,
    transaction_root: &Path,
    stage_root: &Path,
) -> Result<BTreeMap<String, SystemPackageMetadata>, InstallError> {
    let mut packages = BTreeMap::new();
    for installed in database.list_installed_packages()? {
        let package = built_package_from_installed(database, &installed.pkgname)?;
        materialize_current_package(database.layout(), transaction_root, stage_root, &package)?;
        packages.insert(
            package.package_name.clone(),
            package.system_metadata.clone(),
        );
    }

    Ok(packages)
}

fn materialize_current_package(
    layout: &StateLayout,
    transaction_root: &Path,
    stage_root: &Path,
    package: &BuiltPackage,
) -> Result<(), InstallError> {
    let unpack_root = transaction_root
        .join("staged-current")
        .join(&package.package_name);
    if unpack_root.exists() {
        fs::remove_dir_all(&unpack_root)?;
    }
    fs::create_dir_all(&unpack_root)?;
    unpack_payload(&package.payload_path, &unpack_root)?;

    for entry in &package.manifest.entries {
        let live_target = live_path(&layout.root_dir, &entry.path)?;
        let stage_target = live_path(stage_root, &entry.path)?;
        if entry.kind == ManifestEntryKind::Directory {
            fs::create_dir_all(&stage_target)?;
            continue;
        }
        if package.conffiles.iter().any(|path| path == &entry.path)
            && (live_target.exists() || live_target.is_symlink())
        {
            copy_existing_path(&live_target, &stage_target)?;
            copy_sidecar_if_present(&live_target, &stage_target, ".eldanew")?;
            continue;
        }

        copy_packaged_entry(&unpack_root, &stage_target, entry)?;
    }

    Ok(())
}

fn materialize_incoming_package(
    layout: &StateLayout,
    transaction_root: &Path,
    stage_root: &Path,
    package: &BuiltPackage,
    conffile_mode: InstallConffileMode,
    path_decisions: &InstallPathDecisions,
) -> Result<(), InstallError> {
    let unpack_root = transaction_root
        .join("staged-incoming")
        .join(&package.package_name);
    if unpack_root.exists() {
        fs::remove_dir_all(&unpack_root)?;
    }
    fs::create_dir_all(&unpack_root)?;
    unpack_payload(&package.payload_path, &unpack_root)?;

    for entry in &package.manifest.entries {
        if path_decisions.should_skip_entry(&entry.path) {
            continue;
        }
        let stage_target = live_path(stage_root, &entry.path)?;
        let live_target = live_path(&layout.root_dir, &entry.path)?;
        if package.conffiles.iter().any(|path| path == &entry.path)
            && !stage_target.exists()
            && !stage_target.is_symlink()
            && (live_target.exists() || live_target.is_symlink())
        {
            copy_existing_path(&live_target, &stage_target)?;
        }
        apply_staged_entry(&unpack_root, stage_root, entry, package, conffile_mode)?;
    }

    Ok(())
}

fn materialize_removed_conffiles(
    database: &Database,
    stage_root: &Path,
    package_name: &str,
    conffile_mode: RemoveConffileMode,
) -> Result<(), InstallError> {
    let removed_conffiles = database
        .package_files(package_name)?
        .into_iter()
        .filter(|record| record.is_conffile && record.path_kind == "file")
        .collect::<Vec<_>>();
    for record in removed_conffiles {
        let live_target = live_path(&database.layout().root_dir, &record.path)?;
        if !live_target.exists() && !live_target.is_symlink() {
            continue;
        }
        let modified = !record_matches_live_file(&record, &live_target)?;
        match conffile_mode {
            RemoveConffileMode::PreserveAsSave if modified => {
                copy_existing_path(
                    &live_target,
                    &sidecar_path(&live_path(stage_root, &record.path)?, ".eldasave"),
                )?;
            }
            RemoveConffileMode::PreserveInPlaceForUpgrade if modified => {
                copy_existing_path(&live_target, &live_path(stage_root, &record.path)?)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn materialize_system_assets(
    layout: &StateLayout,
    stage_root: &Path,
    live_root: &Path,
    packages: &BTreeMap<String, SystemPackageMetadata>,
) -> Result<(), InstallError> {
    for metadata in packages.values() {
        write_asset(stage_root, metadata.sysusers.as_ref())?;
        write_asset(stage_root, metadata.tmpfiles.as_ref())?;
    }

    for (link, target) in alternative_winners(packages) {
        let link_path = live_path(stage_root, &link)?;
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let resolved_target = resolve_absolute_target(live_root, &target);
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path)?;
        }
        symlink(resolved_target, link_path)?;
    }

    provider_assets::materialize_provider_assets_under_root(
        stage_root,
        packages,
        &active_provider_families(layout)?,
    )?;

    Ok(())
}

fn apply_staged_entry(
    unpack_root: &Path,
    stage_root: &Path,
    entry: &ManifestEntry,
    package: &BuiltPackage,
    conffile_mode: InstallConffileMode,
) -> Result<(), InstallError> {
    let source = unpack_root.join(crate::fsops::strip_leading_slash(&entry.path)?);
    let target = live_path(stage_root, &entry.path)?;
    if entry.kind == ManifestEntryKind::Directory {
        fs::create_dir_all(target)?;
        return Ok(());
    }
    let is_conffile = package.conffiles.iter().any(|path| path == &entry.path);
    if is_conffile && (target.exists() || target.is_symlink()) {
        let _ = apply_conffile_entry(&source, &target, entry, conffile_mode)?;
        return Ok(());
    }
    copy_packaged_entry(unpack_root, &target, entry)
}

fn alternative_winners(
    packages: &BTreeMap<String, SystemPackageMetadata>,
) -> BTreeMap<String, String> {
    let mut winners = BTreeMap::<String, (i64, String, String)>::new();
    for (package_name, metadata) in packages {
        for alternative in &metadata.alternatives {
            let candidate = (
                alternative.priority,
                package_name.clone(),
                alternative.path.clone(),
            );
            match winners.get(&alternative.link) {
                Some(current)
                    if alternative.priority < current.0
                        || (alternative.priority == current.0
                            && (candidate.1.as_str(), candidate.2.as_str())
                                >= (current.1.as_str(), current.2.as_str())) => {}
                _ => {
                    winners.insert(alternative.link.clone(), candidate);
                }
            }
        }
    }

    winners
        .into_iter()
        .map(|(link, (_, _, target))| (link, target))
        .collect()
}
