use elda_db::Database;

use crate::archive_state::{ArchivedPackage, installed_matches_archive};
use crate::cached_archive::built_package_from_archive;
use crate::install_tx::{InstallRecordOptions, install_built_package_internal};
use crate::remove_tx::remove_package_internal;
use crate::{
    InstallConffileMode, InstallError, InstallExecution, InstallReport, MutationPolicy,
    RemoveConffileMode,
};

pub fn recover_pending_transactions(
    database: &Database,
) -> Result<crate::RecoveryReport, InstallError> {
    database.bootstrap()?;
    let _lock = database.acquire_mutation_lock()?;
    crate::journal::recover_pending_journals(database.layout())
}

pub(super) fn remove_rollback_packages(
    database: &Database,
    package_names: &[String],
) -> Result<(), InstallError> {
    for package_name in package_names {
        if database.installed_package(package_name)?.is_some() {
            remove_package_internal(
                database,
                package_name,
                false,
                false,
                RemoveConffileMode::PreserveAsSave,
                &MutationPolicy::default(),
            )?;
        }
    }
    Ok(())
}

pub(super) fn restore_rollback_packages(
    database: &Database,
    archived_packages: &[ArchivedPackage],
) -> Result<Vec<InstallReport>, InstallError> {
    let mut restored_packages = Vec::new();

    for archived in archived_packages {
        let reinstall_needed = database
            .installed_package(&archived.pkgname)?
            .is_none_or(|installed| !installed_matches_archive(&installed, archived));
        if reinstall_needed {
            if database.installed_package(&archived.pkgname)?.is_some() {
                remove_package_internal(
                    database,
                    &archived.pkgname,
                    false,
                    false,
                    RemoveConffileMode::PreserveAsSave,
                    &MutationPolicy::default(),
                )?;
            }
            restored_packages.push(restore_archived_package(database, archived)?);
        }
        database.set_install_reason(&archived.pkgname, &archived.install_reason)?;
        database.set_pinned_version(&archived.pkgname, archived.pinned_version.as_deref())?;
        database.set_hold(
            &archived.pkgname,
            archived.held,
            archived.hold_source.as_deref(),
        )?;
    }

    Ok(restored_packages)
}

fn restore_archived_package(
    database: &Database,
    archived: &ArchivedPackage,
) -> Result<InstallReport, InstallError> {
    let package = built_package_from_archive(database, archived)?;

    install_built_package_internal(
        database,
        &package,
        &archived.install_reason,
        InstallRecordOptions {
            pinned_version: archived.pinned_version.clone(),
            held: archived.held,
            hold_source: archived.hold_source.clone(),
        },
        InstallExecution {
            archive_state: false,
            take_lock: false,
            conffile_mode: InstallConffileMode::FirstOwnership,
        },
        &MutationPolicy::default(),
    )
}
