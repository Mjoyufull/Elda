use elda_build::BuiltPackage;
use elda_db::{
    Database, InstallRecord, InstallationMode, PackageDependencyRecord, PackageFileRecord,
};

use crate::archive_state::archive_current_state;
use crate::fsops::{
    apply_entry, cleanup_transaction_root, manifest_kind_label, map_manifest_path, unpack_payload,
};
use crate::journal::{JournalState, TransactionJournal, ensure_no_pending_journals};
use crate::snapshot::{post_activation_snapshot, pre_activation_snapshot};
use crate::system_backend::{
    activate_staged_state, activation_backend_name, next_state_prefix, prepare_staged_install,
    reconcile_system_state_after_install, run_install_triggers,
};
use crate::{
    InstallConffileMode, InstallError, InstallExecution, InstallReport, MutationPolicy,
    next_state_id,
};

pub fn install_built_package(
    database: &Database,
    package: &BuiltPackage,
    install_reason: &str,
    pinned_version: Option<String>,
    held: bool,
    hold_source: Option<String>,
    policy: &MutationPolicy,
) -> Result<InstallReport, InstallError> {
    install_built_package_internal(
        database,
        package,
        install_reason,
        pinned_version,
        held,
        hold_source,
        InstallExecution {
            archive_state: true,
            take_lock: true,
            conffile_mode: InstallConffileMode::FirstOwnership,
        },
        policy,
    )
}

pub fn install_upgraded_package(
    database: &Database,
    package: &BuiltPackage,
    install_reason: &str,
    pinned_version: Option<String>,
    held: bool,
    hold_source: Option<String>,
    policy: &MutationPolicy,
) -> Result<InstallReport, InstallError> {
    install_built_package_internal(
        database,
        package,
        install_reason,
        pinned_version,
        held,
        hold_source,
        InstallExecution {
            archive_state: true,
            take_lock: true,
            conffile_mode: InstallConffileMode::Upgrade,
        },
        policy,
    )
}

pub(crate) fn install_built_package_internal(
    database: &Database,
    package: &BuiltPackage,
    install_reason: &str,
    pinned_version: Option<String>,
    held: bool,
    hold_source: Option<String>,
    execution: InstallExecution,
    policy: &MutationPolicy,
) -> Result<InstallReport, InstallError> {
    if execution.take_lock {
        database.bootstrap()?;
    } else {
        database.layout().ensure_exists()?;
    }
    let _lock = if execution.take_lock {
        Some(database.acquire_mutation_lock()?)
    } else {
        None
    };
    ensure_no_pending_journals(database.layout())?;
    validate_install_paths(database, package)?;

    let state_id = next_state_id(next_state_prefix(database.layout()));
    let transaction_root = database
        .layout()
        .tmp_dir
        .join("transactions")
        .join(&state_id);
    let mut journal = begin_install_journal(database, package, &state_id, &transaction_root)?;
    if let Some(snapshot) = pre_activation_snapshot(database.layout(), &state_id, policy) {
        journal.snapshots.push(snapshot);
        journal.persist(database.layout())?;
    }

    apply_install_state(
        database,
        package,
        execution,
        &state_id,
        &transaction_root,
        &mut journal,
    )?;
    let files = package_file_records(package);
    let dependencies = package_dependency_records(package);

    database.record_install(
        &InstallRecord {
            pkgname: package.package_name.clone(),
            epoch: package.epoch,
            pkgver: package.pkgver.clone(),
            pkgrel: package.pkgrel,
            arch: Some(package.arch.clone()),
            package_kind: package.package_kind.clone(),
            variant_id: Some(package.variant_id.clone()),
            install_reason: install_reason.to_owned(),
            source_kind: package.source_kind.clone(),
            source_ref: package.source_ref.clone(),
            remote_name: package.remote_name.clone(),
            channel: None,
            state_id: Some(state_id.clone()),
            activation_backend: Some(activation_backend_name(database.layout()).to_owned()),
            repo_commit: package.repo_commit.clone(),
            payload_sha256: Some(package.payload_sha256.clone()),
            manifest_hash: Some(package.manifest_hash.clone()),
            pinned_version,
            held,
            hold_source,
        },
        &files,
        &dependencies,
    )?;
    reconcile_system_state_after_install(database, package)?;
    run_install_triggers(database, package)?;
    database.set_current_state(&state_id)?;
    if let Some(snapshot) = post_activation_snapshot(database.layout(), &state_id, policy) {
        journal.snapshots.push(snapshot);
        journal.persist(database.layout())?;
    }
    if execution.archive_state {
        archive_current_state(database, &state_id, &journal.snapshots)?;
    }
    journal.state = JournalState::DbCommitted;
    journal.persist(database.layout())?;
    cleanup_transaction_root(&transaction_root)?;
    let snapshots = journal.snapshots.clone();
    journal.remove(database.layout())?;

    Ok(InstallReport {
        package_name: package.package_name.clone(),
        state_id,
        activation_backend: activation_backend_name(database.layout()).to_owned(),
        installed_paths: files.len(),
        snapshots,
    })
}

fn validate_install_paths(database: &Database, package: &BuiltPackage) -> Result<(), InstallError> {
    if database.installed_package(&package.package_name)?.is_some() {
        return Err(InstallError::AlreadyInstalled(package.package_name.clone()));
    }

    for entry in &package.manifest.entries {
        if entry.kind == elda_build::ManifestEntryKind::Directory {
            continue;
        }

        let owners = database.path_owners(&entry.path)?;
        if let Some(owner) = owners
            .iter()
            .find(|owner| owner.pkgname != package.package_name && owner.path_kind != "directory")
        {
            return Err(InstallError::PathConflict {
                path: entry.path.clone(),
                owner: owner.pkgname.clone(),
            });
        }

        let target_path = map_manifest_path(database.layout(), &entry.path)?;
        if (target_path.exists() || target_path.is_symlink())
            && !(package.conffiles.iter().any(|path| path == &entry.path) && owners.is_empty())
        {
            return Err(InstallError::UnmanagedPathCollision(entry.path.clone()));
        }
    }

    Ok(())
}

fn begin_install_journal(
    database: &Database,
    package: &BuiltPackage,
    state_id: &str,
    transaction_root: &std::path::Path,
) -> Result<TransactionJournal, InstallError> {
    let mut journal = TransactionJournal::new_install(
        format!("txn-{state_id}"),
        package.package_name.clone(),
        transaction_root.to_path_buf(),
    );
    journal.state_id = Some(state_id.to_owned());
    journal.persist(database.layout())?;
    std::fs::create_dir_all(transaction_root)?;
    Ok(journal)
}

fn apply_manifest(
    database: &Database,
    package: &BuiltPackage,
    execution: InstallExecution,
    transaction_root: &std::path::Path,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    for entry in &package.manifest.entries {
        let applied = apply_entry(
            database.layout(),
            transaction_root,
            entry,
            package.conffiles.iter().any(|path| path == &entry.path),
            execution.conffile_mode,
        )?;
        journal.created_paths.extend(applied.created_paths);
        journal.persist(database.layout())?;
    }

    let manifest_target = database
        .layout()
        .manifests_dir
        .join(format!("{}.manifest", package.package_name));
    std::fs::copy(&package.manifest_path, &manifest_target)?;
    journal.created_paths.push(manifest_target);
    journal.state = JournalState::FilesApplied;
    journal.persist(database.layout())?;

    Ok(())
}

fn apply_install_state(
    database: &Database,
    package: &BuiltPackage,
    execution: InstallExecution,
    state_id: &str,
    transaction_root: &std::path::Path,
    journal: &mut TransactionJournal,
) -> Result<(), InstallError> {
    if database.layout().mode == InstallationMode::System {
        let staged = prepare_staged_install(
            database,
            package,
            state_id,
            transaction_root,
            journal,
            execution.conffile_mode,
        )?;
        activate_staged_state(database, &staged, journal)?;
        journal.state = JournalState::FilesApplied;
        journal.persist(database.layout())?;
        return Ok(());
    }

    unpack_payload(&package.payload_path, transaction_root)?;
    apply_manifest(database, package, execution, transaction_root, journal)
}

fn package_file_records(package: &BuiltPackage) -> Vec<PackageFileRecord> {
    package
        .manifest
        .entries
        .iter()
        .map(|entry| PackageFileRecord {
            pkgname: package.package_name.clone(),
            arch: Some(package.arch.clone()),
            path: entry.path.clone(),
            path_kind: manifest_kind_label(entry.kind),
            sha256: entry.sha256.clone(),
            size: entry.size,
            mode: entry.mode,
            link_target: entry.link_target.clone(),
            is_conffile: package.conffiles.iter().any(|path| path == &entry.path),
        })
        .collect()
}

fn package_dependency_records(package: &BuiltPackage) -> Vec<PackageDependencyRecord> {
    package
        .dependencies
        .iter()
        .map(|dependency| PackageDependencyRecord {
            pkgname: package.package_name.clone(),
            dependency_name: dependency.dependency_name.clone(),
            dependency_kind: dependency.dependency_kind.clone(),
            raw_expr: dependency.raw_expr.clone(),
            is_weak: dependency.is_weak,
            provider_group: dependency.provider_group.clone(),
        })
        .collect()
}
