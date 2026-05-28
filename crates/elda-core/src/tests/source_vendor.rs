use super::support::*;
use super::*;

#[test]
fn install_defaults_to_binary_lane_and_prefer_source_overrides_it() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "dual-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "dual-tool");
    write_dual_lane_recipe(tempdir.path(), &repo_dir, &binary_source, "dual-tool");

    let default_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["dual-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("default install should succeed");

    assert_eq!(default_report.area, "install");
    assert!(
        default_report
            .details
            .and_then(|details| details.get("installs").cloned())
            .and_then(|installs| installs.as_array().cloned())
            .is_some_and(|installs| installs.iter().any(|install| {
                install
                    .get("selected_lane")
                    .and_then(|lane| lane.as_str())
                    .is_some_and(|lane| lane == "binary")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/dual-tool"),
        "binary lane"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["dual-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");

    let source_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["dual-tool".to_owned(), "--prefer-source".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("source-preferred install should succeed");

    assert_eq!(source_report.area, "install");
    assert!(
        source_report
            .details
            .and_then(|details| details.get("installs").cloned())
            .and_then(|installs| installs.as_array().cloned())
            .is_some_and(|installs| installs.iter().any(|install| {
                install
                    .get("selected_lane")
                    .and_then(|lane| lane.as_str())
                    .is_some_and(|lane| lane == "source")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/dual-tool"),
        "sample-tool"
    );
}

#[test]
fn binary_lane_command_rejects_raw_git_targets() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "sample-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ib".to_owned()],
            vec![repo_url],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("ib on a git URL should fail");

    assert!(
        error
            .to_string()
            .contains("does not expose a `binary` install lane")
    );
}

#[test]
fn vendor_add_and_install_local_binary_recipe() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "demo-bin");

    let add_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["vendor".to_owned(), "add".to_owned()],
            vec!["demo-bin".to_owned(), binary.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("vendor add should succeed");

    assert_eq!(add_report.area, "vendor");
    assert!(
        tempdir
            .path()
            .join("etc/elda/recipes/demo-bin/pkg.lua")
            .exists()
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["demo-bin".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/demo-bin"),
        "binary lane"
    );
}

#[test]
fn vendor_export_and_import_round_trip_through_core_handlers() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "demo-bin");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["vendor".to_owned(), "add".to_owned()],
            vec!["demo-bin".to_owned(), binary.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("vendor add should succeed");

    let lock_path = tempdir.path().join("vendor.lock.json");
    let export_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["vendor".to_owned(), "export".to_owned()],
            vec![lock_path.display().to_string(), "demo-bin".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("vendor export should succeed");
    assert_eq!(export_report.area, "vendor");

    fs::remove_dir_all(tempdir.path().join("etc/elda/recipes/demo-bin"))
        .expect("recipe dir should be removable");

    let import_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["vendor".to_owned(), "import".to_owned()],
            vec![lock_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("vendor import should succeed");
    assert_eq!(import_report.area, "vendor");
    assert!(
        tempdir
            .path()
            .join("etc/elda/recipes/demo-bin/pkg.lua")
            .exists()
    );
}
