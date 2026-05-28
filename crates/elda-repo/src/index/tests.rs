use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{SyncOptions, load_snapshot, sync_remotes};
use crate::model::RemoteDocument;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

mod channel;
mod interemote;
mod signed;
mod signed_cleanup;

pub(super) fn command_exists(tool: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {tool} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(super) fn make_git_repo(repo: &Path) {
    run_git(repo, &["init", "-b", "main"]);
    run_git(repo, &["config", "user.email", "elda@example.invalid"]);
    run_git(repo, &["config", "user.name", "Elda Tests"]);
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", "initial"]);
}

pub(super) fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should launch");
    assert!(
        status.success(),
        "git {args:?} failed in {}",
        repo.display()
    );
}

pub(super) fn write_signed_remote_index(root: &Path, package_name: &str) -> PathBuf {
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

pub(super) fn write_signature_file(index_path: &Path) {
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

pub(super) fn write_remote_document(remotes_dir: &Path, remote: RemoteDocument) {
    fs::write(
        remotes_dir.join(format!("{}.toml", remote.name)),
        toml::to_string_pretty(&remote).expect("remote toml should encode"),
    )
    .expect("remote document should be written");
}

pub(super) fn fixture_key_fingerprint() -> String {
    let mut hasher = Sha256::new();
    hasher.update(fixture_signing_key().verifying_key().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(super) fn fixture_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}
