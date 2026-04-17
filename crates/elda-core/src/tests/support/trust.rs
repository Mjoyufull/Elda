use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

pub(in crate::tests) fn fixture_remote_signing_key_primary() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

pub(in crate::tests) fn fixture_remote_signing_key_secondary() -> SigningKey {
    SigningKey::from_bytes(&[9_u8; 32])
}

pub(in crate::tests) fn fingerprint_for_signing_key(signing_key: &SigningKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.verifying_key().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(in crate::tests) fn write_signed_remote_index_with_key(
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

pub(in crate::tests) fn write_rotation_metadata_document(
    metadata_path: &Path,
    signing_key: &SigningKey,
    trusted_keys: &[(&str, &SigningKey)],
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

    fs::write(
        format!("{}.sig", path.display()),
        format!(
            "key_id = \"{key_id}\"\npublic_key = \"{}\"\nsignature = \"{}\"\n",
            STANDARD.encode(signing_key.verifying_key().as_bytes()),
            STANDARD.encode(signature.to_bytes()),
        ),
    )
    .expect("signature file should be written");
}
