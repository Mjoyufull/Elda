use super::*;

#[test]
fn host_scan_tree_reports_missing_metadata() {
    let tempdir = TempDir::new().expect("tempdir");
    let packages = tempdir.path().join("packages").join("demo");
    fs::create_dir_all(&packages).expect("packages dir");
    fs::write(
        packages.join("pkg.lua"),
        r#"
return {
  pkg = {
    name = "demo",
    version = "1.0.0",
    rel = 1,
    arch = { "x86_64" },
    source = { kind = "url_archive", url = "https://example.invalid/demo.tar.gz" },
  },
}
"#,
    )
    .expect("pkg.lua");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["host".to_owned(), "scan-tree".to_owned()],
            vec![tempdir.path().to_string_lossy().into_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("scan-tree should run");

    assert_eq!(report.area, "host");
    assert_eq!(report.status, "issues");
}

#[test]
fn host_profile_loads_from_host_d() {
    let tempdir = TempDir::new().expect("tempdir");
    let host_dir = tempdir.path().join("etc").join("elda").join("host.d");
    fs::create_dir_all(&host_dir).expect("host.d");
    fs::write(
        host_dir.join("yoka.toml"),
        r#"
[host]
profile = "yoka"
tree = "/tmp/yoka-pkgs"
default_channel = "stable"

[host.channels.stable]
branch = "main"
index_subpath = "stable"
"#,
    )
    .expect("profile");

    let profile =
        crate::host_config::load_host_profile(tempdir.path(), Some("yoka")).expect("profile");
    assert_eq!(profile.name, "yoka");
    assert_eq!(profile.default_channel(), "stable");
    assert_eq!(profile.channel_branch("stable"), "main");
}
