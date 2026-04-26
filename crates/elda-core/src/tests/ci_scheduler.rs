use super::support::*;
use super::*;

#[test]
fn ci_run_without_operands_processes_pending_submission_queue() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let source_repo = create_git_make_repo(tempdir.path(), "ci-queued-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-queued-tool");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-queued-tool");

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-queued-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_id = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("id"))
        .and_then(|value| value.as_str())
        .expect("submission id should be reported")
        .to_owned();

    let run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "run".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci scheduler should succeed");

    assert_eq!(
        run.details
            .as_ref()
            .and_then(|details| details.get("processed"))
            .and_then(|value| value.as_array())
            .map(Vec::len),
        Some(1)
    );

    let status = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "status".to_owned()],
            vec![submission_id],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci status should succeed");

    let submission = status
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("submission should be reported");
    assert_eq!(
        submission.get("state").and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        submission.get("attempts").and_then(|value| value.as_u64()),
        Some(1)
    );
    assert_eq!(
        submission
            .get("completed_layers")
            .and_then(|value| value.as_u64()),
        submission
            .get("planned_layers")
            .and_then(|value| value.as_u64())
    );
}

#[test]
fn ci_retry_rebuilds_submission_and_increments_attempts() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let source_repo = create_git_make_repo(tempdir.path(), "ci-retry-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-retry-tool");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-retry-tool");

    let publish = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "run".to_owned()],
            vec!["ci-retry-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci run should succeed");

    let submission_id = publish
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("id"))
        .and_then(|value| value.as_str())
        .expect("submission id should be reported")
        .to_owned();

    let retry = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "retry".to_owned()],
            vec![submission_id],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci retry should succeed");

    let submission = retry
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("submission should be reported");
    assert_eq!(
        submission.get("state").and_then(|value| value.as_str()),
        Some("published")
    );
    assert_eq!(
        submission.get("attempts").and_then(|value| value.as_u64()),
        Some(2)
    );
}
