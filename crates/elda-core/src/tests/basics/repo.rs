use super::*;

#[test]
fn named_install_without_configured_remotes_points_to_remote_bootstrap() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["missing-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("install without configured remotes should fail");

    assert!(error.to_string().contains("no remotes are configured"));
    assert!(
        error
            .to_string()
            .contains("elda rmt add <name>=<index-url>` and then run `elda sync`")
    );
}

#[test]
fn sync_accepts_target_remote_names() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let main_binary = create_vendor_binary(tempdir.path(), "main-only-tool");
    let extra_binary = create_vendor_binary(tempdir.path(), "extra-only-tool");
    let main_index = write_remote_index(tempdir.path(), "main-only-tool", &main_binary);
    let extra_index = write_remote_index(tempdir.path(), "extra-only-tool", &extra_binary);
    register_fixture_remote(tempdir.path(), "main", &main_index);
    register_fixture_remote(tempdir.path(), "extra", &extra_index);

    let sync_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["sync".to_owned()],
            vec!["main".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("targeted sync should succeed");

    let sync = sync_report
        .details
        .as_ref()
        .and_then(|details| details.get("sync"))
        .expect("sync details should exist");
    assert_eq!(
        sync.get("remote_count").and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        sync.get("package_count").and_then(|value| value.as_u64()),
        Some(1)
    );
    assert!(
        sync.get("remotes")
            .and_then(|remotes| remotes.as_array())
            .is_some_and(
                |remotes| remotes[0].get("name").and_then(|name| name.as_str()) == Some("main")
            )
    );
}

#[test]
fn sync_search_info_and_named_install_work_from_repo_snapshot() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "repo-tool");
    let index_path = write_remote_index(tempdir.path(), "repo-tool", &binary);

    register_fixture_remote(tempdir.path(), "main", &index_path);

    let sync_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    assert_eq!(sync_report.area, "sync");
    assert!(
        sync_report
            .details
            .and_then(|details| details.get("sync").cloned())
            .and_then(|sync| sync.get("package_count").cloned())
            .and_then(|count| count.as_u64())
            .is_some_and(|count| count == 1)
    );

    let search_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["search".to_owned()],
            vec!["repo".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("search should succeed");

    assert_eq!(search_report.area, "search");
    assert!(
        search_report
            .details
            .and_then(|details| details.get("results").cloned())
            .and_then(|results| results.as_array().cloned())
            .is_some_and(|results| results.iter().any(|result| {
                result
                    .get("pkgname")
                    .and_then(|pkgname| pkgname.as_str())
                    .is_some_and(|pkgname| pkgname == "repo-tool")
            }))
    );

    let info_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["info".to_owned()],
            vec!["repo-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed");

    assert_eq!(info_report.area, "info");
    assert!(
        info_report
            .details
            .as_ref()
            .and_then(|details| details.get("synced").cloned())
            .and_then(|synced| synced.get("remote_name").cloned())
            .and_then(|remote_name| remote_name.as_str().map(ToOwned::to_owned))
            .is_some_and(|remote_name| remote_name == "main")
    );
    assert!(
        info_report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("declared_provider_assets"))
            .and_then(|assets| assets.as_array())
            .is_some()
    );

    let install_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["repo-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("repo install should succeed");

    assert_eq!(install_report.area, "install");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/repo-tool"),
        "binary lane"
    );
}

#[test]
fn secure_remote_install_rejects_missing_payload_signature() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "unsigned-tool");
    let index_path = tempdir.path().join("unsigned-index.toml");

    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"unsigned-tool\"\nsummary = \"Remote binary package\"\ndescription = \"Missing payload signature fixture\"\nasset_url = \"file://{binary}\"\nsha256 = \"{sha256}\"\npkg_lua = '''\npkg = {{\n  name = \"unsigned-tool\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"unsigned-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            binary = binary.display(),
            sha256 = sha256_file(&binary),
        ),
    )
    .expect("remote index should be written");
    sign_remote_index(&index_path);

    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["unsigned-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("secure remote install should fail without payload signature");

    assert!(
        error
            .to_string()
            .contains("missing `payload_sig` for this package")
    );
}

#[test]
fn synced_install_uses_configured_cache_before_origin_asset() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_script_binary(tempdir.path(), "cache-hit-tool", "cache hit");
    let index_path = write_remote_index(tempdir.path(), "cache-hit-tool", &binary);
    let cache_dir = tempdir.path().join("cache-node");
    fs::create_dir_all(&cache_dir).expect("cache dir should exist");
    fs::copy(&binary, cache_dir.join(sha256_file(&binary))).expect("cache payload should copy");

    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["cache".to_owned(), "add".to_owned()],
            vec![
                format!("lan=file://{}", cache_dir.display()),
                "--priority".to_owned(),
                "20".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("cache add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    fs::remove_file(&binary).expect("origin payload should be removed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["cache-hit-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("cache-backed install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/cache-hit-tool"),
        "cache hit"
    );
}

#[test]
fn offline_reinstall_uses_local_cached_payload() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_script_binary(tempdir.path(), "offline-cache-tool", "offline cache");
    let index_path = write_remote_index(tempdir.path(), "offline-cache-tool", &binary);

    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["offline-cache-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("initial install should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["offline-cache-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");
    fs::remove_file(&binary).expect("origin payload should be removed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["offline-cache-tool".to_owned()],
            OutputMode::Json,
            false,
        )
        .with_offline(true),
    )
    .expect("offline reinstall should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/offline-cache-tool"),
        "offline cache"
    );
}

#[test]
fn json_sync_rejects_first_use_tofu_enrollment() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let binary = create_vendor_binary(tempdir.path(), "tofu-json-tool");
    let index_path = write_remote_index(tempdir.path(), "tofu-json-tool", &binary);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![format!("main=file://{}", index_path.display())],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect_err("json sync should reject first-use tofu");

    assert!(
        error
            .to_string()
            .contains("requires explicit trust bootstrap")
    );
}

#[test]
fn human_sync_can_bootstrap_tofu_then_json_sync_reuses_trust() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let binary = create_vendor_binary(tempdir.path(), "tofu-human-tool");
    let index_path = write_remote_index(tempdir.path(), "tofu-human-tool", &binary);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![format!("main=file://{}", index_path.display())],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["sync".to_owned()],
            Vec::new(),
            OutputMode::Human,
            false,
        ),
    )
    .expect("human sync should bootstrap tofu");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("json sync should reuse trusted tofu state");

    assert_eq!(report.area, "sync");
}
