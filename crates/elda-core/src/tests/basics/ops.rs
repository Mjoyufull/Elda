use super::*;

#[test]
fn profile_show_reports_current_profile_shape() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed");

    assert_eq!(report.area, "profile");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| profiles.is_empty())
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("native_arch"))
            .and_then(|arch| arch.as_str())
            .is_some_and(|arch| !arch.is_empty())
    );
}

#[test]
fn profile_set_init_persists_state_and_updates_profile_show() {
    let tempdir = TempDir::new().expect("tempdir should be created");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");

    assert_eq!(report.area, "profile");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "dinit")
    );

    let show = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed");

    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "dinit")
    );
}

#[test]
fn profile_set_init_dry_run_does_not_persist_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("pf set-init dry-run should succeed");

    let show = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed");

    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value != "dinit")
    );
}

#[test]
fn daemon_status_and_refresh_use_current_snapshot_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let binary = create_vendor_binary(tempdir.path(), "daemon-tool");
    let index_path = write_remote_index(tempdir.path(), "daemon-tool", &binary);

    register_fixture_remote(tempdir.path(), "main", &index_path);

    let status_before = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["daemon".to_owned(), "status".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("daemon status should succeed");
    assert_eq!(status_before.area, "daemon");
    assert!(
        !status_before
            .details
            .as_ref()
            .and_then(|details| details.get("snapshot_present"))
            .and_then(|present| present.as_bool())
            .unwrap_or(true)
    );

    let refresh = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["daemon".to_owned(), "refresh".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("daemon refresh should succeed");
    assert_eq!(refresh.area, "daemon");
    assert!(
        refresh
            .details
            .as_ref()
            .and_then(|details| details.get("sync"))
            .and_then(|sync| sync.get("package_count"))
            .and_then(|count| count.as_u64())
            .is_some_and(|count| count == 1)
    );
}

#[test]
fn fix_triggers_reports_no_pending_work_in_current_backend() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fix-triggers".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("fix-triggers should succeed");

    assert_eq!(report.area, "ops");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("pending_handlers"))
            .and_then(|handlers| handlers.as_array())
            .is_some_and(|handlers| handlers.is_empty())
    );
}

#[test]
fn cache_add_and_list_round_trip() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["cache".to_owned(), "add".to_owned()],
            vec![
                "slow=https://cache.invalid/slow".to_owned(),
                "--priority".to_owned(),
                "40".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("cache add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["cache".to_owned(), "add".to_owned()],
            vec![
                "fast=https://cache.invalid/fast".to_owned(),
                "--priority".to_owned(),
                "10".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("cache add should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["cache".to_owned(), "ls".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("cache ls should succeed");

    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("caches"))
            .and_then(|caches| caches.as_array().cloned())
            .is_some_and(|caches| {
                caches.len() == 2
                    && caches[0].get("name").and_then(|value| value.as_str()) == Some("fast")
                    && caches[0].get("priority").and_then(|value| value.as_u64()) == Some(10)
                    && caches[1].get("name").and_then(|value| value.as_str()) == Some("slow")
            })
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("policy"))
            .and_then(|policy| policy.get("effective_trigger_bytes"))
            .and_then(|bytes| bytes.as_u64())
            .is_some_and(|bytes| bytes > 0)
    );
}

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
        local.iter().filter_map(|v| v.as_str()).any(|n| n == "rc-ls-tool"),
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
