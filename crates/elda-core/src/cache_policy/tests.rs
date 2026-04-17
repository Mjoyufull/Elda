use std::fs;
use std::path::Path;

use elda_build::{CacheEntryKind, CacheEntryMetadata, cache_metadata_path};
use elda_db::Database;
use tempfile::TempDir;

use super::cleanup::reconcile_cache_policy;
use super::{CachePolicy, DAY_SECONDS};

#[test]
fn reconcile_prunes_old_source_artifacts_when_usage_exceeds_trigger() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let database = test_database(tempdir.path());
    let source_path = database.layout().cache_src_dir.join("old-source");
    fs::write(&source_path, vec![b'x'; 32]).expect("source cache file should exist");
    write_cache_metadata(&source_path, CacheEntryKind::SourceArtifact, 0);

    let report = reconcile_cache_policy(
        &database,
        CachePolicy {
            payload_retention_secs: 90 * DAY_SECONDS,
            source_retention_secs: 0,
            fixed_trigger_bytes: 8,
            filesystem_trigger_percent: 10,
        },
    )
    .expect("cache cleanup should succeed");

    assert_eq!(report.deleted_entries.len(), 1);
    assert!(!source_path.exists());
}

#[test]
fn reconcile_keeps_installed_payloads_even_when_usage_exceeds_trigger() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let database = test_database(tempdir.path());
    record_installed_package(&database, "keep-tool", "1.0.0", 1, "amd64");
    let payload_path = database
        .layout()
        .cache_pkg_dir
        .join("keep-tool-1.0.0-1-amd64.pkg.tar.zst");
    let manifest_path = database
        .layout()
        .cache_pkg_dir
        .join("keep-tool-1.0.0-1-amd64.manifest");
    fs::write(&payload_path, vec![b'x'; 32]).expect("payload should exist");
    fs::write(&manifest_path, "{}").expect("manifest should exist");
    write_cache_metadata(&payload_path, CacheEntryKind::PackagePayload, 0);

    let report = reconcile_cache_policy(
        &database,
        CachePolicy {
            payload_retention_secs: 0,
            source_retention_secs: 0,
            fixed_trigger_bytes: 8,
            filesystem_trigger_percent: 10,
        },
    )
    .expect("cache cleanup should succeed");

    assert!(report.deleted_entries.is_empty());
    assert!(payload_path.exists());
    assert!(manifest_path.exists());
}

fn test_database(root: &Path) -> Database {
    let database = Database::new(elda_db::StateLayout::new(root, "/opt/elda"));
    database.bootstrap().expect("database should bootstrap");
    database
}

fn record_installed_package(
    database: &Database,
    pkgname: &str,
    pkgver: &str,
    pkgrel: u64,
    arch: &str,
) {
    database
        .record_install(
            &elda_db::InstallRecord {
                pkgname: pkgname.to_owned(),
                epoch: 0,
                pkgver: pkgver.to_owned(),
                pkgrel,
                arch: Some(arch.to_owned()),
                package_kind: "normal".to_owned(),
                variant_id: Some("default".to_owned()),
                install_reason: "explicit".to_owned(),
                source_kind: "repo_binary".to_owned(),
                source_ref: None,
                remote_name: Some("main".to_owned()),
                channel: None,
                state_id: Some("state-1".to_owned()),
                activation_backend: Some("prefix-copy".to_owned()),
                repo_commit: None,
                payload_sha256: Some("deadbeef".to_owned()),
                manifest_hash: Some("feedface".to_owned()),
                pinned_version: None,
                held: false,
                hold_source: None,
            },
            &Vec::<elda_db::PackageFileRecord>::new(),
            &Vec::<elda_db::PackageDependencyRecord>::new(),
        )
        .expect("install record should persist");
}

fn write_cache_metadata(path: &Path, kind: CacheEntryKind, last_access_unix: u64) {
    fs::write(
        cache_metadata_path(path),
        serde_json::to_vec_pretty(&CacheEntryMetadata {
            schema_version: 1,
            kind,
            last_access_unix,
        })
        .expect("cache metadata should encode"),
    )
    .expect("cache metadata should write");
}
