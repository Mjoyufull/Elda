use super::*;

#[test]
fn profile_apply_installs_base_anchor_and_persists_policy() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_binary = create_vendor_binary(tempdir.path(), "base-tool");
    let profile_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");

    write_local_binary_recipe(tempdir.path(), "base-tool", &dep_binary, &[]);
    write_local_profile_recipe(tempdir.path(), "yoka-core", &profile_repo, &["base-tool"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec![
                "yoka-core".to_owned(),
                "--init".to_owned(),
                "dinit".to_owned(),
                "--foreign-arch".to_owned(),
                "i386".to_owned(),
                "--foreign-arch".to_owned(),
                "arm64".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed");

    assert_eq!(report.area, "profile");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("next_active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| {
                profiles.len() == 1 && profiles[0].as_str() == Some("yoka-core")
            })
    );

    let state = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("state show should succeed");
    assert!(
        state
            .details
            .as_ref()
            .and_then(|details| details.get("world"))
            .and_then(|world| world.as_array())
            .is_some_and(|world| world.is_empty())
    );

    let ls = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["ls".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("ls should succeed");
    let packages = ls
        .details
        .as_ref()
        .and_then(|details| details.get("packages"))
        .and_then(|packages| packages.as_array())
        .cloned()
        .expect("packages should be listed");

    assert!(packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("yoka-core")
            && package.get("package_kind").and_then(|value| value.as_str()) == Some("profile")
            && package
                .get("install_reason")
                .and_then(|value| value.as_str())
                == Some("base")
    }));
    assert!(packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("base-tool")
            && package
                .get("install_reason")
                .and_then(|value| value.as_str())
                == Some("dep")
    }));

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
                profiles.len() == 1 && profiles[0].as_str() == Some("yoka-core")
            })
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "dinit")
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("foreign_arches"))
            .and_then(|arches| arches.as_array())
            .is_some_and(|arches| {
                arches.len() == 2
                    && arches[0].as_str() == Some("i386")
                    && arches[1].as_str() == Some("arm64")
            })
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("pending_handler_transitions"))
            .and_then(|handlers| handlers.as_array())
            .is_some_and(|handlers| {
                handlers.len() == 2
                    && handlers[0].get("kind").and_then(|value| value.as_str())
                        == Some("init-provider-transition")
                    && handlers[1].get("kind").and_then(|value| value.as_str())
                        == Some("multilib-policy-transition")
            })
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("required_activation_class"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "reboot-required")
    );
}

#[test]
fn profile_apply_dry_run_does_not_install_or_persist_state() {
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
            true,
        ),
    )
    .expect("pf apply dry-run should succeed");

    let state = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("state show should succeed");
    assert!(
        state
            .details
            .as_ref()
            .and_then(|details| details.get("packages"))
            .and_then(|packages| packages.as_array())
            .is_some_and(|packages| packages.is_empty())
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
            .and_then(|details| details.get("active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| profiles.is_empty())
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

#[test]
fn profile_apply_replaces_previous_active_profile_anchor_set() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let core_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");
    let desktop_repo = create_git_make_repo(tempdir.path(), "yoka-desktop-source");

    write_local_profile_recipe(tempdir.path(), "yoka-core", &core_repo, &[]);
    write_local_profile_recipe(tempdir.path(), "yoka-desktop", &desktop_repo, &[]);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec!["yoka-core".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("first pf apply should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec!["yoka-desktop".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("second pf apply should succeed");

    let ls = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["ls".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("ls should succeed");
    let packages = ls
        .details
        .as_ref()
        .and_then(|details| details.get("packages"))
        .and_then(|packages| packages.as_array())
        .cloned()
        .expect("packages should be listed");

    assert!(packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("yoka-desktop")
    }));
    assert!(!packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("yoka-core")
    }));
}

#[test]
fn profile_apply_rejects_non_profile_target() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let binary = create_vendor_binary(tempdir.path(), "plain-tool");

    write_local_binary_recipe(tempdir.path(), "plain-tool", &binary, &[]);

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec!["plain-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("non-profile target should fail");

    assert!(
        error
            .to_string()
            .contains("is not a `package_kind = profile` recipe")
    );
}
