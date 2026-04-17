use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use super::github::{current_arch_aliases, current_os_aliases, detect_release_asset};
use super::model::{GitHubReleaseAsset, GitHubReleaseResponse, VendorLockFile};
use super::{add_vendor_recipe, export_vendor_source, import_vendor_source};

#[test]
fn vendor_add_from_local_binary_writes_url_archive_recipe() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes_dir = tempdir.path().join("recipes");
    let binary = tempdir.path().join("demo-bin");
    fs::write(&binary, "#!/bin/sh\necho demo\n").expect("binary should be written");
    let mut permissions = fs::metadata(&binary)
        .expect("metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&binary, permissions).expect("permissions should be set");

    let report = add_vendor_recipe(
        &recipes_dir,
        "demo-bin",
        &binary.to_string_lossy(),
        None,
        None,
    )
    .expect("vendor add should succeed");
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should be readable");

    assert_eq!(report.source_kind, "url_archive");
    assert!(pkg_lua.contains("kind = \"url_archive\""));
    assert!(pkg_lua.contains("rename = \"demo-bin\""));
}

#[test]
fn vendor_manifest_import_and_json_export_round_trip() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes_dir = tempdir.path().join("recipes");
    let binary = tempdir.path().join("demo-bin");
    fs::write(&binary, "#!/bin/sh\necho demo\n").expect("binary should be written");
    let manifest = tempdir.path().join("vendor.manifest");
    fs::write(&manifest, format!("demo-bin {} \n", binary.display()))
        .expect("manifest should be written");

    let import_report =
        import_vendor_source(&recipes_dir, &manifest).expect("vendor import should succeed");
    assert_eq!(import_report.packages.len(), 1);

    let lock_path = tempdir.path().join("vendor.lock.json");
    let export_report = export_vendor_source(&recipes_dir, &lock_path, &[String::from("demo-bin")])
        .expect("vendor export should succeed");
    assert_eq!(export_report.format, "lock-json");

    let lock = serde_json::from_str::<VendorLockFile>(
        &fs::read_to_string(lock_path).expect("lock should be readable"),
    )
    .expect("lock json should parse");
    assert_eq!(lock.entries.len(), 1);
    assert_eq!(lock.entries[0].package_name, "demo-bin");
    assert_eq!(lock.entries[0].source_kind, "url_archive");
}

#[test]
fn github_release_asset_detection_prefers_current_platform_payload() {
    let release = GitHubReleaseResponse {
        tag_name: "v1.0.0".to_owned(),
        assets: vec![
            asset(&format!(
                "tool-{}-{}.tar.xz",
                current_os_aliases()[0],
                current_arch_aliases()[0]
            )),
            asset("tool.sha256"),
            asset("tool-linux-arm64.tar.xz"),
        ],
    };

    let selected = detect_release_asset(&release).expect("asset detection should succeed");

    assert_eq!(
        selected.name,
        format!(
            "tool-{}-{}.tar.xz",
            current_os_aliases()[0],
            current_arch_aliases()[0]
        )
    );
}

#[test]
fn github_release_asset_detection_rejects_ambiguous_matches() {
    let release = GitHubReleaseResponse {
        tag_name: "v1.0.0".to_owned(),
        assets: vec![
            asset(&format!(
                "tool-{}-{}.tar.xz",
                current_os_aliases()[0],
                current_arch_aliases()[0]
            )),
            asset(&format!(
                "tool-portable-{}-{}.tar.xz",
                current_os_aliases()[0],
                current_arch_aliases()[0]
            )),
        ],
    };

    let error = detect_release_asset(&release).expect_err("match should be ambiguous");

    assert!(error.to_string().contains("--asset <name>"));
}

fn asset(name: &str) -> GitHubReleaseAsset {
    GitHubReleaseAsset {
        name: name.to_owned(),
        browser_download_url: format!("https://example.invalid/{name}"),
    }
}
