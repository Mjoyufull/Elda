use std::fs;

use super::super::support::*;
use super::super::*;
use super::fixtures::{create_system_make_repo, update_system_make_repo, write_system_recipe};

#[test]
fn system_mode_rollback_restores_archived_metadata_and_binary() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-tool", "system backend v1");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.1.0",
        "u elda-v1 - EldaUser /usr/bin/false",
        "d /run/elda-v1 0755 root root -",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");
    let v1_state = current_state_id(tempdir.path());

    update_system_make_repo(&repo_dir, "system backend v2");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.2.0",
        "u elda-v2 - EldaUser /usr/bin/false",
        "d /run/elda-v2 0755 root root -",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/usr/bin/system-tool"),
        "system backend v2"
    );
    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/sysusers.d/system-tool.conf")
        )
        .expect("v2 sysusers metadata should exist"),
        "u elda-v2 - EldaUser /usr/bin/false\n"
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
        run_installed_binary(tempdir.path(), "/usr/bin/system-tool"),
        "system backend v1"
    );
    assert_eq!(current_state_id(tempdir.path()), v1_state);
    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/sysusers.d/system-tool.conf")
        )
        .expect("v1 sysusers metadata should be restored"),
        "u elda-v1 - EldaUser /usr/bin/false\n"
    );
    assert_eq!(
        fs::read_link(tempdir.path().join("usr/bin/toolctl"))
            .expect("alternative link should survive rollback"),
        tempdir.path().join("usr/bin/system-tool")
    );
    assert!(
        tempdir
            .path()
            .join("var/lib/elda/states")
            .join(&v1_state)
            .join("root/usr/bin/system-tool")
            .exists()
    );
}
