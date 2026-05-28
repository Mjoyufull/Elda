use super::support::*;
use super::*;

#[test]
fn upgrade_refreshes_new_recommends_when_requested_by_flag() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-refresh-weak-index.toml");
    let app_v1 = create_script_binary(tempdir.path(), "refresh-app-v1", "refresh app v1");
    write_remote_index_with_version(&index_path, "refresh-app", &app_v1, "1.0.0");

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
            vec!["refresh-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let app_v2 = create_script_binary(tempdir.path(), "refresh-app-v2", "refresh app v2");
    let rec_binary = create_vendor_binary(tempdir.path(), "refresh-extra");
    write_remote_index_with_lua_fields(
        &index_path,
        "refresh-app",
        &app_v2,
        "1.1.0",
        "{}",
        "{ \"refresh-extra\" }",
    );
    fs::write(
        &index_path,
        format!(
            "{}\n[[packages]]\npkgname = \"refresh-extra\"\nsummary = \"Remote recommend\"\ndescription = \"Weak dependency introduced by upgrade\"\nasset_url = \"file://{rec_binary}\"\nsha256 = \"{rec_sha256}\"\npayload_sig = \"{rec_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"refresh-extra\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{rec_binary}\",\n    sha256 = \"{rec_sha256}\",\n    rename = \"refresh-extra\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            fs::read_to_string(&index_path).expect("updated index should exist"),
            rec_binary = rec_binary.display(),
            rec_sha256 = sha256_file(&rec_binary),
            rec_payload_sig = remote_payload_signature(&rec_binary),
        ),
    )
    .expect("remote index with recommend should be written");
    sign_remote_index(&index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["refresh-app".to_owned(), "--refresh-weak-deps".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("upgrade should succeed");
    let upgrades = report
        .details
        .and_then(|details| details.get("upgrades").cloned())
        .and_then(|upgrades| upgrades.as_array().cloned())
        .expect("upgrade report should include applied changes");

    assert!(upgrades.iter().any(|entry| {
        entry.get("target").and_then(|value| value.as_str()) == Some("refresh-extra")
            && entry.get("action").and_then(|value| value.as_str()) == Some("install-dependency")
    }));
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/refresh-app"),
        "refresh app v2"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/refresh-extra"),
        "binary lane"
    );
}

#[test]
fn upgrade_refreshes_new_recommends_when_enabled_in_config() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_policy(tempdir.path(), "/opt/elda", true, true);
    let index_path = tempdir.path().join("upgrade-config-weak-index.toml");
    let app_v1 = create_script_binary(tempdir.path(), "config-app-v1", "config app v1");
    write_remote_index_with_version(&index_path, "config-app", &app_v1, "1.0.0");

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
            vec!["config-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let app_v2 = create_script_binary(tempdir.path(), "config-app-v2", "config app v2");
    let rec_binary = create_vendor_binary(tempdir.path(), "config-extra");
    write_remote_index_with_lua_fields(
        &index_path,
        "config-app",
        &app_v2,
        "1.1.0",
        "{}",
        "{ \"config-extra\" }",
    );
    fs::write(
        &index_path,
        format!(
            "{}\n[[packages]]\npkgname = \"config-extra\"\nsummary = \"Remote recommend\"\ndescription = \"Weak dependency introduced by upgrade\"\nasset_url = \"file://{rec_binary}\"\nsha256 = \"{rec_sha256}\"\npayload_sig = \"{rec_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"config-extra\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{rec_binary}\",\n    sha256 = \"{rec_sha256}\",\n    rename = \"config-extra\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            fs::read_to_string(&index_path).expect("updated index should exist"),
            rec_binary = rec_binary.display(),
            rec_sha256 = sha256_file(&rec_binary),
            rec_payload_sig = remote_payload_signature(&rec_binary),
        ),
    )
    .expect("remote index with recommend should be written");
    sign_remote_index(&index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["config-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("upgrade should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/config-app"),
        "config app v2"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/config-extra"),
        "binary lane"
    );
}

#[test]
fn targeted_upgrade_fails_when_installed_reverse_dependency_would_break() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-coherence-index.toml");
    let lib_v1 = create_script_binary(tempdir.path(), "libcore-v1", "libcore v1");
    let app_v1 = create_script_binary(tempdir.path(), "app-v1", "app v1");

    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"libcore\"\nsummary = \"Remote dependency\"\ndescription = \"Reverse dependency fixture\"\nasset_url = \"file://{lib_binary}\"\nsha256 = \"{lib_sha256}\"\npayload_sig = \"{lib_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"libcore\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{lib_binary}\",\n    sha256 = \"{lib_sha256}\",\n    rename = \"libcore\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n\n[[packages]]\npkgname = \"app\"\nsummary = \"Remote application\"\ndescription = \"Reverse dependency fixture\"\nasset_url = \"file://{app_binary}\"\nsha256 = \"{app_sha256}\"\npayload_sig = \"{app_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"app\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{app_binary}\",\n    sha256 = \"{app_sha256}\",\n    rename = \"app\",\n  }},\n  depends = {{ \"libcore<2\" }},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            lib_binary = lib_v1.display(),
            lib_sha256 = sha256_file(&lib_v1),
            lib_payload_sig = remote_payload_signature(&lib_v1),
            app_binary = app_v1.display(),
            app_sha256 = sha256_file(&app_v1),
            app_payload_sig = remote_payload_signature(&app_v1),
        ),
    )
    .expect("initial remote index should be written");
    sign_remote_index(&index_path);

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
            vec!["app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let lib_v2 = create_script_binary(tempdir.path(), "libcore-v2", "libcore v2");
    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"libcore\"\nsummary = \"Remote dependency\"\ndescription = \"Reverse dependency fixture\"\nasset_url = \"file://{lib_binary}\"\nsha256 = \"{lib_sha256}\"\npayload_sig = \"{lib_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"libcore\",\n  epoch = 0,\n  version = \"2.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{lib_binary}\",\n    sha256 = \"{lib_sha256}\",\n    rename = \"libcore\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n\n[[packages]]\npkgname = \"app\"\nsummary = \"Remote application\"\ndescription = \"Reverse dependency fixture\"\nasset_url = \"file://{app_binary}\"\nsha256 = \"{app_sha256}\"\npayload_sig = \"{app_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"app\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{app_binary}\",\n    sha256 = \"{app_sha256}\",\n    rename = \"app\",\n  }},\n  depends = {{ \"libcore<2\" }},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            lib_binary = lib_v2.display(),
            lib_sha256 = sha256_file(&lib_v2),
            lib_payload_sig = remote_payload_signature(&lib_v2),
            app_binary = app_v1.display(),
            app_sha256 = sha256_file(&app_v1),
            app_payload_sig = remote_payload_signature(&app_v1),
        ),
    )
    .expect("updated remote index should be written");
    sign_remote_index(&index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["libcore".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("coherence-breaking targeted upgrade should fail");

    assert!(
        error
            .to_string()
            .contains("installed package `app` requires `libcore<2`")
    );
}
