use std::fs;

use elda_db::{Database, StateLayout};

use super::super::support::*;
use super::super::*;
use super::fixtures::{create_system_make_repo, write_system_recipe};

#[test]
fn system_mode_install_materializes_metadata_and_records_linux_backend() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-tool", "system backend v1");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.1.0",
        "u elda - EldaUser /usr/bin/false",
        "d /run/elda 0755 root root -",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    let layout = StateLayout::new(tempdir.path(), "/usr");
    let database = Database::new(layout.clone());
    let installed = database
        .installed_package("system-tool")
        .expect("package lookup should succeed")
        .expect("system-tool should be installed");

    assert_eq!(installed.activation_backend.as_deref(), Some("linux-copy"));
    assert!(
        installed
            .state_id
            .as_deref()
            .is_some_and(|state_id| state_id.starts_with("system-"))
    );
    assert_eq!(
        current_state_id(tempdir.path()),
        installed.state_id.expect("state id should be recorded")
    );
    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/sysusers.d/system-tool.conf")
        )
        .expect("sysusers metadata should exist"),
        "u elda - EldaUser /usr/bin/false\n"
    );
    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/tmpfiles.d/system-tool.conf")
        )
        .expect("tmpfiles metadata should exist"),
        "d /run/elda 0755 root root -\n"
    );
    assert_eq!(
        fs::read_link(tempdir.path().join("usr/bin/toolctl"))
            .expect("alternative link should exist"),
        tempdir.path().join("usr/bin/system-tool")
    );
    assert!(
        layout
            .state_dir
            .join("system-backend/triggers/ldconfig.json")
            .exists()
    );
    assert!(
        layout
            .state_dir
            .join("system-backend/triggers/desktop_db.json")
            .exists()
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode remove should succeed");

    assert!(
        !tempdir
            .path()
            .join("usr/lib/elda/sysusers.d/system-tool.conf")
            .exists()
    );
    assert!(
        !tempdir
            .path()
            .join("usr/lib/elda/tmpfiles.d/system-tool.conf")
            .exists()
    );
    assert!(!tempdir.path().join("usr/bin/toolctl").exists());
}

#[test]
fn info_reports_installed_system_assets_for_system_mode_package() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-tool", "system backend v1");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.1.0",
        "u elda - EldaUser /usr/bin/false",
        "d /run/elda 0755 root root -",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["info".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed");

    assert_eq!(report.area, "info");
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("installed_files_summary"))
            .and_then(|summary| summary.get("total_paths"))
            .and_then(|count| count.as_u64()),
        Some(8)
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("installed_system_assets"))
            .and_then(|assets| assets.get("sysusers"))
            .and_then(|sysusers| sysusers.get("path"))
            .and_then(|path| path.as_str()),
        Some("/usr/lib/elda/sysusers.d/system-tool.conf")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("installed_system_assets"))
            .and_then(|assets| assets.get("alternatives"))
            .and_then(|alternatives| alternatives.as_array())
            .is_some_and(|alternatives| alternatives.iter().any(|alternative| {
                alternative.get("link").and_then(|link| link.as_str()) == Some("/usr/bin/toolctl")
            }))
    );
}

#[test]
fn system_mode_stages_the_composed_next_root_under_states_dir() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");

    let alpha_repo = create_system_make_repo(tempdir.path(), "alpha-tool", "alpha v1");
    write_system_recipe(
        tempdir.path(),
        "alpha-tool",
        &alpha_repo,
        "0.1.0",
        "u alpha - AlphaUser /usr/bin/false",
        "d /run/alpha 0755 root root -",
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["alpha-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("alpha install should succeed");

    let beta_repo = create_system_make_repo(tempdir.path(), "beta-tool", "beta v1");
    write_system_recipe(
        tempdir.path(),
        "beta-tool",
        &beta_repo,
        "0.1.0",
        "u beta - BetaUser /usr/bin/false",
        "d /run/beta 0755 root root -",
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("beta install should succeed");

    let second_state = current_state_id(tempdir.path());
    let second_stage_root = tempdir
        .path()
        .join("var/lib/elda/states")
        .join(&second_state)
        .join("root");
    assert!(second_stage_root.join("usr/bin/alpha-tool").exists());
    assert!(second_stage_root.join("usr/bin/beta-tool").exists());
    assert_eq!(
        run_installed_binary(tempdir.path(), "/usr/bin/alpha-tool"),
        "alpha v1"
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/usr/bin/beta-tool"),
        "beta v1"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["alpha-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("alpha remove should succeed");

    let third_state = current_state_id(tempdir.path());
    let third_stage_root = tempdir
        .path()
        .join("var/lib/elda/states")
        .join(&third_state)
        .join("root");
    assert!(!third_stage_root.join("usr/bin/alpha-tool").exists());
    assert!(third_stage_root.join("usr/bin/beta-tool").exists());
    assert!(!tempdir.path().join("usr/bin/alpha-tool").exists());
    assert_eq!(
        run_installed_binary(tempdir.path(), "/usr/bin/beta-tool"),
        "beta v1"
    );
}
