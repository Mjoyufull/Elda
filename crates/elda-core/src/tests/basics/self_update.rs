use super::*;

#[test]
fn self_update_dry_runs_report_main_and_dev_branches() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    for (command, branch) in [("su", "main"), ("dsu", "dev")] {
        let report = run_from_root(
            tempdir.path(),
            CommandRequest::new(vec![command.to_owned()], Vec::new(), OutputMode::Json, true),
        )
        .expect("self-update dry-run should succeed");

        assert_eq!(report.area, "self-update");
        assert_eq!(report.status, "planned");
        assert_eq!(
            report
                .details
                .as_ref()
                .and_then(|details| details.get("branch"))
                .and_then(serde_json::Value::as_str),
            Some(branch)
        );
    }
}

#[test]
fn self_update_rejects_offline_mode() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["su".to_owned()], Vec::new(), OutputMode::Json, false)
            .with_offline(true),
    )
    .expect_err("offline self-update should fail");

    assert!(error.to_string().contains("requires network access"));
}
