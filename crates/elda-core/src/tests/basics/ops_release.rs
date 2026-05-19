use super::*;
use elda_types::ExitStatus;

#[test]
fn remote_trust_reports_configured_policy() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                "main=https://example.invalid/index.toml".to_owned(),
                "--trust".to_owned(),
                "pinned".to_owned(),
                "--trusted-key".to_owned(),
                "sha256:abc123".to_owned(),
                "--signature-url".to_owned(),
                "https://example.invalid/index.toml.sig".to_owned(),
                "--metadata-url".to_owned(),
                "https://example.invalid/remote-metadata-v1.toml".to_owned(),
                "--allow-stale".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "trust".to_owned()],
            vec!["main".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote trust should succeed");

    let trust = report
        .details
        .as_ref()
        .and_then(|details| details.get("trust_report"))
        .expect("trust report should exist");
    assert_eq!(
        trust.get("trust").and_then(|value| value.as_str()),
        Some("pinned")
    );
    assert_eq!(
        trust
            .get("configured_trusted_keys")
            .and_then(|keys| keys.as_array())
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        trust
            .get("snapshot_present")
            .and_then(|present| present.as_bool()),
        Some(false)
    );
    assert!(
        trust
            .get("payload_verification")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.contains("no synced snapshot yet"))
    );
}

#[test]
fn remote_remove_deletes_remote_document() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec!["heather=https://example.invalid/heather-overlay".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "rm".to_owned()],
            vec!["heather".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote remove should succeed");

    assert_eq!(report.area, "remote");
    assert!(
        !tempdir
            .path()
            .join("etc/elda/remotes.d/heather.toml")
            .exists()
    );
}

#[test]
fn rc_show_reports_local_recipe_details() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "rc-show-tool");
    write_local_binary_recipe(tempdir.path(), "rc-show-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rc".to_owned(), "show".to_owned()],
            vec!["rc-show-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("rc show should succeed");

    assert_eq!(report.area, "recipe");
    assert_eq!(report.status, "ok");
    let show = report
        .details
        .as_ref()
        .and_then(|details| details.get("show"))
        .expect("show details");
    assert_eq!(
        show.get("selected_source").and_then(|value| value.as_str()),
        Some("local")
    );
    assert_eq!(
        show.get("local")
            .and_then(|local| local.get("recipe"))
            .and_then(|recipe| recipe.get("package"))
            .and_then(|package| package.get("name"))
            .and_then(|value| value.as_str()),
        Some("rc-show-tool")
    );
}

#[test]
fn rc_diff_reports_missing_when_synced_definition_absent() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "rc-diff-tool");
    write_local_binary_recipe(tempdir.path(), "rc-diff-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rc".to_owned(), "diff".to_owned()],
            vec!["rc-diff-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("rc diff should return a structured report");

    assert_eq!(report.area, "recipe");
    assert_eq!(report.status, "missing");
    let diff = report
        .details
        .as_ref()
        .and_then(|details| details.get("diff"))
        .expect("diff details");
    assert!(diff.get("local").is_some_and(|value| !value.is_null()));
    assert!(diff.get("synced").is_some_and(|value| value.is_null()));
}

#[test]
fn rc_publish_ready_reports_metadata_blockers() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "rc-publish-tool");
    write_local_binary_recipe(tempdir.path(), "rc-publish-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rc".to_owned(), "publish-ready".to_owned()],
            vec!["rc-publish-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("publish-ready should return a structured report");

    assert_eq!(report.area, "recipe");
    assert_eq!(report.status, "not-ready");
    let blockers = report
        .details
        .as_ref()
        .and_then(|details| details.get("publish_ready"))
        .and_then(|ready| ready.get("blockers"))
        .and_then(|blockers| blockers.as_array())
        .expect("blockers should be listed");
    assert!(
        blockers
            .iter()
            .filter_map(|value| value.as_str())
            .any(|value| value.contains("pkg.description"))
    );
}

#[test]
fn doctor_reports_bootstrap_readiness() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["doctor".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("doctor should return a structured report");

    assert_eq!(report.area, "doctor");
    assert_eq!(report.status, "ok");
    assert_eq!(report.exit_status, ExitStatus::Success);
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("release_readiness"))
            .and_then(|readiness| readiness.get("unsupported_commands_fail_closed"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("advisories"))
            .and_then(|value| value.as_array())
            .is_some_and(|advisories| !advisories.is_empty())
    );
}

#[test]
fn unsupported_runtime_command_fails_closed() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["not-real".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("unsupported command should return a structured report");

    assert_eq!(report.area, "command");
    assert_eq!(report.status, "unsupported");
    assert_eq!(report.exit_status, ExitStatus::OperatorFailure);
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("blocked"))
            .and_then(|blocked| blocked.as_bool())
            .unwrap_or(false)
    );
}
