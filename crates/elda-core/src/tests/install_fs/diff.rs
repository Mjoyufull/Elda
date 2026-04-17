use super::super::support::*;
use super::super::*;

#[test]
fn diff_reports_live_drift_for_installed_package() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary_source = create_script_binary(tempdir.path(), "diff-tool-v1", "diff v1");
    write_local_binary_recipe(tempdir.path(), "diff-tool", &binary_source, &[]);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["diff-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    fs::write(
        tempdir.path().join("opt/elda/bin/diff-tool"),
        "#!/bin/sh\necho 'drifted'\n",
    )
    .expect("installed file should be overwritten");
    make_executable(&tempdir.path().join("opt/elda/bin/diff-tool"));

    let diff_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["diff".to_owned()],
            vec!["diff-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("diff should succeed");

    assert_eq!(diff_report.area, "diff");
    assert_eq!(diff_report.status, "different");
    assert!(
        diff_report
            .details
            .and_then(|details| details.get("changes").cloned())
            .and_then(|changes| changes.as_array().cloned())
            .is_some_and(|changes| changes.iter().any(|change| {
                change
                    .get("path")
                    .and_then(|path| path.as_str())
                    .is_some_and(|path| path == "/usr/bin/diff-tool")
                    && change
                        .get("change")
                        .and_then(|kind| kind.as_str())
                        .is_some_and(|kind| kind == "live-drift")
            }))
    );
}

#[test]
fn diff_candidate_reports_manifest_changes_against_next_candidate() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let v1_binary = create_script_binary(tempdir.path(), "candiff-v1", "candidate v1");
    write_local_binary_recipe_with_version(tempdir.path(), "candiff", &v1_binary, &[], "0.1.0");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["candiff".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let v2_binary = create_script_binary(tempdir.path(), "candiff-v2", "candidate v2");
    write_local_binary_recipe_with_version(tempdir.path(), "candiff", &v2_binary, &[], "0.2.0");

    let diff_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["diff".to_owned()],
            vec!["candiff".to_owned(), "--candidate".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("candidate diff should succeed");

    assert_eq!(diff_report.area, "diff");
    assert_eq!(diff_report.status, "different");
    assert!(
        diff_report
            .details
            .as_ref()
            .and_then(|details| details.get("candidate"))
            .and_then(|candidate| candidate.get("version"))
            .and_then(|version| version.as_str())
            .is_some_and(|version| version == "0:0.2.0-1")
    );
    assert!(
        diff_report
            .details
            .and_then(|details| details.get("changes").cloned())
            .and_then(|changes| changes.as_array().cloned())
            .is_some_and(|changes| changes.iter().any(|change| {
                change
                    .get("path")
                    .and_then(|path| path.as_str())
                    .is_some_and(|path| path == "/usr/bin/candiff")
                    && change
                        .get("change")
                        .and_then(|kind| kind.as_str())
                        .is_some_and(|kind| kind == "modify")
            }))
    );
}
