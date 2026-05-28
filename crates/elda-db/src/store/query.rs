use std::fs;

use rusqlite::Connection;

use crate::error::DbError;
use crate::schema;
use crate::store::{
    BootstrapReport, Database, HealthReport, InstalledPackageDetails, InstalledPackageRecord,
    PackageDependencyRecord, PackageFileRecord, ReverseDependencyRecord, StateSnapshot,
};

impl Database {
    pub fn bootstrap(&self) -> Result<BootstrapReport, DbError> {
        self.layout.ensure_exists()?;
        let _lock = self.acquire_mutation_lock()?;
        let created_database = !self.layout.db_path.exists();
        let connection = Connection::open(&self.layout.db_path)?;
        schema::initialize(&connection)?;
        let schema_version = schema::current_version(&connection)?;

        Ok(BootstrapReport {
            created_database,
            schema_version,
        })
    }

    pub fn list_installed_packages(&self) -> Result<Vec<InstalledPackageRecord>, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let mut statement = connection.prepare(
            "
            SELECT pkgname, arch, epoch, pkgver, pkgrel, install_reason, source_kind, remote_name, state_id
                 , package_kind, variant_id, source_ref, repo_commit, payload_sha256, manifest_hash
                 , pinned_version, held, hold_source
            FROM installed_packages
            ORDER BY pkgname ASC, arch ASC
            ",
        )?;
        let rows = statement.query_map([], |row| {
            let epoch: u64 = row.get(2)?;
            let pkgver: String = row.get(3)?;
            let pkgrel: u64 = row.get(4)?;
            Ok(InstalledPackageRecord {
                pkgname: row.get(0)?,
                arch: row.get(1)?,
                version: format!("{epoch}:{pkgver}-{pkgrel}"),
                install_reason: row.get(5)?,
                source_kind: row.get(6)?,
                remote_name: row.get(7)?,
                state_id: row.get(8)?,
                package_kind: row.get(9)?,
                variant_id: row.get(10)?,
                source_ref: row.get(11)?,
                repo_commit: row.get(12)?,
                payload_sha256: row.get(13)?,
                manifest_hash: row.get(14)?,
                pinned_version: row.get(15)?,
                held: row.get::<_, i64>(16)? != 0,
                hold_source: row.get(17)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn state_snapshot(&self) -> Result<StateSnapshot, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let installed_packages: usize =
            connection.query_row("SELECT COUNT(*) FROM installed_packages", [], |row| {
                row.get(0)
            })?;

        let schema_version = schema::current_version(&connection)?;
        let active_state = fs::read_to_string(&self.layout.current_state_path)?
            .lines()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let world = read_world(&self.layout.world_path)?;

        Ok(StateSnapshot {
            schema_version,
            active_state,
            world,
            installed_packages,
        })
    }

    pub fn health_report(&self) -> Result<HealthReport, DbError> {
        let snapshot = self.state_snapshot()?;
        let pending_journals = fs::read_dir(&self.layout.journal_dir)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut issues = Vec::new();

        if snapshot.active_state.is_some() && !self.layout.states_dir.exists() {
            issues.push("active state pointer exists but states directory is missing".to_owned());
        }

        Ok(HealthReport {
            schema_version: snapshot.schema_version,
            installed_packages: snapshot.installed_packages,
            world_anchors: snapshot.world.len(),
            pending_journals,
            issues,
        })
    }

    pub fn installed_package(
        &self,
        package_name: &str,
    ) -> Result<Option<InstalledPackageDetails>, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let mut statement = connection.prepare(
            "
            SELECT pkgname, epoch, pkgver, pkgrel, arch, package_kind, variant_id, install_reason,
                   source_kind, source_ref, remote_name, state_id, activation_backend, repo_commit,
                   payload_sha256, manifest_hash, pinned_version, held, hold_source
            FROM installed_packages
            WHERE pkgname = ?
            ORDER BY installed_at DESC
            LIMIT 1
            ",
        )?;
        let mut rows = statement.query([package_name])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        Ok(Some(InstalledPackageDetails {
            pkgname: row.get(0)?,
            epoch: row.get(1)?,
            pkgver: row.get(2)?,
            pkgrel: row.get(3)?,
            arch: row.get(4)?,
            package_kind: row.get(5)?,
            variant_id: row.get(6)?,
            install_reason: row.get(7)?,
            source_kind: row.get(8)?,
            source_ref: row.get(9)?,
            remote_name: row.get(10)?,
            state_id: row.get(11)?,
            activation_backend: row.get(12)?,
            repo_commit: row.get(13)?,
            payload_sha256: row.get(14)?,
            manifest_hash: row.get(15)?,
            pinned_version: row.get(16)?,
            held: row.get::<_, i64>(17)? != 0,
            hold_source: row.get(18)?,
        }))
    }

    pub fn package_files(&self, package_name: &str) -> Result<Vec<PackageFileRecord>, DbError> {
        self.query_file_records(
            "
            SELECT pkgname, arch, path, path_kind, sha256, size, mode, link_target, is_conffile
            FROM package_files
            WHERE pkgname = ?
            ORDER BY path ASC
            ",
            package_name,
        )
    }

    pub fn search_package_files(&self, query: &str) -> Result<Vec<PackageFileRecord>, DbError> {
        let pattern = format!("%{query}%");
        self.query_file_records(
            "
            SELECT pkgname, arch, path, path_kind, sha256, size, mode, link_target, is_conffile
            FROM package_files
            WHERE path LIKE ?
            ORDER BY path ASC, pkgname ASC
            ",
            &pattern,
        )
    }

    pub fn path_owners(&self, path: &str) -> Result<Vec<PackageFileRecord>, DbError> {
        self.query_file_records(
            "
            SELECT pkgname, arch, path, path_kind, sha256, size, mode, link_target, is_conffile
            FROM package_files
            WHERE path = ?
            ORDER BY pkgname ASC
            ",
            path,
        )
    }

    pub fn package_dependencies(
        &self,
        package_name: &str,
        include_weak: bool,
    ) -> Result<Vec<PackageDependencyRecord>, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let sql = if include_weak {
            "
            SELECT pkgname, dependency_name, dependency_kind, raw_expr, is_weak, provider_group
            FROM package_dependencies
            WHERE pkgname = ?
            ORDER BY is_weak ASC, dependency_kind ASC, dependency_name ASC
            "
        } else {
            "
            SELECT pkgname, dependency_name, dependency_kind, raw_expr, is_weak, provider_group
            FROM package_dependencies
            WHERE pkgname = ? AND is_weak = 0
            ORDER BY dependency_kind ASC, dependency_name ASC
            "
        };
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map([package_name], |row| {
            Ok(PackageDependencyRecord {
                pkgname: row.get(0)?,
                dependency_name: row.get(1)?,
                dependency_kind: row.get(2)?,
                raw_expr: row.get(3)?,
                is_weak: row.get::<_, i64>(4)? != 0,
                provider_group: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn reverse_dependencies(
        &self,
        package_name: &str,
        include_weak: bool,
    ) -> Result<Vec<ReverseDependencyRecord>, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let sql = if include_weak {
            "
            SELECT d.pkgname, d.dependency_kind, d.raw_expr, d.is_weak, d.provider_group,
                   p.install_reason, p.pinned_version, p.held
            FROM package_dependencies AS d
            JOIN installed_packages AS p ON p.pkgname = d.pkgname
            WHERE d.dependency_name = ?
            ORDER BY d.is_weak ASC, d.pkgname ASC
            "
        } else {
            "
            SELECT d.pkgname, d.dependency_kind, d.raw_expr, d.is_weak, d.provider_group,
                   p.install_reason, p.pinned_version, p.held
            FROM package_dependencies AS d
            JOIN installed_packages AS p ON p.pkgname = d.pkgname
            WHERE d.dependency_name = ? AND d.is_weak = 0
            ORDER BY d.pkgname ASC
            "
        };
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map([package_name], |row| {
            Ok(ReverseDependencyRecord {
                pkgname: row.get(0)?,
                dependency_kind: row.get(1)?,
                raw_expr: row.get(2)?,
                is_weak: row.get::<_, i64>(3)? != 0,
                provider_group: row.get(4)?,
                install_reason: row.get(5)?,
                pinned_version: row.get(6)?,
                held: row.get::<_, i64>(7)? != 0,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    fn query_file_records(
        &self,
        sql: &str,
        operand: &str,
    ) -> Result<Vec<PackageFileRecord>, DbError> {
        let connection = Connection::open(&self.layout.db_path)?;
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map([operand], |row| {
            Ok(PackageFileRecord {
                pkgname: row.get(0)?,
                arch: row.get(1)?,
                path: row.get(2)?,
                path_kind: row.get(3)?,
                sha256: row.get(4)?,
                size: row.get(5)?,
                mode: row.get::<_, u64>(6)? as u32,
                link_target: row.get(7)?,
                is_conffile: row.get::<_, i64>(8)? != 0,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

fn read_world(world_path: &std::path::Path) -> Result<Vec<String>, DbError> {
    let world = fs::read_to_string(world_path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    Ok(world)
}
