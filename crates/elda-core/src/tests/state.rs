use super::support::*;
use super::*;

#[test]
fn state_export_reports_desired_machine_shape() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "state-tool");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec!["main=https://example.invalid/index.toml".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["vendor".to_owned(), "add".to_owned()],
            vec!["state-tool".to_owned(), binary.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("vendor add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["state-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pin".to_owned()],
            vec!["state-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pin should succeed");

    let export_path = tempdir.path().join("machine.eldastate");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "export".to_owned()],
            vec![export_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("state export should succeed");

    assert_eq!(report.area, "state");
    let exported = fs::read_to_string(&export_path).expect("exported state should exist");
    let parsed: serde_json::Value =
        serde_json::from_str(&exported).expect("exported json should parse");
    assert!(
        parsed
            .get("world")
            .and_then(|world| world.as_array())
            .is_some_and(|world| world
                .iter()
                .any(|entry| { entry.as_str().is_some_and(|entry| entry == "state-tool") }))
    );
    assert!(
        parsed
            .get("remotes")
            .and_then(|remotes| remotes.as_array())
            .is_some_and(|remotes| remotes.iter().any(|remote| {
                remote
                    .get("name")
                    .and_then(|name| name.as_str())
                    .is_some_and(|name| name == "main")
            }))
    );
}

#[test]
fn state_export_uses_persisted_profile_base_after_profile_apply() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let profile_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");

    write_local_profile_recipe(tempdir.path(), "yoka-core", &profile_repo, &[]);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec![
                "yoka-core".to_owned(),
                "--init".to_owned(),
                "dinit".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed");

    let export_path = tempdir.path().join("profile.eldastate");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "export".to_owned()],
            vec![export_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("state export should succeed");

    let exported = fs::read_to_string(&export_path).expect("exported state should exist");
    let parsed: serde_json::Value =
        serde_json::from_str(&exported).expect("exported json should parse");

    assert_eq!(
        parsed
            .get("profile")
            .and_then(|profile| profile.get("base"))
            .and_then(|base| base.as_str()),
        Some("yoka-core")
    );
    assert_eq!(
        parsed
            .get("profile")
            .and_then(|profile| profile.get("init"))
            .and_then(|init| init.as_str()),
        Some("dinit")
    );
}

#[test]
fn state_import_writes_remotes_and_installs_world_targets() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let import_base_repo = create_git_make_repo(tempdir.path(), "import-base-source");
    let import_desktop_repo = create_git_make_repo(tempdir.path(), "import-desktop-source");
    let binary = create_script_binary(tempdir.path(), "import-tool-v1", "import tool");
    let index_path = write_remote_index(tempdir.path(), "import-tool", &binary);
    let state_path = tempdir.path().join("import.eldastate");
    write_local_profile_recipe(tempdir.path(), "import-base", &import_base_repo, &[]);
    write_local_profile_recipe(tempdir.path(), "import-desktop", &import_desktop_repo, &[]);
    fs::write(
        &state_path,
        format!(
            "{{\n  \"format_version\": 1,\n  \"exported_at\": \"123\",\n  \"installation_mode\": \"prefix\",\n  \"prefix\": \"/opt/elda\",\n  \"profile\": {{\n    \"active_profiles\": [\"import-base\", \"import-desktop\"],\n    \"base\": \"import-base\",\n    \"native_arch\": \"amd64\",\n    \"foreign_arches\": [\"i386\"],\n    \"init\": \"dinit\"\n  }},\n  \"remotes\": [{{\n    \"name\": \"main\",\n    \"index_url\": \"file://{}\",\n    \"metadata_url\": null,\n    \"enabled\": true,\n    \"trust\": \"pinned\",\n    \"trusted_keys\": [\"{}\"],\n    \"priority\": 100\n  }}],\n  \"world\": [\"import-tool\"],\n  \"installed\": []\n}}\n",
            index_path.display(),
            fixture_remote_key_fingerprint(),
        ),
    )
    .expect("state import fixture should be written");

    let dry_run_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "import".to_owned()],
            vec![state_path.display().to_string()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("state import dry-run should succeed");
    assert_eq!(dry_run_report.area, "plan");

    let import_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "import".to_owned()],
            vec![state_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("state import should succeed");

    assert_eq!(import_report.area, "state");
    assert!(
        tempdir.path().join("etc/elda/remotes.d/main.toml").exists(),
        "remote document should be written"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/import-tool"),
        "import tool"
    );
    let profile_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed after state import");
    assert!(
        profile_report
            .details
            .as_ref()
            .and_then(|details| details.get("active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| profiles
                .iter()
                .any(|profile| profile.as_str() == Some("import-base"))
                && profiles
                    .iter()
                    .any(|profile| profile.as_str() == Some("import-desktop")))
    );
    assert_eq!(
        profile_report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|init| init.as_str()),
        Some("dinit")
    );
}
