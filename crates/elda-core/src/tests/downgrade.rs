use super::support::*;
use super::*;

#[test]
fn downgrade_restores_the_latest_older_archived_version() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let v1_binary = create_script_binary(tempdir.path(), "down-tool-v1", "down v1");
    write_local_binary_recipe_with_version(tempdir.path(), "down-tool", &v1_binary, &[], "1.0.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["down-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("initial install should succeed");

    let v2_binary = create_script_binary(tempdir.path(), "down-tool-v2", "down v2");
    write_local_binary_recipe_with_version(tempdir.path(), "down-tool", &v2_binary, &[], "1.1.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/down-tool"),
        "down v2"
    );

    let plan = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["down-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("downgrade dry-run should succeed");
    assert!(
        plan.details
            .and_then(|details| details.get("plan").cloned())
            .and_then(|plan| plan.get("candidate").cloned())
            .and_then(|candidate| candidate.get("pkgver").cloned())
            .and_then(|pkgver| pkgver.as_str().map(ToOwned::to_owned))
            .is_some_and(|pkgver| pkgver == "1.0.0")
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["down-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("downgrade should succeed");

    assert_eq!(report.area, "downgrade");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/down-tool"),
        "down v1"
    );
}

#[test]
fn downgrade_respects_pinned_versions() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let v1_binary = create_script_binary(tempdir.path(), "pin-down-v1", "pin down v1");
    write_local_binary_recipe_with_version(tempdir.path(), "pin-down", &v1_binary, &[], "1.0.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["pin-down".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("initial install should succeed");

    let v2_binary = create_script_binary(tempdir.path(), "pin-down-v2", "pin down v2");
    write_local_binary_recipe_with_version(tempdir.path(), "pin-down", &v2_binary, &[], "1.1.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pin".to_owned()],
            vec!["pin-down".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pin should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["pin-down".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("downgrade should fail while pinned");

    assert!(error.to_string().contains("clear the pin first"));
}

#[test]
fn downgrade_rejects_versions_that_break_installed_reverse_dependencies() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let lib_v1 = create_script_binary(tempdir.path(), "compat-lib-v1", "compat lib v1");
    write_local_binary_recipe_with_version(tempdir.path(), "compat-lib", &lib_v1, &[], "1.0.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["compat-lib".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("initial install should succeed");

    let lib_v2 = create_script_binary(tempdir.path(), "compat-lib-v2", "compat lib v2");
    write_local_binary_recipe_with_version(tempdir.path(), "compat-lib", &lib_v2, &[], "1.1.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["compat-lib".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("upgrade should succeed");

    let app_binary = create_vendor_binary(tempdir.path(), "compat-app");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "compat-app",
        &app_binary,
        "1.0.0",
        "{ \"compat-lib>=1.1.0\" }",
        "{}",
        "{}",
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["compat-app".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("dependent package install should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["compat-lib".to_owned(), "1.0.0-1".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("downgrade should fail when reverse deps need the newer version");

    assert!(error.to_string().contains("requires `compat-lib>=1.1.0`"));
}
