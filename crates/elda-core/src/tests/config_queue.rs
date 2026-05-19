use super::support::*;
use super::*;

#[test]
fn config_diff_and_apply_resolves_eldanew_queue() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir =
        create_git_make_conffile_repo(tempdir.path(), "merge-tool", "binary v1", "packaged = v1\n");
    write_local_git_conffile_recipe(tempdir.path(), "merge-tool", &repo_dir, "0.1.0");

    let live_conffile = tempdir.path().join("opt/elda/etc/merge-tool.conf");
    fs::create_dir_all(
        live_conffile
            .parent()
            .expect("conffile parent should be present"),
    )
    .expect("conffile parent should be created");
    fs::write(&live_conffile, "local = keep\n").expect("local conffile should exist");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["merge-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let diff = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["config".to_owned(), "diff".to_owned()],
            vec!["/etc/merge-tool.conf".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("config diff should succeed");
    let diff_lines = diff
        .details
        .as_ref()
        .expect("details")
        .get("config_diff")
        .and_then(|value| value.get("diff"))
        .and_then(|value| value.as_array())
        .expect("diff lines");
    assert!(
        diff_lines
            .iter()
            .any(|line| line.as_str() == Some("-1: local = keep"))
    );
    assert!(
        diff_lines
            .iter()
            .any(|line| line.as_str() == Some("+1: packaged = v1"))
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["config".to_owned(), "apply".to_owned()],
            vec!["merge-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("config apply should succeed");

    assert_eq!(
        fs::read_to_string(&live_conffile).expect("live conffile should be readable"),
        "packaged = v1\n"
    );
    assert!(
        !tempdir
            .path()
            .join("opt/elda/etc/merge-tool.conf.eldanew")
            .exists()
    );
}

#[test]
fn config_keep_discards_eldanew_and_keeps_live_file() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir =
        create_git_make_conffile_repo(tempdir.path(), "keep-tool", "binary v1", "packaged = v1\n");
    write_local_git_conffile_recipe(tempdir.path(), "keep-tool", &repo_dir, "0.1.0");

    let live_conffile = tempdir.path().join("opt/elda/etc/keep-tool.conf");
    fs::create_dir_all(
        live_conffile
            .parent()
            .expect("conffile parent should be present"),
    )
    .expect("conffile parent should be created");
    fs::write(&live_conffile, "local = keep\n").expect("local conffile should exist");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["keep-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["config".to_owned(), "keep".to_owned()],
            vec!["/etc/keep-tool.conf".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("config keep should succeed");

    assert_eq!(
        fs::read_to_string(&live_conffile).expect("live conffile should be readable"),
        "local = keep\n"
    );
    assert!(
        !tempdir
            .path()
            .join("opt/elda/etc/keep-tool.conf.eldanew")
            .exists()
    );
}
