use std::fs;
use std::path::PathBuf;

use elda_build::{
    BuiltPackage, CacheEntryKind, ObjectMetadata, PackageDependency, PackageManifest,
    record_cache_access,
};
use elda_db::{Database, StateLayout};

use crate::InstallError;
use crate::archive_state::{ArchivedDependency, ArchivedPackage};
use crate::system_backend::load_installed_system_metadata;

pub(crate) fn built_package_from_archive(
    database: &Database,
    archived: &ArchivedPackage,
) -> Result<BuiltPackage, InstallError> {
    let (payload_path, manifest_path) = archive_package_paths(database.layout(), archived);
    ensure_cached_artifact(&payload_path, "payload")?;
    ensure_cached_artifact(&manifest_path, "manifest")?;
    record_cache_access(&payload_path, CacheEntryKind::PackagePayload)?;
    record_cache_access(&manifest_path, CacheEntryKind::PackageManifest)?;

    let manifest = serde_json::from_slice::<PackageManifest>(&fs::read(&manifest_path)?)?;
    Ok(BuiltPackage {
        package_name: archived.pkgname.clone(),
        epoch: archived.epoch,
        pkgver: archived.pkgver.clone(),
        pkgrel: archived.pkgrel,
        arch: archived.arch.clone(),
        package_kind: archived.package_kind.clone(),
        variant_id: archived
            .variant_id
            .clone()
            .unwrap_or_else(|| "default".to_owned()),
        source_kind: archived.source_kind.clone(),
        source_ref: archived.source_ref.clone(),
        remote_name: archived.remote_name.clone(),
        repo_commit: archived.repo_commit.clone(),
        dependencies: archived_dependencies(&archived.dependencies),
        conffiles: archived.conffiles.clone(),
        system_metadata: archived.system_metadata.clone(),
        object_metadata: ObjectMetadata::default(),
        payload_path,
        payload_sha256: required_archive_field(
            archived.payload_sha256.clone(),
            &archived.pkgname,
            "payload sha256",
        )?,
        manifest_path,
        manifest_hash: required_archive_field(
            archived.manifest_hash.clone(),
            &archived.pkgname,
            "manifest hash",
        )?,
        manifest,
        interbuild: None,
    })
}

pub(crate) fn built_package_from_installed(
    database: &Database,
    package_name: &str,
) -> Result<BuiltPackage, InstallError> {
    let installed = database.installed_package(package_name)?.ok_or_else(|| {
        InstallError::StateArchive(format!(
            "installed package `{package_name}` disappeared while preparing the staged system state"
        ))
    })?;
    let arch = installed.arch.clone().ok_or_else(|| {
        InstallError::StateArchive(format!(
            "installed package `{package_name}` is missing its canonical arch"
        ))
    })?;
    let (payload_path, manifest_path) = archive_package_paths_for_fields(
        database.layout(),
        &installed.pkgname,
        &installed.pkgver,
        installed.pkgrel,
        &arch,
    );
    ensure_cached_artifact(&payload_path, "payload")?;
    ensure_cached_artifact(&manifest_path, "manifest")?;
    record_cache_access(&payload_path, CacheEntryKind::PackagePayload)?;
    record_cache_access(&manifest_path, CacheEntryKind::PackageManifest)?;

    let manifest = serde_json::from_slice::<PackageManifest>(&fs::read(&manifest_path)?)?;
    let dependencies = database
        .package_dependencies(package_name, true)?
        .into_iter()
        .map(|dependency| PackageDependency {
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
    let system_metadata =
        load_installed_system_metadata(database.layout(), package_name)?.unwrap_or_default();

    Ok(BuiltPackage {
        package_name: installed.pkgname.clone(),
        epoch: installed.epoch,
        pkgver: installed.pkgver.clone(),
        pkgrel: installed.pkgrel,
        arch,
        package_kind: installed.package_kind,
        variant_id: installed.variant_id.unwrap_or_else(|| "default".to_owned()),
        source_kind: installed.source_kind,
        source_ref: installed.source_ref,
        remote_name: installed.remote_name,
        repo_commit: installed.repo_commit,
        dependencies,
        conffiles,
        system_metadata,
        object_metadata: ObjectMetadata::default(),
        payload_path,
        payload_sha256: required_archive_field(
            installed.payload_sha256,
            package_name,
            "payload sha256",
        )?,
        manifest_path,
        manifest_hash: required_archive_field(
            installed.manifest_hash,
            package_name,
            "manifest hash",
        )?,
        manifest,
        interbuild: None,
    })
}

pub(crate) fn archive_package_paths(
    layout: &StateLayout,
    archived: &ArchivedPackage,
) -> (PathBuf, PathBuf) {
    archive_package_paths_for_fields(
        layout,
        &archived.pkgname,
        &archived.pkgver,
        archived.pkgrel,
        &archived.arch,
    )
}

fn archive_package_paths_for_fields(
    layout: &StateLayout,
    package_name: &str,
    pkgver: &str,
    pkgrel: u64,
    arch: &str,
) -> (PathBuf, PathBuf) {
    let base_name = format!("{package_name}-{pkgver}-{pkgrel}-{arch}");
    (
        layout
            .cache_pkg_dir
            .join(format!("{base_name}.pkg.tar.zst")),
        layout.cache_pkg_dir.join(format!("{base_name}.manifest")),
    )
}

fn ensure_cached_artifact(path: &std::path::Path, artifact_kind: &str) -> Result<(), InstallError> {
    if path.exists() {
        return Ok(());
    }

    Err(InstallError::StateArchive(format!(
        "cached {artifact_kind} for rollback is missing: {}",
        path.display()
    )))
}

fn required_archive_field(
    value: Option<String>,
    package_name: &str,
    field_label: &str,
) -> Result<String, InstallError> {
    value.ok_or_else(|| {
        InstallError::StateArchive(format!(
            "archived package `{package_name}` is missing {field_label} metadata"
        ))
    })
}

fn archived_dependencies(dependencies: &[ArchivedDependency]) -> Vec<PackageDependency> {
    dependencies
        .iter()
        .map(|dependency| PackageDependency {
            dependency_name: dependency.dependency_name.clone(),
            dependency_kind: dependency.dependency_kind.clone(),
            raw_expr: dependency.raw_expr.clone(),
            is_weak: dependency.is_weak,
            provider_group: dependency.provider_group.clone(),
        })
        .collect()
}
