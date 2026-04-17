use super::super::support::*;
use super::super::*;

#[test]
fn dependency_install_populates_why_rdeps_and_autoremove() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_binary = create_vendor_binary(tempdir.path(), "dep-tool");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe(tempdir.path(), "dep-tool", &dep_binary, &[]);
    write_local_binary_recipe(tempdir.path(), "app-tool", &app_binary, &["dep-tool"]);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/app-tool"),
        "app tool"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/dep-tool"),
        "binary lane"
    );

    let state_report = run_from_root(
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
        state_report
            .details
            .and_then(|details| details.get("world").cloned())
            .and_then(|world| world.as_array().cloned())
            .is_some_and(|world| {
                world.len() == 1
                    && world
                        .iter()
                        .any(|entry| entry.as_str().is_some_and(|entry| entry == "app-tool"))
            })
    );

    let why_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["why".to_owned()],
            vec!["dep-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("why should succeed");
    assert!(
        why_report
            .details
            .and_then(|details| details.get("reverse_dependencies").cloned())
            .and_then(|dependencies| dependencies.as_array().cloned())
            .is_some_and(|dependencies| dependencies.iter().any(|dependency| {
                dependency
                    .get("pkgname")
                    .and_then(|pkgname| pkgname.as_str())
                    .is_some_and(|pkgname| pkgname == "app-tool")
            }))
    );

    let rdeps_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rdeps".to_owned()],
            vec!["dep-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("rdeps should succeed");
    assert!(
        rdeps_report
            .details
            .and_then(|details| details.get("reverse_dependencies").cloned())
            .and_then(|dependencies| dependencies.as_array().cloned())
            .is_some_and(|dependencies| dependencies.len() == 1)
    );

    let remove_error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["dep-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("removing a required dependency should fail");
    assert!(remove_error.to_string().contains("use `--cascade`"));

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");

    let autoremove_plan = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["autoremove".to_owned()],
            Vec::new(),
            OutputMode::Json,
            true,
        ),
    )
    .expect("autoremove dry-run should succeed");
    assert!(
        autoremove_plan
            .details
            .and_then(|details| details.get("plan").cloned())
            .and_then(|plan| plan.get("actions").cloned())
            .and_then(|actions| actions.as_array().cloned())
            .is_some_and(|actions| actions.iter().any(|action| {
                action
                    .get("target")
                    .and_then(|target| target.as_str())
                    .is_some_and(|target| target == "dep-tool")
            }))
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["autoremove".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("autoremove should succeed");

    assert!(!tempdir.path().join("opt/elda/bin/dep-tool").exists());
}

#[test]
fn install_fails_on_ambiguous_any_of_dependency() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_a = create_vendor_binary(tempdir.path(), "dep-a");
    let dep_b = create_vendor_binary(tempdir.path(), "dep-b");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe(tempdir.path(), "dep-a", &dep_a, &[]);
    write_local_binary_recipe(tempdir.path(), "dep-b", &dep_b, &[]);
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ { any = { \"dep-a\", \"dep-b\" } } }",
        "{}",
        "{}",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("ambiguous any-of dependency should fail");
    assert!(
        error
            .to_string()
            .contains("ambiguous dependency alternatives")
    );
}

#[test]
fn install_selects_unique_virtual_provider_and_records_reverse_dependency() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let provider_binary = create_vendor_binary(tempdir.path(), "mesa-provider");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_provides(
        tempdir.path(),
        "mesa-provider",
        &provider_binary,
        "{}",
        &["gl-provider"],
    );
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider\" }",
        "{}",
        "{}",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/mesa-provider"),
        "binary lane"
    );

    let why_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["why".to_owned()],
            vec!["mesa-provider".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("why should succeed");
    assert!(
        why_report
            .details
            .and_then(|details| details.get("reverse_dependencies").cloned())
            .and_then(|dependencies| dependencies.as_array().cloned())
            .is_some_and(|dependencies| dependencies.iter().any(|dependency| {
                dependency.get("pkgname").and_then(|value| value.as_str()) == Some("app-tool")
                    && dependency.get("raw_expr").and_then(|value| value.as_str())
                        == Some("gl-provider")
            }))
    );
}

#[test]
fn install_fails_on_ambiguous_virtual_provider_dependency() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let provider_a = create_vendor_binary(tempdir.path(), "mesa-provider");
    let provider_b = create_vendor_binary(tempdir.path(), "nvidia-provider");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_provides(
        tempdir.path(),
        "mesa-provider",
        &provider_a,
        "{}",
        &["gl-provider"],
    );
    write_local_binary_recipe_with_provides(
        tempdir.path(),
        "nvidia-provider",
        &provider_b,
        "{}",
        &["gl-provider"],
    );
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider\" }",
        "{}",
        "{}",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("ambiguous virtual provider dependency should fail");
    assert!(error.to_string().contains("ambiguous virtual provider"));
}
