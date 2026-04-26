use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::model::{RemoteDocument, TrustMode};

use super::{SyncOptions, load_remote_payload_trust, load_snapshot, sync_remotes};

mod channel;

#[test]
fn sync_verifies_pinned_remote_and_writes_snapshot_metadata() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "demo-tool");
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
            priority: 100,
        },
    );

    let report = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("sync should succeed");

    assert_eq!(report.package_count, 1);
    assert_eq!(report.verified_remote_count, 1);
    assert_eq!(report.failed_remote_count, 0);
    assert!(report.remotes[0].verified);
    assert_eq!(
        report.remotes[0].selected_key.as_deref(),
        Some("fixture-remote")
    );

    let snapshot = load_snapshot(&report.snapshot_path).expect("snapshot should load");
    assert_eq!(snapshot.schema_version, 3);
    assert!(!snapshot.offline);
    assert_eq!(snapshot.packages.len(), 1);
    assert_eq!(snapshot.remotes[0].channel, "stable");
}

#[test]
fn sync_uses_stale_verified_snapshot_when_remote_is_unreachable_and_policy_allows_it() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "stale-tool");
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
            allow_stale: true,
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
        },
    )
    .expect("initial sync should succeed");
    fs::remove_file(&index_path).expect("index file should be removed");
    fs::remove_file(index_path.with_extension("toml.sig"))
        .expect("signature file should be removed");

    let report = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("sync should work");

    assert_eq!(report.package_count, 1);
    assert_eq!(report.stale_remote_count, 1);
    assert_eq!(report.failed_remote_count, 0);
    assert!(report.remotes[0].stale);
    assert_eq!(report.remotes[0].source, "stale-cache");
}

#[test]
fn sync_persists_trusted_public_keys_for_payload_verification() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "payload-tool");
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
        },
    )
    .expect("sync should succeed");
    let payload_trust =
        load_remote_payload_trust(&snapshot_path, "main").expect("payload trust should load");

    assert!(payload_trust.verified);
    assert_eq!(payload_trust.trusted_public_keys.len(), 1);
    assert_eq!(
        payload_trust.trusted_public_keys[0].fingerprint,
        fixture_key_fingerprint()
    );
}

#[test]
fn offline_sync_uses_cached_verified_snapshots_only() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "offline-tool");
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
            allow_stale: true,
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
        },
    )
    .expect("initial sync should succeed");
    fs::remove_file(&index_path).expect("index file should be removed");
    fs::remove_file(index_path.with_extension("toml.sig"))
        .expect("signature file should be removed");

    let report = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: true,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("offline sync should succeed");

    assert!(report.offline);
    assert_eq!(report.package_count, 1);
    assert_eq!(report.stale_remote_count, 1);
    assert_eq!(report.remotes[0].source, "offline-cache");
}

#[test]
fn tofu_sync_allows_first_use_enrollment_when_enabled() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "tofu-allow-tool");
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
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            priority: 100,
        },
    );

    let report = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions {
            offline: false,
            allow_initial_tofu: true,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("tofu sync should enroll first-use trust");

    assert_eq!(report.verified_remote_count, 1);
    assert!(report.remotes[0].verified);
}

#[test]
fn tofu_sync_rejects_first_use_enrollment_when_disabled() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_remote_index(tempdir.path(), "tofu-deny-tool");
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
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            priority: 100,
        },
    );

    let error = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect_err("disabled first-use tofu should fail");

    assert!(
        error
            .to_string()
            .contains("requires explicit trust bootstrap before unattended sync")
    );
}

fn write_signed_remote_index(root: &Path, package_name: &str) -> PathBuf {
    let index_path = root.join(format!("{package_name}.toml"));
    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"{package_name}\"\npkg_lua = '''\npkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file:///tmp/{package_name}\",\n    sha256 = \"deadbeef\",\n    rename = \"{package_name}\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
        ),
    )
    .expect("remote index should be written");
    write_signature_file(&index_path);
    index_path
}

fn write_signature_file(index_path: &Path) {
    let content = fs::read(index_path).expect("index should exist");
    let signing_key = fixture_signing_key();
    let signature = signing_key.sign(&content);
    let signature_path = index_path.with_extension("toml.sig");

    fs::write(
        signature_path,
        format!(
            "key_id = \"fixture-remote\"\npublic_key = \"{}\"\nsignature = \"{}\"\n",
            STANDARD.encode(signing_key.verifying_key().as_bytes()),
            STANDARD.encode(signature.to_bytes()),
        ),
    )
    .expect("signature file should be written");
}

fn write_remote_document(remotes_dir: &Path, remote: RemoteDocument) {
    fs::write(
        remotes_dir.join(format!("{}.toml", remote.name)),
        toml::to_string_pretty(&remote).expect("remote toml should encode"),
    )
    .expect("remote document should be written");
}

fn fixture_key_fingerprint() -> String {
    let mut hasher = Sha256::new();
    hasher.update(fixture_signing_key().verifying_key().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn fixture_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}
