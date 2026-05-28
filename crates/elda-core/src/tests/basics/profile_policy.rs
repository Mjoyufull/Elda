use super::*;

#[test]
fn profile_apply_uses_declared_profile_policy_defaults() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let base_tool = create_vendor_binary(tempdir.path(), "base-tool");
    let profile_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");

    write_local_binary_recipe(tempdir.path(), "base-tool", &base_tool, &[]);
    write_local_profile_recipe_with_policy(
        tempdir.path(),
        "yoka-core",
        &profile_repo,
        &["base-tool"],
        Some("amd64"),
        &["i386"],
        Some("dinit"),
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec!["yoka-core".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed");

    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_native_arch"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "amd64")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "dinit")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_foreign_arches"))
            .and_then(|value| value.as_array())
            .is_some_and(|arches| arches.len() == 1 && arches[0].as_str() == Some("i386"))
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
            .and_then(|details| details.get("declared_profile_policy"))
            .and_then(|policy| policy.get("declared_by"))
            .and_then(|entries| entries.as_array())
            .is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry.get("profile").and_then(|value| value.as_str()) == Some("yoka-core")
                        && entry.get("init").and_then(|value| value.as_str()) == Some("dinit")
                })
            })
    );
}

#[test]
fn profile_apply_cli_overrides_declared_profile_policy() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let profile_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");

    write_local_profile_recipe_with_policy(
        tempdir.path(),
        "yoka-core",
        &profile_repo,
        &[],
        Some("amd64"),
        &["i386"],
        Some("dinit"),
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec![
                "yoka-core".to_owned(),
                "--init".to_owned(),
                "openrc".to_owned(),
                "--native-arch".to_owned(),
                "arm64".to_owned(),
                "--foreign-arch".to_owned(),
                "armhf".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed");

    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_native_arch"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "arm64")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "openrc")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_foreign_arches"))
            .and_then(|value| value.as_array())
            .is_some_and(|arches| arches.len() == 1 && arches[0].as_str() == Some("armhf"))
    );
}

#[test]
fn profile_add_and_remove_update_active_anchor_set() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let core_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");
    let desktop_repo = create_git_make_repo(tempdir.path(), "yoka-desktop-source");

    write_local_profile_recipe_with_policy(
        tempdir.path(),
        "yoka-core",
        &core_repo,
        &[],
        None,
        &[],
        Some("dinit"),
    );
    write_local_profile_recipe_with_policy(
        tempdir.path(),
        "yoka-desktop",
        &desktop_repo,
        &[],
        None,
        &["i386"],
        None,
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec!["yoka-core".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "add".to_owned()],
            vec!["yoka-desktop".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "rm".to_owned()],
            vec!["yoka-core".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf rm should succeed");

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
            .and_then(|details| details.get("active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| {
                profiles.len() == 1 && profiles[0].as_str() == Some("yoka-desktop")
            })
    );
}

#[test]
fn profile_machine_shape_edit_commands_persist() {
    let tempdir = TempDir::new().expect("tempdir should be created");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-arch".to_owned()],
            vec!["arm64".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-arch should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "add-foreign-arch".to_owned()],
            vec!["i386".to_owned(), "armhf".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf add-foreign-arch should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "remove-foreign-arch".to_owned()],
            vec!["armhf".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf remove-foreign-arch should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "clear-init".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf clear-init should succeed");

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
            .and_then(|details| details.get("native_arch"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "arm64")
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("foreign_arches"))
            .and_then(|value| value.as_array())
            .is_some_and(|arches| arches.len() == 1 && arches[0].as_str() == Some("i386"))
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.is_empty())
    );
}
