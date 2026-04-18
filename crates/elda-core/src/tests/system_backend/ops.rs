use std::fs;

use serde_json::json;

use elda_db::StateLayout;

use super::super::support::*;
use super::super::*;
use super::fixtures::{create_system_make_repo, write_system_recipe};

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
}
