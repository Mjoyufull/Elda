use super::super::support::*;
use super::super::*;

#[test]
fn install_plan_and_runtime_include_recommends_for_explicit_targets() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_recommends(tempdir.path(), "/opt/elda", true);
    let dep_binary = create_vendor_binary(tempdir.path(), "dep-tool");
    let rec_binary = create_vendor_binary(tempdir.path(), "rec-tool");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe(tempdir.path(), "dep-tool", &dep_binary, &[]);
    write_local_binary_recipe(tempdir.path(), "rec-tool", &rec_binary, &[]);
    write_local_binary_recipe_with_recommends(
        tempdir.path(),
        "app-tool",
        &app_binary,
        &["dep-tool"],
        &["rec-tool"],
    );

    let plan_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("install dry-run should succeed");
    let actions = plan_report
        .details
        .and_then(|details| details.get("plan").cloned())
        .and_then(|plan| plan.get("actions").cloned())
        .and_then(|actions| actions.as_array().cloned())
        .expect("install plan should include actions");
    assert_eq!(actions.len(), 3);
    assert!(actions.iter().any(|action| {
        action.get("package").and_then(|value| value.as_str()) == Some("dep-tool")
            && action
                .get("dependency_kind")
                .and_then(|value| value.as_str())
                == Some("depends")
            && action.get("is_weak").and_then(|value| value.as_bool()) == Some(false)
    }));
    assert!(actions.iter().any(|action| {
        action.get("package").and_then(|value| value.as_str()) == Some("rec-tool")
            && action
                .get("dependency_kind")
                .and_then(|value| value.as_str())
                == Some("recommends")
            && action.get("is_weak").and_then(|value| value.as_bool()) == Some(true)
    }));

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
        run_installed_binary(tempdir.path(), "/opt/elda/bin/rec-tool"),
        "binary lane"
    );

    let default_rdeps = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rdeps".to_owned()],
            vec!["rec-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("rdeps should succeed");
    assert!(
        default_rdeps
            .details
            .and_then(|details| details.get("reverse_dependencies").cloned())
            .and_then(|dependencies| dependencies.as_array().cloned())
            .is_some_and(|dependencies| dependencies.is_empty())
    );

    let weak_rdeps = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rdeps".to_owned()],
            vec!["rec-tool".to_owned(), "--weak".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("weak rdeps should succeed");
    assert!(
        weak_rdeps
            .details
            .and_then(|details| details.get("reverse_dependencies").cloned())
            .and_then(|dependencies| dependencies.as_array().cloned())
            .is_some_and(|dependencies| dependencies.iter().any(|dependency| {
                dependency.get("pkgname").and_then(|value| value.as_str()) == Some("app-tool")
                    && dependency.get("is_weak").and_then(|value| value.as_bool()) == Some(true)
            }))
    );
}

#[test]
fn install_skips_recommends_when_disabled_in_config() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_recommends(tempdir.path(), "/opt/elda", false);
    let rec_binary = create_vendor_binary(tempdir.path(), "rec-tool");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    write_local_binary_recipe(tempdir.path(), "rec-tool", &rec_binary, &[]);
    write_local_binary_recipe_with_recommends(
        tempdir.path(),
        "app-tool",
        &app_binary,
        &[],
        &["rec-tool"],
    );

    let plan_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("install dry-run should succeed");
    let actions = plan_report
        .details
        .and_then(|details| details.get("plan").cloned())
        .and_then(|plan| plan.get("actions").cloned())
        .and_then(|actions| actions.as_array().cloned())
        .expect("install plan should include actions");
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].get("package").and_then(|value| value.as_str()),
        Some("app-tool")
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

    assert!(!tempdir.path().join("opt/elda/bin/rec-tool").exists());
}
