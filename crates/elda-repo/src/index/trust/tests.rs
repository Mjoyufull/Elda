use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::model::{RemoteDocument, TrustMode};

use super::super::select::load_remote_payload_trust;
use super::super::sync::{SyncOptions, sync_remotes};

#[test]
fn tofu_sync_accepts_rotated_key_from_signed_metadata_document() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = tempdir.path().join("rotation-index.toml");
    let metadata_path = tempdir.path().join("remote-metadata-v1.toml");
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    write_signed_remote_index(&index_path, "rotation-tool", &signing_key_a(), "fixture-a");
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            packages_url: None,
            metadata_url: Some(format!("file://{}", metadata_path.display())),
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            priority: 100,
        },
    );

    sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: true,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("initial tofu sync should bootstrap trust");

    write_signed_remote_index(&index_path, "rotation-tool", &signing_key_b(), "fixture-b");
    write_rotation_metadata(
        &metadata_path,
        &signing_key_a(),
        &[("fixture-b", signing_key_b())],
        &[fingerprint_for_key(&signing_key_a())],
    );

    let error = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect_err("rotation should require explicit confirmation");

    assert!(error.to_string().contains("requires operator confirmation"));

    let report = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: vec!["main".to_owned()],
        },
    )
    .expect("rotation metadata should authorize the new signing key once accepted");

    assert_eq!(report.verified_remote_count, 1);
    assert_eq!(report.remotes[0].selected_key.as_deref(), Some("fixture-b"));

    let payload_trust =
        load_remote_payload_trust(&snapshot_path, "main").expect("payload trust should load");
    assert!(payload_trust.verified);
    assert_eq!(payload_trust.trusted_public_keys.len(), 1);
    assert_eq!(
        payload_trust.trusted_public_keys[0].fingerprint,
        fingerprint_for_key(&signing_key_b())
    );
}

#[test]
fn tofu_sync_rejects_rotated_key_without_authorizing_metadata() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = tempdir.path().join("rotation-index.toml");
    let metadata_path = tempdir.path().join("remote-metadata-v1.toml");
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    write_signed_remote_index(&index_path, "rotation-tool", &signing_key_a(), "fixture-a");
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            packages_url: None,
            metadata_url: Some(format!("file://{}", metadata_path.display())),
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            priority: 100,
        },
    );

    sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: true,
            accept_rotated_keys: Vec::new(),
        },
    )
    .expect("initial tofu sync should bootstrap trust");

    write_signed_remote_index(&index_path, "rotation-tool", &signing_key_b(), "fixture-b");
    write_rotation_metadata(
        &metadata_path,
        &signing_key_a(),
        &[("fixture-a", signing_key_a())],
        &[],
    );

    let error = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: vec!["main".to_owned()],
        },
    )
    .expect_err("rotation should fail when metadata does not authorize the new key");

    assert!(
        error
            .to_string()
            .contains("does not authorize the new signing key")
    );
}

fn write_signed_remote_index(
    index_path: &Path,
    package_name: &str,
    signing_key: &SigningKey,
    key_id: &str,
) {
    fs::write(
        index_path,
        format!(
            "[[packages]]\npkgname = \"{package_name}\"\npkg_lua = '''\npkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file:///tmp/{package_name}\",\n    sha256 = \"deadbeef\",\n    rename = \"{package_name}\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
        ),
    )
    .expect("remote index should be written");
    write_signature_file(index_path, signing_key, key_id);
}

fn write_rotation_metadata(
    metadata_path: &Path,
    signing_key: &SigningKey,
    trusted_keys: &[(&str, SigningKey)],
    revoked_fingerprints: &[String],
) {
    let trusted_keys_toml = trusted_keys
        .iter()
        .map(|(key_id, key)| {
            format!(
                "[[trusted_keys]]\nkey_id = \"{key_id}\"\npublic_key = \"{}\"\n",
                STANDARD.encode(key.verifying_key().as_bytes()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let revoked = if revoked_fingerprints.is_empty() {
        "revoked_fingerprints = []\n".to_owned()
    } else {
        format!(
            "revoked_fingerprints = [{}]\n",
            revoked_fingerprints
                .iter()
                .map(|fingerprint| format!("\"{fingerprint}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let content = format!("{trusted_keys_toml}\n{revoked}");

    fs::write(metadata_path, &content).expect("metadata document should be written");
    write_signature_file(metadata_path, signing_key, "fixture-metadata");
}

fn write_signature_file(path: &Path, signing_key: &SigningKey, key_id: &str) {
    let content = fs::read(path).expect("signed file should exist");
    let signature = signing_key.sign(&content);
    let signature_path = signature_path(path);

    fs::write(
        signature_path,
        format!(
            "key_id = \"{key_id}\"\npublic_key = \"{}\"\nsignature = \"{}\"\n",
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

fn signature_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.sig", path.display()))
}

fn fingerprint_for_key(signing_key: &SigningKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.verifying_key().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn signing_key_a() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn signing_key_b() -> SigningKey {
    SigningKey::from_bytes(&[9_u8; 32])
}
