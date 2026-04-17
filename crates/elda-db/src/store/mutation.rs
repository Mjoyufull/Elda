use std::fs;

use rusqlite::{Connection, Transaction, params};

use crate::error::DbError;
use crate::store::{Database, InstallRecord, PackageDependencyRecord, PackageFileRecord};

impl Database {
    pub fn record_install(
        &self,
        package: &InstallRecord,
        files: &[PackageFileRecord],
        dependencies: &[PackageDependencyRecord],
    ) -> Result<(), DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let transaction = connection.unchecked_transaction()?;
        delete_package_rows(&transaction, &package.pkgname)?;
        insert_package_row(&transaction, package)?;
        insert_file_rows(&transaction, files)?;
        insert_dependency_rows(&transaction, dependencies)?;
        transaction.commit()?;

        update_world(
            &self.layout.world_path,
            &package.pkgname,
            package.install_reason == "explicit",
        )?;

        Ok(())
    }

    pub fn remove_package(&self, package_name: &str) -> Result<(), DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let transaction = connection.unchecked_transaction()?;
        delete_package_rows(&transaction, package_name)?;
        transaction.commit()?;
        update_world(&self.layout.world_path, package_name, false)?;

        Ok(())
    }

    pub fn set_current_state(&self, state_id: &str) -> Result<(), DbError> {
        fs::write(&self.layout.current_state_path, format!("{state_id}\n"))?;
        Ok(())
    }

    pub fn set_install_reason(
        &self,
        package_name: &str,
        install_reason: &str,
    ) -> Result<(), DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        connection.execute(
            "UPDATE installed_packages SET install_reason = ? WHERE pkgname = ?",
            [install_reason, package_name],
        )?;
        update_world(
            &self.layout.world_path,
            package_name,
            install_reason == "explicit",
        )?;

        Ok(())
    }

    pub fn set_pinned_version(
        &self,
        package_name: &str,
        pinned_version: Option<&str>,
    ) -> Result<(), DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        connection.execute(
            "UPDATE installed_packages SET pinned_version = ? WHERE pkgname = ?",
            (&pinned_version, &package_name),
        )?;

        Ok(())
    }

    pub fn set_hold(
        &self,
        package_name: &str,
        held: bool,
        hold_source: Option<&str>,
    ) -> Result<(), DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        connection.execute(
            "UPDATE installed_packages SET held = ?, hold_source = ? WHERE pkgname = ?",
            (if held { 1_i64 } else { 0_i64 }, hold_source, package_name),
        )?;

        Ok(())
    }
}

fn delete_package_rows(transaction: &Transaction<'_>, package_name: &str) -> Result<(), DbError> {
    transaction.execute(
        "DELETE FROM package_files WHERE pkgname = ?",
        [package_name],
    )?;
    transaction.execute(
        "DELETE FROM package_dependencies WHERE pkgname = ?",
        [package_name],
    )?;
    transaction.execute(
        "DELETE FROM installed_packages WHERE pkgname = ?",
        [package_name],
    )?;

    Ok(())
}

fn insert_package_row(
    transaction: &Transaction<'_>,
    package: &InstallRecord,
) -> Result<(), DbError> {
    transaction.execute(
        "
        INSERT INTO installed_packages (
          pkgname, epoch, pkgver, pkgrel, arch, package_kind, variant_id, install_reason,
          source_kind, source_ref, remote_name, channel, state_id, activation_backend,
          repo_commit, payload_sha256, manifest_hash, pinned_version, held, hold_source
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ",
        params![
            &package.pkgname,
            package.epoch,
            &package.pkgver,
            package.pkgrel,
            &package.arch,
            &package.package_kind,
            &package.variant_id,
            &package.install_reason,
            &package.source_kind,
            &package.source_ref,
            &package.remote_name,
            &package.channel,
            &package.state_id,
            &package.activation_backend,
            &package.repo_commit,
            &package.payload_sha256,
            &package.manifest_hash,
            &package.pinned_version,
            if package.held { 1_i64 } else { 0_i64 },
            &package.hold_source,
        ],
    )?;

    Ok(())
}

fn insert_dependency_rows(
    transaction: &Transaction<'_>,
    dependencies: &[PackageDependencyRecord],
) -> Result<(), DbError> {
    let mut statement = transaction.prepare(
        "
        INSERT INTO package_dependencies (
          pkgname, dependency_name, dependency_kind, raw_expr, is_weak, provider_group
        )
        VALUES (?, ?, ?, ?, ?, ?)
        ",
    )?;

    for dependency in dependencies {
        statement.execute((
            &dependency.pkgname,
            &dependency.dependency_name,
            &dependency.dependency_kind,
            &dependency.raw_expr,
            if dependency.is_weak { 1_i64 } else { 0_i64 },
            &dependency.provider_group,
        ))?;
    }

    Ok(())
}

fn insert_file_rows(
    transaction: &Transaction<'_>,
    files: &[PackageFileRecord],
) -> Result<(), DbError> {
    let mut statement = transaction.prepare(
        "
        INSERT INTO package_files (
          pkgname, arch, path, path_kind, sha256, size, mode, link_target, is_conffile
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ",
    )?;

    for file in files {
        statement.execute((
            &file.pkgname,
            &file.arch,
            &file.path,
            &file.path_kind,
            &file.sha256,
            file.size,
            u64::from(file.mode),
            &file.link_target,
            if file.is_conffile { 1_i64 } else { 0_i64 },
        ))?;
    }

    Ok(())
}

fn update_world(
    world_path: &std::path::Path,
    package_name: &str,
    present: bool,
) -> Result<(), DbError> {
    let mut world = if world_path.exists() {
        fs::read_to_string(world_path)?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    if present {
        if !world.iter().any(|entry| entry == package_name) {
            world.push(package_name.to_owned());
        }
    } else {
        world.retain(|entry| entry != package_name);
    }
    world.sort();
    fs::write(world_path, world.join("\n"))?;

    Ok(())
}
