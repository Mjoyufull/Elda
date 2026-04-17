use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use elda_db::{Database, InstallationMode, StateLayout};

use crate::InstallError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArchivedStateDocument {
    pub(crate) format_version: u32,
    pub(crate) state_id: String,
    pub(crate) installation_mode: String,
    pub(crate) prefix: String,
    pub(crate) world: Vec<String>,
    pub(crate) packages: Vec<ArchivedPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArchivedPackage {
    pub(crate) pkgname: String,
    pub(crate) epoch: u64,
    pub(crate) pkgver: String,
    pub(crate) pkgrel: u64,
    pub(crate) arch: String,
    pub(crate) package_kind: String,
    #[serde(default)]
    pub(crate) variant_id: Option<String>,
    pub(crate) install_reason: String,
    pub(crate) source_kind: String,
    pub(crate) source_ref: Option<String>,
    pub(crate) remote_name: Option<String>,
    pub(crate) repo_commit: Option<String>,
    pub(crate) payload_sha256: Option<String>,
    pub(crate) manifest_hash: Option<String>,
    #[serde(default)]
    pub(crate) conffiles: Vec<String>,
    pub(crate) pinned_version: Option<String>,
    pub(crate) held: bool,
    pub(crate) hold_source: Option<String>,
    pub(crate) dependencies: Vec<ArchivedDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArchivedDependency {
    pub(crate) dependency_name: String,
    pub(crate) dependency_kind: String,
    pub(crate) raw_expr: String,
    pub(crate) is_weak: bool,
    pub(crate) provider_group: Option<String>,
}

pub(crate) fn archive_current_state(
    database: &Database,
    state_id: &str,
) -> Result<(), InstallError> {
    let snapshot = database.state_snapshot()?;
    let packages = database
        .list_installed_packages()?
        .into_iter()
        .map(|package| package.pkgname)
        .collect::<Vec<_>>();
    let mut archived_packages = packages
        .into_iter()
        .map(|package_name| archived_package(database, &package_name))
        .collect::<Result<Vec<_>, _>>()?;
    archived_packages.sort_by(|left, right| left.pkgname.cmp(&right.pkgname));

    let document = ArchivedStateDocument {
        format_version: 1,
        state_id: state_id.to_owned(),
        installation_mode: match database.layout().mode {
            InstallationMode::System => "system".to_owned(),
            InstallationMode::Prefix => "prefix".to_owned(),
        },
        prefix: database.layout().prefix.display().to_string(),
        world: snapshot.world,
        packages: archived_packages,
    };
    let archive_path = state_archive_path(database.layout(), state_id);
    fs::write(archive_path, serde_json::to_vec_pretty(&document)?)?;

    Ok(())
}

pub(crate) fn available_state_ids(layout: &StateLayout) -> Result<Vec<String>, InstallError> {
    let mut archive_ids = fs::read_dir(&layout.states_dir)?
        .map(|entry| {
            entry.map(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .trim_end_matches(".json")
                    .to_owned()
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    archive_ids.sort();
    archive_ids.dedup();

    Ok(archive_ids)
}

#[must_use]
pub(crate) fn state_archive_path(layout: &StateLayout, state_id: &str) -> PathBuf {
    layout.states_dir.join(format!("{state_id}.json"))
}

pub(crate) fn read_state_archive(
    layout: &StateLayout,
    state_id: &str,
) -> Result<ArchivedStateDocument, InstallError> {
    let archive_path = state_archive_path(layout, state_id);
    let bytes = fs::read(&archive_path).map_err(|error| {
        InstallError::StateArchive(format!(
            "failed to read archived state `{state_id}` at {}: {error}",
            archive_path.display()
        ))
    })?;
    serde_json::from_slice(&bytes).map_err(InstallError::from)
}

#[must_use]
pub(crate) fn installed_matches_archive(
    installed: &elda_db::InstalledPackageDetails,
    archived: &ArchivedPackage,
) -> bool {
    installed.epoch == archived.epoch
        && installed.pkgver == archived.pkgver
        && installed.pkgrel == archived.pkgrel
        && installed.arch.as_deref() == Some(archived.arch.as_str())
        && installed.package_kind == archived.package_kind
        && installed.variant_id == archived.variant_id
        && installed.install_reason == archived.install_reason
        && installed.source_kind == archived.source_kind
        && installed.source_ref == archived.source_ref
        && installed.remote_name == archived.remote_name
        && installed.repo_commit == archived.repo_commit
        && installed.payload_sha256 == archived.payload_sha256
        && installed.manifest_hash == archived.manifest_hash
        && installed.pinned_version == archived.pinned_version
        && installed.held == archived.held
        && installed.hold_source == archived.hold_source
}

fn archived_package(
    database: &Database,
    package_name: &str,
) -> Result<ArchivedPackage, InstallError> {
    let package = database.installed_package(package_name)?.ok_or_else(|| {
        InstallError::StateArchive(format!(
            "installed package `{package_name}` disappeared while archiving state"
        ))
    })?;
    let arch = package.arch.clone().ok_or_else(|| {
        InstallError::StateArchive(format!(
            "installed package `{package_name}` is missing its canonical arch"
        ))
    })?;
    let dependencies = database
        .package_dependencies(package_name, true)?
        .into_iter()
        .map(|dependency| ArchivedDependency {
            dependency_name: dependency.dependency_name,
            dependency_kind: dependency.dependency_kind,
            raw_expr: dependency.raw_expr,
            is_weak: dependency.is_weak,
            provider_group: dependency.provider_group,
        })
        .collect::<Vec<_>>();
    let conffiles = database
        .package_files(package_name)?
        .into_iter()
        .filter(|record| record.is_conffile)
        .map(|record| record.path)
        .collect::<Vec<_>>();

    Ok(ArchivedPackage {
        pkgname: package.pkgname,
        epoch: package.epoch,
        pkgver: package.pkgver,
        pkgrel: package.pkgrel,
        arch,
        package_kind: package.package_kind,
        variant_id: package.variant_id,
        install_reason: package.install_reason,
        source_kind: package.source_kind,
        source_ref: package.source_ref,
        remote_name: package.remote_name,
        repo_commit: package.repo_commit,
        payload_sha256: package.payload_sha256,
        manifest_hash: package.manifest_hash,
        conffiles,
        pinned_version: package.pinned_version,
        held: package.held,
        hold_source: package.hold_source,
        dependencies,
    })
}
