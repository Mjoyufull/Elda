use std::fs;

use serde_json::json;

use elda_db::StateLayout;

use super::super::support::*;
use super::super::*;
use super::fixtures::{
    create_system_make_repo, write_system_recipe, write_system_recipe_with_provider_assets,
};

#[test]
fn fix_triggers_repairs_current_system_backend_outputs_and_check_reports_pending_records() {
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
    let ldconfig_output = layout
        .state_dir
        .join("system-backend/triggers/ldconfig.json");
    fs::remove_file(&ldconfig_output).expect("trigger output should be removable");

    let trigger_state_path = layout.state_dir.join("system-backend/triggers.json");
    fs::write(
        &trigger_state_path,
        serde_json::to_vec_pretty(&json!({
            "pending": [{ "name": "ldconfig", "reason": "manual test injection" }],
            "last_run": [],
        }))
        .expect("trigger state should serialize"),
    )
    .expect("trigger state should be writable");

    let check_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["check".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("check should succeed");
    assert_eq!(check_report.area, "check");
    assert_eq!(check_report.status, "issues");
    assert!(
        check_report
            .details
            .as_ref()
            .and_then(|details| details.get("pending_triggers"))
            .and_then(|pending| pending.as_array())
            .is_some_and(|pending| pending.len() == 1)
    );

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

    assert_eq!(report.area, "ops");
    assert_eq!(report.status, "ok");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("trigger_repair"))
            .and_then(|repair| repair.get("backend"))
            .and_then(|backend| backend.as_str())
            .is_some_and(|backend| backend == "linux-copy")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("trigger_repair"))
            .and_then(|repair| repair.get("repaired"))
            .and_then(|repaired| repaired.as_array())
            .is_some_and(|repaired| {
                repaired
                    .iter()
                    .any(|value| value.as_str() == Some("ldconfig"))
            })
    );
    assert!(ldconfig_output.exists());

    let trigger_list = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["trigger".to_owned(), "ls".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("trigger ls should succeed");
    assert_eq!(trigger_list.area, "trigger");
    assert_eq!(trigger_list.status, "ok");
    assert!(
        trigger_list
            .details
            .as_ref()
            .and_then(|details| details.get("triggers"))
            .and_then(|triggers| triggers.get("last_run"))
            .and_then(|last_run| last_run.as_array())
            .is_some_and(|last_run| last_run.iter().any(|record| {
                record.get("name").and_then(|name| name.as_str()) == Some("ldconfig")
            }))
    );

    let trigger_info = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["trigger".to_owned(), "info".to_owned()],
            vec!["ldconfig".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("trigger info should succeed");
    assert!(
        trigger_info
            .details
            .as_ref()
            .and_then(|details| details.get("trigger"))
            .and_then(|trigger| trigger.get("output"))
            .is_some_and(|output| !output.is_null())
    );
}

#[test]
fn check_reports_pending_critical_boot_trigger_repair() {
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
    let trigger_state_path = layout.state_dir.join("system-backend/triggers.json");
    fs::write(
        &trigger_state_path,
        serde_json::to_vec_pretty(&json!({
            "pending": [{
                "name": "initramfs",
                "reason": "manual critical test injection",
                "boot_path": true,
                "critical": true
            }],
            "last_run": [],
        }))
        .expect("trigger state should serialize"),
    )
    .expect("trigger state should be writable");

    let check_report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["check".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("check should succeed");

    assert_eq!(check_report.area, "check");
    assert_eq!(check_report.status, "issues");
    assert!(
        check_report
            .details
            .as_ref()
            .and_then(|details| details.get("health"))
            .and_then(|health| health.get("issues"))
            .and_then(|issues| issues.as_array())
            .is_some_and(|issues| issues.iter().any(|issue| {
                issue
                    .as_str()
                    .is_some_and(|issue| issue.contains("critical boot trigger repair"))
            }))
    );
    assert!(
        check_report
            .details
            .as_ref()
            .and_then(|details| details.get("backend"))
            .and_then(|backend| backend.get("boot"))
            .and_then(|boot| boot.get("pending_triggers"))
            .and_then(|pending| pending.as_array())
            .is_some_and(|pending| pending.iter().any(|record| {
                record.get("name").and_then(|name| name.as_str()) == Some("initramfs")
            }))
    );
}

#[test]
fn fix_triggers_reconciles_provider_assets_after_init_change() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-service", "system backend v1");

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

    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "system-service",
        &repo_dir,
        "0.1.0",
        "u service - ServiceUser /usr/bin/false",
        "d /run/system-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/system-service",
          text = "#!/bin/sh\necho dinit\n",
          mode = "0755",
        },
      },
      openrc = {
        {
          kind = "file",
          target = "/etc/init.d/system-service",
          text = "#!/bin/sh\necho openrc\n",
          mode = "0755",
        },
      },
    },
  }"##,
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    assert!(tempdir.path().join("etc/dinit.d/system-service").exists());
    assert!(!tempdir.path().join("etc/init.d/system-service").exists());

    let set_init = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["openrc".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");
    assert!(
        set_init
            .details
            .as_ref()
            .and_then(|details| details.get("provider_reconciliation"))
            .and_then(|reconciliation| reconciliation.get("backend"))
            .and_then(|backend| backend.as_str())
            .is_some_and(|backend| backend == "linux-copy")
    );
    assert!(
        set_init
            .details
            .as_ref()
            .and_then(|details| details.get("pending_handler_transitions"))
            .and_then(|handlers| handlers.as_array())
            .is_some_and(|handlers| handlers.is_empty())
    );
    assert!(!tempdir.path().join("etc/dinit.d/system-service").exists());
    assert_eq!(
        fs::read_to_string(tempdir.path().join("etc/init.d/system-service"))
            .expect("openrc provider asset should exist"),
        "#!/bin/sh\necho openrc\n"
    );

    fs::remove_file(tempdir.path().join("etc/init.d/system-service"))
        .expect("provider asset should be removable for repair test");

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

    assert_eq!(report.status, "ok");
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_repair"))
            .and_then(|repair| repair.get("backend"))
            .and_then(|backend| backend.as_str())
            .is_some_and(|backend| backend == "linux-copy")
    );
    assert_eq!(
        fs::read_to_string(tempdir.path().join("etc/init.d/system-service"))
            .expect("openrc provider asset should exist"),
        "#!/bin/sh\necho openrc\n"
    );
}

#[test]
fn system_mode_set_init_reports_packages_missing_assets_for_the_new_provider() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-service", "system backend v1");

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

    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "system-service",
        &repo_dir,
        "0.1.0",
        "u service - ServiceUser /usr/bin/false",
        "d /run/system-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/system-service",
          text = "#!/bin/sh\necho dinit\n",
          mode = "0755",
        },
      },
    },
  }"##,
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["openrc".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");

    assert!(!tempdir.path().join("etc/dinit.d/system-service").exists());
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_reconciliation"))
            .and_then(|reconciliation| reconciliation.get("missing_provider_packages"))
            .and_then(|packages| packages.as_array())
            .is_some_and(|packages| {
                packages.len() == 1 && packages[0].as_str() == Some("system-service")
            })
    );
}
