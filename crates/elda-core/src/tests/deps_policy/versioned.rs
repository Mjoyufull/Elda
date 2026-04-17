use super::super::support::*;
use super::super::*;

#[test]
fn install_honors_versioned_exact_dependency_constraints() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_binary = create_vendor_binary(tempdir.path(), "dep-tool");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_version(tempdir.path(), "dep-tool", &dep_binary, &[], "1.2.0");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"dep-tool>=1.1.0\" }",
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
        run_installed_binary(tempdir.path(), "/opt/elda/bin/dep-tool"),
        "binary lane"
    );
}

#[test]
fn install_rejects_unsatisfied_versioned_exact_dependency_constraints() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_binary = create_vendor_binary(tempdir.path(), "dep-tool");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_version(tempdir.path(), "dep-tool", &dep_binary, &[], "1.0.0");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"dep-tool>=1.1.0\" }",
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
    .expect_err("install should fail");

    assert!(
        error
            .to_string()
            .contains("no package or explicit versioned provide satisfies `dep-tool>=1.1.0`")
    );
}

#[test]
fn install_allows_versioned_virtual_provider_when_explicitly_declared() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let provider_binary = create_vendor_binary(tempdir.path(), "mesa-provider");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "mesa-provider",
        &provider_binary,
        "0.1.0",
        "{}",
        "{}",
        "{ \"gl-provider=2\" }",
    );
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider>=2\" }",
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
}

#[test]
fn install_rejects_unversioned_provider_for_versioned_dependency() {
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
        "{ \"gl-provider>=2\" }",
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
    .expect_err("install should fail");

    assert!(
        error
            .to_string()
            .contains("no package or explicit versioned provide satisfies `gl-provider>=2`")
    );
}

#[test]
fn exact_package_name_beats_virtual_provider_for_versioned_dependency() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let exact_binary = create_vendor_binary(tempdir.path(), "gl-provider");
    let provider_binary = create_vendor_binary(tempdir.path(), "mesa-provider");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe_with_version(
        tempdir.path(),
        "gl-provider",
        &exact_binary,
        &[],
        "2.0.0",
    );
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "mesa-provider",
        &provider_binary,
        "0.1.0",
        "{}",
        "{}",
        "{ \"gl-provider=2\" }",
    );
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider>=2\" }",
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

    assert!(tempdir.path().join("opt/elda/bin/gl-provider").exists());
    assert!(!tempdir.path().join("opt/elda/bin/mesa-provider").exists());
}
