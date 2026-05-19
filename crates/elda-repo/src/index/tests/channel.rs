use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::model::{RemoteDocument, TrustMode};

use super::{SyncOptions, load_snapshot, sync_remotes};

#[test]
fn sync_filters_remote_snapshot_to_selected_channel() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path = write_signed_channel_index(
        tempdir.path(),
        "channels",
        &[("stable-tool", None), ("testing-tool", Some("testing"))],
    );
    super::write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            channel: "testing".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![super::fixture_key_fingerprint()],
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );

    let report = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions::default(),
    )
    .expect("sync should succeed");

    assert_eq!(report.package_count, 1);
    assert_eq!(report.remotes[0].channel, "testing");
    let snapshot = load_snapshot(&report.snapshot_path).expect("snapshot should load");
    assert_eq!(snapshot.packages.len(), 1);
    assert_eq!(snapshot.packages[0].pkgname, "testing-tool");
    assert_eq!(snapshot.packages[0].channel.as_deref(), Some("testing"));
}

#[test]
fn offline_sync_rejects_cached_snapshot_for_different_channel() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let index_path =
        write_signed_channel_index(tempdir.path(), "channels", &[("stable-tool", None)]);
    super::write_remote_document(
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
            trusted_keys: vec![super::fixture_key_fingerprint()],
            allow_stale: true,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    sync_remotes(&remotes_dir, &snapshot_path, SyncOptions::default())
        .expect("initial sync should succeed");
    super::write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            channel: "testing".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![super::fixture_key_fingerprint()],
            allow_stale: true,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    fs::remove_file(&index_path).expect("index file should be removed");
    fs::remove_file(index_path.with_extension("toml.sig"))
        .expect("signature file should be removed");

    let error = sync_remotes(
        &remotes_dir,
        &snapshot_path,
        SyncOptions {
            offline: true,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
            ..SyncOptions::default()
        },
    )
    .expect_err("offline sync should reject the wrong cached channel");

    assert!(error
        .to_string()
        .contains("cached snapshot for remote `main` is for channel `stable` but the configured channel is `testing`"));
}

fn write_signed_channel_index(
    root: &Path,
    file_name: &str,
    packages: &[(&str, Option<&str>)],
) -> PathBuf {
    let index_path = root.join(format!("{file_name}.toml"));
    let content = packages
        .iter()
        .map(|(package_name, channel)| {
            let channel_line = channel
                .map(|channel| format!("channel = \"{channel}\"\n"))
                .unwrap_or_default();
            format!(
                "[[packages]]\npkgname = \"{package_name}\"\n{channel_line}pkg_lua = '''\npkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file:///tmp/{package_name}\",\n    sha256 = \"deadbeef\",\n    rename = \"{package_name}\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n"
            )
        })
        .collect::<String>();
    fs::write(&index_path, content).expect("remote index should be written");
    super::write_signature_file(&index_path);
    index_path
}
