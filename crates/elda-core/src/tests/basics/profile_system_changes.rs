use super::*;

#[test]
fn profile_show_reports_pending_init_and_multilib_handlers() {
    let tempdir = TempDir::new().expect("tempdir should be created");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "add-foreign-arch".to_owned()],
            vec!["i386".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf add-foreign-arch should succeed");

    let show = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed");

    let kinds = handler_kinds(
        show.details
            .as_ref()
            .and_then(|details| details.get("pending_handler_transitions"))
            .expect("pending handlers should be present"),
    );

    assert_eq!(
        kinds,
        vec!["init-provider-transition", "multilib-policy-transition"]
    );
    assert!(
        show.details
            .as_ref()
            .and_then(|details| details.get("required_activation_class"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "reboot-required")
    );
}

#[test]
fn profile_apply_dry_run_reports_pending_profile_reconciliation() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let profile_repo = create_git_make_repo(tempdir.path(), "yoka-core-source");

    write_local_profile_recipe(tempdir.path(), "yoka-core", &profile_repo, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec![
                "yoka-core".to_owned(),
                "--init".to_owned(),
                "dinit".to_owned(),
            ],
            OutputMode::Json,
            true,
        ),
    )
    .expect("pf apply dry-run should succeed");

    let kinds = handler_kinds(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("plan"))
            .and_then(|plan| plan.get("pending_handler_transitions"))
            .expect("pending plan handlers should be present"),
    );

    assert_eq!(
        kinds,
        vec![
            "profile-provider-reconciliation",
            "init-provider-transition"
        ]
    );
}

#[test]
fn fix_triggers_reports_pending_profile_handlers() {
    let tempdir = TempDir::new().expect("tempdir should be created");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fix-triggers".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("fix-triggers should succeed");

    assert_eq!(report.status, "pending");
    assert_eq!(
        handler_kinds(
            report
                .details
                .as_ref()
                .and_then(|details| details.get("pending_handlers"))
                .expect("pending handlers should be present"),
        ),
        vec!["init-provider-transition"]
    );
}

fn handler_kinds(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .expect("handler list should be an array")
        .iter()
        .map(|entry| {
            entry
                .get("kind")
                .and_then(|kind| kind.as_str())
                .expect("handler kind should be a string")
        })
        .collect()
}
