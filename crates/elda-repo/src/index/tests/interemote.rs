use std::fs;

use tempfile::TempDir;

use crate::model::{RemoteDocument, TrustMode};

use super::super::{SyncOptions, load_snapshot, preview_interemote, sync_remotes};
use super::{command_exists, make_git_repo, run_git, write_remote_document};

#[test]
fn preview_interemote_reports_catalog_and_excludes() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    if !command_exists("git") {
        return;
    }
    let overlay = tempdir.path().join("overlay");
    fs::create_dir_all(overlay.join("profiles")).expect("profiles dir should exist");
    fs::write(overlay.join("profiles/repo_name"), "overlay\n")
        .expect("repo_name should be written");
    let package_dir = overlay.join("app-misc/demo");
    fs::create_dir_all(&package_dir).expect("package dir should exist");
    fs::write(
        package_dir.join("demo-1.2.3.ebuild"),
        "EAPI=\"8\"\nDESCRIPTION=\"Demo app\"\nHOMEPAGE=\"https://example.invalid/demo\"\nLICENSE=\"MIT\"\nRDEPEND=\"dev-libs/example\"\n",
    )
    .expect("ebuild should be written");
    let excluded_dir = overlay.join("app-misc/skipme");
    fs::create_dir_all(&excluded_dir).expect("excluded package dir should exist");
    fs::write(
        excluded_dir.join("skipme-1.ebuild"),
        "EAPI=\"8\"\nDESCRIPTION=\"Skipped app\"\n",
    )
    .expect("excluded ebuild should be written");
    let invalid_dir = overlay.join("app-misc/zz-broken");
    fs::create_dir_all(&invalid_dir).expect("invalid package dir should exist");
    fs::write(invalid_dir.join("zz-broken-1.ebuild"), [0xff, 0xfe])
        .expect("invalid ebuild should be written");
    make_git_repo(&overlay);

    let preview = preview_interemote(&RemoteDocument {
        name: "overlay".to_owned(),
        index_url: overlay.display().to_string(),
        channel: "stable".to_owned(),
        packages_url: None,
        metadata_url: None,
        signature_url: None,
        enabled: true,
        trust: TrustMode::Tofu,
        trusted_keys: Vec::new(),
        allow_stale: false,
        exclude: vec!["skipme".to_owned()],
        priority: 100,
    })
    .expect("preview should succeed");

    assert_eq!(preview.kind, "gentoo_overlay");
    assert_eq!(preview.source_kind, "gentoo_overlay");
    assert_eq!(preview.discovered_count, 3);
    assert_eq!(preview.included_count, 2);
    assert_eq!(preview.excluded_count, 1);
    assert_eq!(preview.matched_excludes, vec!["skipme"]);
    assert_eq!(preview.packages[0].name, "demo");
    assert_eq!(preview.packages[0].version.as_deref(), Some("1.2.3"));
    assert_eq!(preview.packages[0].summary.as_deref(), Some("Demo app"));
    assert_eq!(preview.parseable_count, 1);
    assert_eq!(preview.issues.len(), 1);
    assert_eq!(preview.issues[0].name, "zz-broken");
}

#[test]
fn sync_reports_removed_packages_from_previous_snapshot() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    if !command_exists("git") {
        return;
    }
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let overlay = tempdir.path().join("delta-overlay");
    fs::create_dir_all(overlay.join("profiles")).expect("profiles dir should exist");
    fs::write(overlay.join("profiles/repo_name"), "delta\n").expect("repo_name should be written");
    let alpha_dir = overlay.join("app-misc/alpha");
    fs::create_dir_all(&alpha_dir).expect("alpha dir should exist");
    fs::write(
        alpha_dir.join("alpha-1.ebuild"),
        "EAPI=\"8\"\nDESCRIPTION=\"Alpha\"\n",
    )
    .expect("alpha ebuild should be written");
    let beta_dir = overlay.join("app-misc/beta");
    fs::create_dir_all(&beta_dir).expect("beta dir should exist");
    fs::write(
        beta_dir.join("beta-1.ebuild"),
        "EAPI=\"8\"\nDESCRIPTION=\"Beta\"\n",
    )
    .expect("beta ebuild should be written");
    make_git_repo(&overlay);
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "delta".to_owned(),
            index_url: overlay.display().to_string(),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    let snapshot_path = tempdir.path().join("repo-snapshot.json");

    let first = sync_remotes(&remotes_dir, &snapshot_path, SyncOptions::default())
        .expect("initial interemote sync should succeed");
    assert_eq!(first.package_count, 2);
    assert_eq!(first.package_deltas[0].added_count, 2);

    fs::remove_dir_all(&beta_dir).expect("beta package should be removed");
    run_git(&overlay, &["add", "-A"]);
    run_git(&overlay, &["commit", "-m", "remove beta"]);

    let second = sync_remotes(&remotes_dir, &snapshot_path, SyncOptions::default())
        .expect("second interemote sync should succeed");
    let delta = second
        .package_deltas
        .iter()
        .find(|delta| delta.remote_name == "delta")
        .expect("delta remote package delta should exist");

    assert_eq!(second.package_count, 1);
    assert_eq!(delta.previous_count, 2);
    assert_eq!(delta.current_count, 1);
    assert_eq!(delta.removed_count, 1);
    assert_eq!(delta.removed_packages, vec!["beta"]);
    assert_eq!(delta.kept_count, 1);
}

#[test]
fn sync_all_failed_error_distinguishes_index_and_interemote_failures() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    if !command_exists("git") {
        return;
    }
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");

    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "bad-index".to_owned(),
            index_url: format!("file://{}", tempdir.path().join("missing.toml").display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Insecure,
            trusted_keys: Vec::new(),
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "bad-overlay".to_owned(),
            index_url: tempdir.path().join("missing-overlay").display().to_string(),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            exclude: Vec::new(),
            priority: 100,
        },
    );

    let error = sync_remotes(
        &remotes_dir,
        &tempdir.path().join("repo-snapshot.json"),
        SyncOptions::default(),
    )
    .expect_err("all failed sync should explain every remote class");
    let message = error.to_string();

    assert!(message.contains("sync produced no usable packages from 2 enabled remote(s)"));
    assert!(message.contains("index sync failed for `bad-index`"));
    assert!(message.contains("interemote sync failed for `bad-overlay`"));
    assert!(message.contains("previous snapshot was left unchanged"));
}

#[test]
fn sync_interemote_gentoo_overlay_generates_interbuild_recipes() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    if !command_exists("git") {
        return;
    }
    let remotes_dir = tempdir.path().join("remotes.d");
    fs::create_dir_all(&remotes_dir).expect("remotes dir should exist");
    let overlay = tempdir.path().join("heather-overlay");
    fs::create_dir_all(overlay.join("profiles")).expect("profiles dir should exist");
    fs::write(overlay.join("profiles/repo_name"), "heather\n")
        .expect("repo_name should be written");
    let package_dir = overlay.join("gui-apps/foot");
    fs::create_dir_all(&package_dir).expect("package dir should exist");
    fs::write(
        package_dir.join("foot-9999.ebuild"),
        r#"EAPI="8"

inherit meson systemd xdg git-r3

DESCRIPTION="Wayland terminal"
HOMEPAGE="https://codeberg.org/dnkl/foot"
EGIT_REPO_URI="https://codeberg.org/dnkl/foot.git"
LICENSE="MIT"
SLOT="0"
IUSE="+grapheme-clustering test utempter"
RDEPEND="dev-libs/wayland"
BDEPEND="dev-build/meson"
"#,
    )
    .expect("ebuild should be written");
    let invalid_dir = overlay.join("gui-apps/broken");
    fs::create_dir_all(&invalid_dir).expect("invalid package dir should exist");
    fs::write(invalid_dir.join("broken-1.ebuild"), [0xff, 0xfe])
        .expect("invalid ebuild should be written");
    make_git_repo(&overlay);
    write_remote_document(
        &remotes_dir,
        RemoteDocument {
            name: "heather".to_owned(),
            index_url: overlay.display().to_string(),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
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
    .expect("interemote sync should succeed");

    assert_eq!(report.package_count, 1);
    assert_eq!(report.remotes[0].source, "interemote");
    assert_eq!(report.interemotes.len(), 1);
    assert_eq!(report.interemotes[0].remote_name, "heather");
    assert_eq!(report.interemotes[0].kind, "gentoo_overlay");
    assert_eq!(report.interemotes[0].included_count, 2);
    assert_eq!(report.interemotes[0].parseable_count, 1);
    assert_eq!(report.interemotes[0].issues.len(), 1);
    assert_eq!(report.interemotes[0].issues[0].name, "broken");
    assert!(
        report.remotes[0]
            .issue
            .as_deref()
            .unwrap_or("")
            .contains("skipped 1")
    );
    assert_eq!(report.interemotes[0].packages[0].name, "broken");
    assert_eq!(report.interemotes[0].packages[1].name, "foot");
    let snapshot = load_snapshot(&report.snapshot_path).expect("snapshot should load");
    let package = &snapshot.packages[0];
    assert_eq!(package.pkgname, "foot");
    assert_eq!(package.source_kind.as_deref(), Some("interemote"));
    assert!(package.pkg_lua.contains("kind = \"gentoo_overlay\""));
    assert!(package.pkg_lua.contains("package = \"gui-apps/foot\""));
    assert!(package.pkg_lua.contains("[\"grapheme-clustering\"] = true"));
    assert!(package.pkg_lua.contains("[\"utempter\"] = true"));
}
