use std::fs;
use std::path::Path;

use tempfile::TempDir;

use elda_repo::{
    CacheDocument, SyncedIndexSnapshot, SyncedPackageRecord, SyncedRemoteRecord, TrustMode,
    save_cache,
};

use crate::cli::MirrorRemoteArgs;
use crate::operations::mirror_remote;

#[test]
fn mirror_remote_filters_binary_payloads_by_channel() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    super::write_prefix_config(tempdir.path(), "/opt/elda");
    let layout = elda_db::StateLayout::new(tempdir.path(), "/opt/elda");
    layout.ensure_exists().expect("layout should exist");

    let stable_payload = tempdir.path().join("stable-tool.bin");
    let testing_payload = tempdir.path().join("testing-tool.bin");
    fs::write(&stable_payload, b"stable payload").expect("stable payload should exist");
    fs::write(&testing_payload, b"testing payload").expect("testing payload should exist");
    let stable_sha256 = super::sha256_file(&stable_payload);
    let testing_sha256 = super::sha256_file(&testing_payload);
    write_snapshot(
        &layout.db_dir.join("repo-snapshot.json"),
        vec![
            synced_package("main", "stable-tool", None, &stable_payload, &stable_sha256),
            synced_package(
                "main",
                "testing-tool",
                Some("testing"),
                &testing_payload,
                &testing_sha256,
            ),
        ],
    );
    save_cache(
        &layout.caches_dir,
        CacheDocument {
            name: "lan".to_owned(),
            base_url: format!("file://{}", tempdir.path().join("lan-cache").display()),
            priority: 10,
            enabled: true,
        },
    )
    .expect("cache should be saved");

    let report = mirror_remote(
        layout,
        MirrorRemoteArgs {
            cache: "lan".to_owned(),
            remote: "main".to_owned(),
            channel: Some("testing".to_owned()),
            package: Vec::new(),
            manifest_out: None,
            dry_run: false,
        },
    )
    .expect("mirror-remote should succeed");

    assert_eq!(report.mirrored, 1);
    assert!(
        !tempdir
            .path()
            .join("lan-cache")
            .join(stable_sha256)
            .exists()
    );
    assert!(
        tempdir
            .path()
            .join("lan-cache")
            .join(testing_sha256)
            .exists()
    );
}

fn write_snapshot(path: &Path, packages: Vec<SyncedPackageRecord>) {
    let snapshot = SyncedIndexSnapshot {
        schema_version: 3,
        generated_at: 0,
        offline: false,
        remotes: vec![SyncedRemoteRecord {
            name: "main".to_owned(),
            index_url: "file:///tmp/main.toml".to_owned(),
            channel: "stable".to_owned(),
            priority: 10,
            package_count: packages.len(),
            trust: TrustMode::Insecure,
            verified: false,
            stale: false,
            source: "fresh".to_owned(),
            selected_key: None,
            last_sync_unix: Some(0),
            last_verified_unix: None,
            issue: None,
        }],
        packages,
    };
    fs::write(
        path,
        serde_json::to_vec_pretty(&snapshot).expect("snapshot should encode"),
    )
    .expect("snapshot should be written");
}

fn synced_package(
    remote_name: &str,
    package_name: &str,
    channel: Option<&str>,
    payload_path: &Path,
    sha256: &str,
) -> SyncedPackageRecord {
    SyncedPackageRecord {
        remote_name: remote_name.to_owned(),
        remote_priority: 10,
        pkgname: package_name.to_owned(),
        epoch: 0,
        pkgver: "1.0.0".to_owned(),
        pkgrel: 1,
        arch: vec!["amd64".to_owned()],
        package_kind: "normal".to_owned(),
        variant_id: None,
        summary: Some(format!("{package_name} summary")),
        description: Some(format!("{package_name} description")),
        homepage: None,
        license: None,
        channel: channel.map(ToOwned::to_owned),
        asset_url: Some(format!("file://{}", payload_path.display())),
        sha256: Some(sha256.to_owned()),
        size: None,
        payload_sig: None,
        sbom_url: None,
        attestation_url: None,
        source_kind: None,
        source_ref: None,
        fallback_git_url: None,
        repo_commit: None,
        release_tag: None,
        pkg_lua: format!(
            "pkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{}\",\n    sha256 = \"{}\",\n    rename = \"{package_name}\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}}\n}}\n",
            payload_path.display(),
            sha256,
        ),
    }
}
