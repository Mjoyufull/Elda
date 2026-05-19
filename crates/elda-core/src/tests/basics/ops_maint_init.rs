use super::*;
use elda_types::ExitStatus;

#[test]
fn init_creates_layout_and_default_config() {
    let tempdir = tempfile::TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["init".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("init should succeed");

    assert_eq!(report.area, "init");
    assert_eq!(report.exit_status, ExitStatus::Success);
    assert!(tempdir.path().join("etc/elda/config.toml").is_file());
}

#[test]
fn maint_check_reports_modules() {
    let tempdir = tempfile::TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["maint".to_owned(), "check".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("maint check should succeed");

    assert_eq!(report.area, "maint");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("modules"))
            .and_then(|modules| modules.as_array())
            .is_some_and(|modules| !modules.is_empty())
    );
}
