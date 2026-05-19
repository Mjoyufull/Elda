use std::fs;

use tempfile::TempDir;

use crate::model::{RemoteDocument, TrustMode};

use super::super::{SyncOptions, inspect_remote_trust, load_snapshot, sync_remotes};
use super::{fixture_key_fingerprint, write_remote_document, write_signed_remote_index};

#[test]
fn sync_clears_snapshot_when_remote_is_removed() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "removed-tool");
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_key_fingerprint()],
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
            ..SyncOptions::default()
        },
    )
    .expect("initial sync should succeed");
    fs::remove_file(remotes_dir.join("main.toml")).expect("remote document should be removed");

    let report = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
            ..SyncOptions::default()
        },
    )
    .expect("sync with no remotes should clear stale packages");
    let snapshot = load_snapshot(&snapshot_path).expect("snapshot should load");

    assert_eq!(report.remote_count, 0);
    assert_eq!(report.package_count, 0);
    assert_eq!(report.package_deltas[0].remote_name, "main");
    assert_eq!(
        report.package_deltas[0].removed_packages,
        vec!["removed-tool"]
    );
    assert!(snapshot.packages.is_empty());
    assert!(snapshot.remotes.is_empty());
}

#[test]
fn trust_inspection_reports_persisted_keys_after_sync() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "trusted-tool");
    let remote = RemoteDocument {
        name: "main".to_owned(),
        index_url: format!("file://{}", index_path.display()),
        channel: "stable".to_owned(),
        packages_url: None,
        metadata_url: Some("file:///tmp/remote-metadata-v1.toml".to_owned()),
        signature_url: None,
        enabled: true,
        trust: TrustMode::Pinned,
        trusted_keys: vec![fixture_key_fingerprint()],
        allow_stale: true,
        exclude: Vec::new(),
        priority: 100,
    };
    write_remote_document(&remotes_dir, remote.clone());
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
            ..SyncOptions::default()
        },
    )
    .expect("sync should succeed");

    let trust =
        inspect_remote_trust(&snapshot_path, &remote).expect("trust inspection should load");
    assert_eq!(trust.remote_name, "main");
    assert_eq!(
        trust.configured_trusted_keys,
        vec![fixture_key_fingerprint()]
    );
    assert_eq!(trust.persisted_trusted_public_keys.len(), 1);
    assert_eq!(trust.snapshot_verified, Some(true));
    assert!(
        trust
            .payload_verification
            .contains("signed payload verification enabled")
    );
}

#[test]
fn sync_targets_only_named_remote() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let main_index = write_signed_remote_index(tempdir.path(), "main-tool");
    let extra_index = write_signed_remote_index(tempdir.path(), "extra-tool");
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", main_index.display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_key_fingerprint()],
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "extra".to_owned(),
            index_url: format!("file://{}", extra_index.display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_key_fingerprint()],
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );

    let report = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions {
            target_remotes: vec!["main".to_owned()],
            ..SyncOptions::default()
        },
    )
    .expect("targeted sync should succeed");

    assert_eq!(report.remote_count, 1);
    assert_eq!(report.package_count, 1);
    assert_eq!(report.remotes[0].name, "main");
    assert_eq!(report.remotes[0].package_count, 1);
}

#[test]
fn sync_target_rejects_unknown_remote() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");

    let error = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions {
            target_remotes: vec!["missing".to_owned()],
            ..SyncOptions::default()
        },
    )
    .expect_err("unknown target should fail");

    assert!(
        error
            .to_string()
            .contains("remote `missing` is not registered")
    );
}
