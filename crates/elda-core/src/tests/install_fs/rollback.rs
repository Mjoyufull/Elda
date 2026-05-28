use super::super::support::*;
use super::super::*;

#[test]
fn rollback_restores_previous_archived_prefix_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let v1_binary = create_script_binary(tempdir.path(), "rollback-tool-v1", "rollback v1");
    write_local_binary_recipe_with_version(
        tempdir.path(),
        "rollback-tool",
        &v1_binary,
        &[],
        "0.1.0",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["rollback-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");
    let v1_state = current_state_id(tempdir.path());

    let v2_binary = create_script_binary(tempdir.path(), "rollback-tool-v2", "rollback v2");
    write_local_binary_recipe_with_version(
        tempdir.path(),
        "rollback-tool",
        &v2_binary,
        &[],
        "0.2.0",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/rollback-tool"),
        "rollback v2"
    );

    let rollback_plan = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rollback".to_owned()],
            Vec::new(),
            OutputMode::Json,
            true,
        ),
    )
    .expect("rollback dry-run should succeed");
    assert!(
        rollback_plan
            .details
            .as_ref()
            .and_then(|details| details.get("plan"))
            .and_then(|plan| plan.get("to_state"))
            .and_then(|state| state.as_str())
            .is_some_and(|state| state == v1_state)
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rollback".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("rollback should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/rollback-tool"),
        "rollback v1"
    );
    assert_eq!(current_state_id(tempdir.path()), v1_state);
}
