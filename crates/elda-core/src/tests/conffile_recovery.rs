use super::support::*;
use super::*;

#[test]
fn install_keeps_preexisting_conffile_and_writes_eldanew() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir =
        create_git_make_conffile_repo(tempdir.path(), "conf-tool", "binary v1", "packaged = v1\n");
    write_local_git_conffile_recipe(tempdir.path(), "conf-tool", &repo_dir, "0.1.0");

    let live_conffile = tempdir.path().join("opt/elda/etc/conf-tool.conf");
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
            vec!["conf-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        fs::read_to_string(&live_conffile).expect("live conffile should be readable"),
        "local = keep\n"
    );
    assert_eq!(
        fs::read_to_string(tempdir.path().join("opt/elda/etc/conf-tool.conf.eldanew"))
            .expect("eldanew file should be readable"),
        "packaged = v1\n"
    );
}

#[test]
fn remove_preserves_modified_conffile_as_eldasave() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir =
        create_git_make_conffile_repo(tempdir.path(), "save-tool", "binary v1", "packaged = v1\n");
    write_local_git_conffile_recipe(tempdir.path(), "save-tool", &repo_dir, "0.1.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["save-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let live_conffile = tempdir.path().join("opt/elda/etc/save-tool.conf");
    fs::write(&live_conffile, "local = modified\n").expect("conffile should be rewritten");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["save-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");

    assert!(!live_conffile.exists(), "live conffile should be removed");
    assert_eq!(
        fs::read_to_string(tempdir.path().join("opt/elda/etc/save-tool.conf.eldasave"))
            .expect("eldasave file should exist"),
        "local = modified\n"
    );
}

#[test]
fn purge_remove_drops_modified_conffile_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir =
        create_git_make_conffile_repo(tempdir.path(), "purge-tool", "binary v1", "packaged = v1\n");
    write_local_git_conffile_recipe(tempdir.path(), "purge-tool", &repo_dir, "0.1.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["purge-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let live_conffile = tempdir.path().join("opt/elda/etc/purge-tool.conf");
    fs::write(&live_conffile, "local = modified\n").expect("conffile should be rewritten");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["purge-tool".to_owned(), "--purge-conffiles".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove with purge should succeed");

    assert!(!live_conffile.exists(), "live conffile should be removed");
    assert!(
        !tempdir
            .path()
            .join("opt/elda/etc/purge-tool.conf.eldasave")
            .exists(),
        "eldasave file should not exist after purge remove"
    );
}

#[test]
fn upgrade_keeps_modified_conffile_and_writes_new_version_as_eldanew() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_conffile_repo(
        tempdir.path(),
        "upgrade-conf",
        "binary v1",
        "packaged = v1\n",
    );
    write_local_git_conffile_recipe(tempdir.path(), "upgrade-conf", &repo_dir, "0.1.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["upgrade-conf".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let live_conffile = tempdir.path().join("opt/elda/etc/upgrade-conf.conf");
    fs::write(&live_conffile, "local = keep\n").expect("conffile should be rewritten");

    update_git_make_conffile_repo(&repo_dir, "binary v2", "packaged = v2\n");
    write_local_git_conffile_recipe(tempdir.path(), "upgrade-conf", &repo_dir, "0.2.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");

    assert_eq!(
        fs::read_to_string(&live_conffile).expect("live conffile should remain readable"),
        "local = keep\n"
    );
    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("opt/elda/etc/upgrade-conf.conf.eldanew")
        )
        .expect("eldanew file should exist"),
        "packaged = v2\n"
    );
}

#[test]
fn recover_cleans_pending_install_journal() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let journal_dir = tempdir
        .path()
        .join("var/lib/elda/prefixes/prefix-2f6f70742f656c6461/db/journal");
    let target_path = tempdir.path().join("opt/elda/bin/demo");
    fs::create_dir_all(journal_dir.clone()).expect("journal dir should exist");
    fs::create_dir_all(target_path.parent().expect("parent should exist"))
        .expect("target parent should exist");
    fs::write(&target_path, "demo").expect("target file should exist");
    fs::write(
        journal_dir.join("txn-test.json"),
        format!(
            "{{\n  \"journal_id\": \"txn-test\",\n  \"package_name\": \"demo\",\n  \"transaction_kind\": \"install\",\n  \"state\": \"files-applied\",\n  \"transaction_root\": \"{}\",\n  \"state_id\": \"prefix-test\",\n  \"created_paths\": [\"{}\"],\n  \"backup_entries\": []\n}}\n",
            tempdir.path().join("var/tmp/elda/prefixes/prefix-2f6f70742f656c6461/transactions/test").display(),
            target_path.display(),
        ),
    )
    .expect("journal should be written");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["recover".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("recover should succeed");

    assert_eq!(report.area, "recovery");
    assert!(!target_path.exists());
    assert!(
        !journal_dir.join("txn-test.json").exists(),
        "recovery should remove the journal"
    );
}
