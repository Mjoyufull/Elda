use super::support::*;
use super::*;

#[test]
fn upgrade_reinstalls_world_package_when_snapshot_version_increases() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-index.toml");
    let v1_binary = create_script_binary(tempdir.path(), "upgrade-tool-v1", "upgrade v1");
    write_remote_index_with_version(&index_path, "upgrade-tool", &v1_binary, "1.0.0");

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
            vec!["upgrade-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/upgrade-tool"),
        "upgrade v1"
    );

    let v2_binary = create_script_binary(tempdir.path(), "upgrade-tool-v2", "upgrade v2");
    write_remote_index_with_version(&index_path, "upgrade-tool", &v2_binary, "1.1.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");

    assert_eq!(report.area, "upgrade");
    assert!(
        report
            .details
            .and_then(|details| details.get("upgrades").cloned())
            .and_then(|upgrades| upgrades.as_array().cloned())
            .is_some_and(|upgrades| upgrades.len() == 1)
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/upgrade-tool"),
        "upgrade v2"
    );
}

#[test]
fn upgrade_installs_new_required_dependency_from_same_snapshot() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-closure-index.toml");
    let app_v1 = create_script_binary(tempdir.path(), "closure-app-v1", "closure app v1");
    write_remote_index_with_version(&index_path, "closure-app", &app_v1, "1.0.0");

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
            vec!["closure-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let app_v2 = create_script_binary(tempdir.path(), "closure-app-v2", "closure app v2");
    let dep_binary = create_vendor_binary(tempdir.path(), "closure-dep");
    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"closure-app\"\nsummary = \"Remote binary package\"\ndescription = \"Snapshot-backed install fixture\"\nasset_url = \"file://{app_binary}\"\nsha256 = \"{app_sha256}\"\npayload_sig = \"{app_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"closure-app\",\n  epoch = 0,\n  version = \"1.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{app_binary}\",\n    sha256 = \"{app_sha256}\",\n    rename = \"closure-app\",\n  }},\n  depends = {{ \"closure-dep\" }},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n\n[[packages]]\npkgname = \"closure-dep\"\nsummary = \"Remote dependency\"\ndescription = \"Dependency introduced by upgrade\"\nasset_url = \"file://{dep_binary}\"\nsha256 = \"{dep_sha256}\"\npayload_sig = \"{dep_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"closure-dep\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{dep_binary}\",\n    sha256 = \"{dep_sha256}\",\n    rename = \"closure-dep\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            app_binary = app_v2.display(),
            app_sha256 = sha256_file(&app_v2),
            app_payload_sig = remote_payload_signature(&app_v2),
            dep_binary = dep_binary.display(),
            dep_sha256 = sha256_file(&dep_binary),
            dep_payload_sig = remote_payload_signature(&dep_binary),
        ),
    )
    .expect("updated remote index should be written");
    sign_remote_index(&index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");
    let upgrades = report
        .details
        .and_then(|details| details.get("upgrades").cloned())
        .and_then(|upgrades| upgrades.as_array().cloned())
        .expect("upgrade report should include applied changes");
    assert!(upgrades.iter().any(|entry| {
        entry.get("target").and_then(|value| value.as_str()) == Some("closure-app")
            && entry.get("action").and_then(|value| value.as_str()) == Some("upgrade-target")
    }));
    assert!(upgrades.iter().any(|entry| {
        entry.get("target").and_then(|value| value.as_str()) == Some("closure-dep")
            && entry.get("action").and_then(|value| value.as_str()) == Some("install-dependency")
    }));
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/closure-app"),
        "closure app v2"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/closure-dep"),
        "binary lane"
    );
}

#[test]
fn upgrade_replaces_installed_package_when_new_candidate_declares_replaces() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-replaces-index.toml");
    let alpha_v1 = create_script_binary(tempdir.path(), "alpha-tool-v1", "alpha v1");
    let beta_v1 = create_script_binary(tempdir.path(), "beta-tool-v1", "beta v1");

    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"alpha-tool\"\nsummary = \"Remote binary package\"\ndescription = \"Replacement fixture\"\nasset_url = \"file://{alpha_binary}\"\nsha256 = \"{alpha_sha256}\"\npayload_sig = \"{alpha_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"alpha-tool\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{alpha_binary}\",\n    sha256 = \"{alpha_sha256}\",\n    rename = \"alpha-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n\n[[packages]]\npkgname = \"beta-tool\"\nsummary = \"Remote binary package\"\ndescription = \"Replacement fixture\"\nasset_url = \"file://{beta_binary}\"\nsha256 = \"{beta_sha256}\"\npayload_sig = \"{beta_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"beta-tool\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{beta_binary}\",\n    sha256 = \"{beta_sha256}\",\n    rename = \"beta-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            alpha_binary = alpha_v1.display(),
            alpha_sha256 = sha256_file(&alpha_v1),
            alpha_payload_sig = remote_payload_signature(&alpha_v1),
            beta_binary = beta_v1.display(),
            beta_sha256 = sha256_file(&beta_v1),
            beta_payload_sig = remote_payload_signature(&beta_v1),
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
            vec!["alpha-tool".to_owned(), "beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("initial install should succeed");

    let beta_v2 = create_script_binary(tempdir.path(), "beta-tool-v2", "beta v2");
    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"beta-tool\"\nsummary = \"Remote binary package\"\ndescription = \"Replacement fixture\"\nasset_url = \"file://{beta_binary}\"\nsha256 = \"{beta_sha256}\"\npayload_sig = \"{beta_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"beta-tool\",\n  epoch = 0,\n  version = \"1.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{beta_binary}\",\n    sha256 = \"{beta_sha256}\",\n    rename = \"beta-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{ \"alpha-tool\" }},\n  replaces = {{ \"alpha-tool\" }},\n  conffiles = {{}},\n}}\n'''\n",
            beta_binary = beta_v2.display(),
            beta_sha256 = sha256_file(&beta_v2),
            beta_payload_sig = remote_payload_signature(&beta_v2),
        ),
    )
    .expect("updated remote index should be written");
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
            vec!["beta-tool".to_owned()],
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
        entry.get("target").and_then(|value| value.as_str()) == Some("beta-tool")
            && entry
                .get("replacements")
                .and_then(|value| value.as_array())
                .is_some_and(|replacements| replacements.len() == 1)
    }));
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/beta-tool"),
        "beta v2"
    );
    assert!(!tempdir.path().join("opt/elda/bin/alpha-tool").exists());
}

#[test]
fn upgrade_does_not_auto_install_new_recommends() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("upgrade-recommends-index.toml");
    let app_v1 = create_script_binary(tempdir.path(), "recommend-app-v1", "recommend app v1");
    write_remote_index_with_version(&index_path, "recommend-app", &app_v1, "1.0.0");

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
            vec!["recommend-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let app_v2 = create_script_binary(tempdir.path(), "recommend-app-v2", "recommend app v2");
    let rec_binary = create_vendor_binary(tempdir.path(), "recommend-extra");
    write_remote_index_with_lua_fields(
        &index_path,
        "recommend-app",
        &app_v2,
        "1.1.0",
        "{}",
        "{ \"recommend-extra\" }",
    );
    fs::write(
        &index_path,
        format!(
            "{}\n[[packages]]\npkgname = \"recommend-extra\"\nsummary = \"Remote recommend\"\ndescription = \"Weak dependency introduced by upgrade\"\nasset_url = \"file://{rec_binary}\"\nsha256 = \"{rec_sha256}\"\npayload_sig = \"{rec_payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"recommend-extra\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{rec_binary}\",\n    sha256 = \"{rec_sha256}\",\n    rename = \"recommend-extra\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
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
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/recommend-app"),
        "recommend app v2"
    );
    assert!(!tempdir.path().join("opt/elda/bin/recommend-extra").exists());
}
