use super::super::support::*;
use super::super::*;

#[test]
fn install_dry_run_returns_structured_plan() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let dep_binary = create_vendor_binary(tempdir.path(), "fd");
    let app_binary = create_vendor_binary(tempdir.path(), "ripgrep");
    write_local_binary_recipe(tempdir.path(), "fd", &dep_binary, &[]);
    write_local_binary_recipe(tempdir.path(), "ripgrep", &app_binary, &["fd"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["ripgrep".to_owned(), "fd".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("install dry-run should succeed");

    assert_eq!(report.area, "plan");
    assert!(
        report
            .details
            .and_then(|details| details.get("plan").cloned())
            .and_then(|plan| plan.get("actions").cloned())
            .and_then(|actions| actions.as_array().cloned())
            .is_some_and(|actions| {
                actions.len() == 2
                    && actions.iter().any(|action| {
                        action.get("package").and_then(|value| value.as_str()) == Some("fd")
                    })
                    && actions.iter().any(|action| {
                        action.get("package").and_then(|value| value.as_str()) == Some("ripgrep")
                    })
            })
    );
}

#[test]
fn direct_git_install_round_trips_through_prefix_backend() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "sample-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    let install_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(install_report.area, "install");
    assert!(
        install_report
            .details
            .as_ref()
            .and_then(|details| details.get("installs"))
            .and_then(|installs| installs.as_array())
            .and_then(|installs| installs.first())
            .and_then(|install| install.get("package"))
            .and_then(|package| package.get("pkgver"))
            .and_then(|pkgver| pkgver.as_str())
            .is_some_and(|pkgver| pkgver.starts_with("0.git."))
    );
    assert!(tempdir.path().join("opt/elda/bin/sample-tool").exists());

    let ls_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["ls".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("ls should succeed");
    assert!(
        ls_report
            .details
            .as_ref()
            .and_then(|details| details.get("packages"))
            .and_then(|packages| packages.as_array())
            .is_some_and(|packages| packages.iter().any(|package| {
                package.get("pkgname").and_then(|value| value.as_str()) == Some("sample-tool")
                    && package
                        .get("version")
                        .and_then(|value| value.as_str())
                        .is_some_and(|version| version.contains(":0.git."))
            }))
    );

    let files_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["files".to_owned()],
            vec!["sample-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("files should succeed");
    assert!(
        files_report
            .details
            .and_then(|details| details.get("files").cloned())
            .and_then(|files| files.as_array().cloned())
            .is_some_and(|files| files.iter().any(|file| {
                file.get("path")
                    .and_then(|path| path.as_str())
                    .is_some_and(|path| path == "/usr/bin/sample-tool")
            }))
    );

    let owner_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["files".to_owned(), "owner".to_owned()],
            vec!["/usr/bin/sample-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("files owner should succeed");
    assert!(
        owner_report
            .details
            .and_then(|details| details.get("owners").cloned())
            .and_then(|owners| owners.as_array().cloned())
            .is_some_and(|owners| owners.len() == 1)
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["sample-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");

    assert!(!tempdir.path().join("opt/elda/bin/sample-tool").exists());
}

#[test]
fn verify_reports_missing_managed_file() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "sample-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");
    fs::remove_file(tempdir.path().join("opt/elda/bin/sample-tool"))
        .expect("installed file should be removable");

    let verify_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["verify".to_owned()],
            vec!["sample-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("verify should succeed");

    assert_eq!(verify_report.area, "verify");
    assert_eq!(verify_report.status, "verify-failed");
    assert!(
        verify_report
            .details
            .and_then(|details| details.get("verify_report").cloned())
            .and_then(|verify| verify.get("issues").cloned())
            .and_then(|issues| issues.as_array().cloned())
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue
                    .get("kind")
                    .and_then(|kind| kind.as_str())
                    .is_some_and(|kind| kind == "missing-file")
            }))
    );
}
