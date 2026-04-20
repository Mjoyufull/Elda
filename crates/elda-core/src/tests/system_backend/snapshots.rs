use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::super::support::*;
use super::super::*;
use super::fixtures::{create_system_make_repo, write_system_recipe};

#[test]
fn system_mode_transactions_record_snapshots_in_reports_and_archived_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let snapshot_tool = write_fake_snapper(tempdir.path());
    write_prefix_config_with_extras(
        tempdir.path(),
        "/usr",
        true,
        false,
        &format!("snapshot_tool = \"{}\"\n", snapshot_tool.display()),
    );

    let repo_dir = create_system_make_repo(tempdir.path(), "system-tool", "system backend v1");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.1.0",
        "u elda - EldaUser /usr/bin/false",
        "d /run/elda 0755 root root -",
    );

    let install = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system install should succeed");
    assert_snapshot_records(
        install
            .details
            .as_ref()
            .and_then(|details| details.get("installs"))
            .and_then(|installs| installs.as_array())
            .and_then(|installs| installs.first())
            .and_then(|install| install.get("install"))
            .and_then(|install| install.get("snapshots"))
            .and_then(|snapshots| snapshots.as_array())
            .expect("install snapshots should exist"),
    );

    let install_state = current_state_id(tempdir.path());
    assert_archive_snapshots(tempdir.path(), &install_state);

    let remove = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system remove should succeed");
    assert_snapshot_records(
        remove
            .details
            .as_ref()
            .and_then(|details| details.get("removals"))
            .and_then(|removals| removals.as_array())
            .and_then(|removals| removals.first())
            .and_then(|removal| removal.get("snapshots"))
            .and_then(|snapshots| snapshots.as_array())
            .expect("remove snapshots should exist"),
    );

    let remove_state = current_state_id(tempdir.path());
    assert_archive_snapshots(tempdir.path(), &remove_state);

    assert_eq!(
        fs::read_to_string(tempdir.path().join("snapper.log"))
            .expect("snapper log should exist")
            .lines()
            .count(),
        4
    );
}

#[test]
fn unsupported_snapshot_tools_are_recorded_as_failed_requests() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/usr",
        true,
        false,
        "snapshot_tool = \"timeshift\"\n",
    );

    let repo_dir = create_system_make_repo(tempdir.path(), "system-tool", "system backend v1");
    write_system_recipe(
        tempdir.path(),
        "system-tool",
        &repo_dir,
        "0.1.0",
        "u elda - EldaUser /usr/bin/false",
        "d /run/elda 0755 root root -",
    );

    let install = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system install should still succeed");

    let snapshots = install
        .details
        .as_ref()
        .and_then(|details| details.get("installs"))
        .and_then(|installs| installs.as_array())
        .and_then(|installs| installs.first())
        .and_then(|install| install.get("install"))
        .and_then(|install| install.get("snapshots"))
        .and_then(|snapshots| snapshots.as_array())
        .expect("snapshot records should exist");

    assert_eq!(snapshots.len(), 2);
    assert!(snapshots.iter().all(|snapshot| {
        snapshot.get("status").and_then(|value| value.as_str()) == Some("failed")
            && snapshot
                .get("error")
                .and_then(|value| value.as_str())
                .is_some_and(|error| error.contains("not supported"))
    }));
}

fn write_fake_snapper(root: &Path) -> PathBuf {
    let bin_dir = root.join("test-bin");
    fs::create_dir_all(&bin_dir).expect("fake snapper dir should exist");
    let snapper_path = bin_dir.join("snapper");
    let counter_path = root.join("snapper.counter");
    let log_path = root.join("snapper.log");
    fs::write(
        &snapper_path,
        format!(
            "#!/bin/sh\ncount=0\nif [ -f \"{counter}\" ]; then count=$(cat \"{counter}\"); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > \"{counter}\"\nprintf '%s\\n' \"$*\" >> \"{log}\"\nprintf 'snap-%s\\n' \"$count\"\n",
            counter = counter_path.display(),
            log = log_path.display(),
        ),
    )
    .expect("fake snapper should be written");
    let mut permissions = fs::metadata(&snapper_path)
        .expect("fake snapper metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&snapper_path, permissions).expect("fake snapper should be executable");

    snapper_path
}

fn assert_snapshot_records(records: &[Value]) {
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0]
            .get("phase")
            .and_then(|value| value.as_str())
            .expect("phase should exist"),
        "pre-activation"
    );
    assert_eq!(
        records[1]
            .get("phase")
            .and_then(|value| value.as_str())
            .expect("phase should exist"),
        "post-activation"
    );
    assert!(records.iter().all(|record| {
        record.get("status").and_then(|value| value.as_str()) == Some("captured")
            && record
                .get("snapshot_id")
                .and_then(|value| value.as_str())
                .is_some()
    }));
}

fn assert_archive_snapshots(root: &Path, state_id: &str) {
    let archive = serde_json::from_slice::<Value>(
        &fs::read(
            root.join("var/lib/elda/states")
                .join(format!("{state_id}.json")),
        )
        .expect("state archive should exist"),
    )
    .expect("state archive should parse");
    assert_snapshot_records(
        archive
            .get("snapshots")
            .and_then(|snapshots| snapshots.as_array())
            .expect("archived snapshots should exist"),
    );
}
