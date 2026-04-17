use std::fs;
use std::path::PathBuf;

use elda_build::{
    BuiltPackage, CacheEntryKind, PackageDependency, PackageManifest, record_cache_access,
};
use elda_db::{Database, StateLayout};

use crate::InstallError;
use crate::archive_state::{ArchivedDependency, ArchivedPackage};

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
    })
}

pub(crate) fn archive_package_paths(
    layout: &StateLayout,
    archived: &ArchivedPackage,
) -> (PathBuf, PathBuf) {
    let base_name = format!(
        "{}-{}-{}-{}",
        archived.pkgname, archived.pkgver, archived.pkgrel, archived.arch
    );
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
