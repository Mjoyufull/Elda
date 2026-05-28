use super::*;

#[test]
fn remote_add_persists_remote_document() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                "https://example.invalid/yoka-main.toml".to_owned(),
                "--priority".to_owned(),
                "20".to_owned(),
                "--channel".to_owned(),
                "stable-7d".to_owned(),
                "--metadata-url".to_owned(),
                "https://example.invalid/remote-metadata-v1.toml".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");

    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("remote").cloned())
            .and_then(|remote| remote.get("name").cloned())
            .and_then(|name| name.as_str().map(ToOwned::to_owned))
            .is_some_and(|name| name == "yoka-main")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("remote").cloned())
            .and_then(|remote| remote.get("priority").cloned())
            .and_then(|priority| priority.as_u64())
            .is_some_and(|priority| priority == 20)
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("remote").cloned())
            .and_then(|remote| remote.get("channel").cloned())
            .and_then(|channel| channel.as_str().map(ToOwned::to_owned))
            .is_some_and(|channel| channel == "stable-7d")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("remote").cloned())
            .and_then(|remote| remote.get("metadata_url").cloned())
            .and_then(|metadata_url| metadata_url.as_str().map(ToOwned::to_owned))
            .is_some_and(
                |metadata_url| metadata_url == "https://example.invalid/remote-metadata-v1.toml"
            )
    );
}

#[test]
fn remote_add_dry_run_does_not_persist_remote_document() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec!["https://example.invalid/yoka-main.toml".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("remote add dry-run should succeed");

    assert_eq!(report.status, "planned");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("registered"))
            .and_then(|registered| registered.as_bool())
            .is_some_and(|registered| !registered)
    );
    let remotes_dir = tempdir.path().join("var/lib/elda/remotes");
    assert!(
        !remotes_dir.join("yoka-main.toml").is_file(),
        "dry-run should not write remote config"
    );
}

#[test]
fn rc_edit_dry_run_reports_recipe_path_without_launching_editor() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "rc-edit-tool");
    write_local_binary_recipe(tempdir.path(), "rc-edit-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rc".to_owned(), "edit".to_owned()],
            vec!["rc-edit-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("rc edit dry-run should succeed");

    assert_eq!(report.area, "recipe");
    assert_eq!(report.status, "planned");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("recipe"))
            .and_then(|recipe| recipe.get("path"))
            .and_then(|path| path.as_str())
            .is_some_and(|path| path.ends_with("/etc/elda/recipes/rc-edit-tool"))
    );
}

#[test]
fn rc_ls_reports_local_recipe_name() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "rc-ls-tool");
    write_local_binary_recipe(tempdir.path(), "rc-ls-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rc".to_owned(), "ls".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("rc ls should succeed");

    assert_eq!(report.area, "recipe");
    let local = report
        .details
        .as_ref()
        .and_then(|details| details.get("catalog"))
        .and_then(|catalog| catalog.get("local_recipes"))
        .and_then(|value| value.as_array())
        .expect("local_recipes array");
    assert!(
        local
            .iter()
            .filter_map(|v| v.as_str())
            .any(|n| n == "rc-ls-tool"),
        "expected rc-ls-tool in local_recipes: {local:?}"
    );
}

#[test]
fn cache_add_writes_session_log_for_mutating_command() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["cache".to_owned(), "add".to_owned()],
            vec!["fast=https://cache.invalid/fast".to_owned()],
            OutputMode::Json,
            false,
        )
        .with_log_level(Some(3)),
    )
    .expect("cache add should succeed");

    let log_path = report
        .details
        .as_ref()
        .and_then(|details| details.get("session_log"))
        .and_then(|session_log| session_log.get("path"))
        .and_then(|value| value.as_str())
        .expect("session log path should be attached");
    let log_content = fs::read_to_string(log_path).expect("session log should be readable");

    assert!(log_path.contains("/.config/elda/logs/"));
    assert!(log_content.contains("result = success"));
    assert!(log_content.contains("command_path = cache add"));
    assert!(log_content.contains("[config]"));
}
