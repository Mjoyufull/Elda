use std::fs;

use super::*;
use elda_db::StateLayout;

#[test]
fn tofu_rotation_requires_explicit_operator_confirmation() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let index_path = tempdir.path().join("rotation-index.toml");
    let metadata_path = tempdir.path().join("remote-metadata-v1.toml");

    let primary = fixture_remote_signing_key_primary();
    let secondary = fixture_remote_signing_key_secondary();

    write_prefix_config(tempdir.path(), "/opt/elda");
    write_signed_remote_index_with_key(&index_path, "rotation-tool", &primary, "fixture-a");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                format!("main=file://{}", index_path.display()),
                "--metadata-url".to_owned(),
                format!("file://{}", metadata_path.display()),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["sync".to_owned()],
            Vec::new(),
            OutputMode::Human,
            false,
        ),
    )
    .expect("initial human sync should bootstrap tofu");

    write_signed_remote_index_with_key(&index_path, "rotation-tool", &secondary, "fixture-b");
    write_rotation_metadata_document(
        &metadata_path,
        &primary,
        &[("fixture-b", &secondary)],
        &[fingerprint_for_signing_key(&primary)],
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["sync".to_owned()],
            Vec::new(),
            OutputMode::Human,
            false,
        ),
    )
    .expect_err("rotation should require explicit confirmation");

    assert!(error.to_string().contains("requires operator confirmation"));
    assert!(error.to_string().contains("--accept-rotated-key main"));
}

#[test]
fn tofu_rotation_proceeds_when_the_remote_is_explicitly_accepted() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let index_path = tempdir.path().join("rotation-index.toml");
    let metadata_path = tempdir.path().join("remote-metadata-v1.toml");

    let primary = fixture_remote_signing_key_primary();
    let secondary = fixture_remote_signing_key_secondary();

    write_prefix_config(tempdir.path(), "/opt/elda");
    write_signed_remote_index_with_key(&index_path, "rotation-tool", &primary, "fixture-a");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                format!("main=file://{}", index_path.display()),
                "--metadata-url".to_owned(),
                format!("file://{}", metadata_path.display()),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["sync".to_owned()],
            Vec::new(),
            OutputMode::Human,
            false,
        ),
    )
    .expect("initial human sync should bootstrap tofu");

    write_signed_remote_index_with_key(&index_path, "rotation-tool", &secondary, "fixture-b");
    write_rotation_metadata_document(
        &metadata_path,
        &primary,
        &[("fixture-b", &secondary)],
        &[fingerprint_for_signing_key(&primary)],
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false)
            .with_accepted_rotated_keys(vec!["main".to_owned()]),
    )
    .expect("rotation should succeed when explicitly accepted");

    assert_eq!(report.area, "sync");

    let layout = StateLayout::new(tempdir.path(), "/opt/elda");
    let trust_state_path = layout.db_dir.join("repo-state/main.trust.json");
    let trust_state = fs::read_to_string(trust_state_path).expect("trust state should exist");

    assert!(trust_state.contains(&fingerprint_for_signing_key(&secondary)));
}
