use super::support::*;
use super::*;

#[test]
fn desired_state_round_trips_profile_machine_shape_between_prefix_roots() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let source_root = tempdir.path().join("source-root");
    let target_root = tempdir.path().join("target-root");
    fs::create_dir_all(&source_root).expect("source root should exist");
    fs::create_dir_all(&target_root).expect("target root should exist");

    write_prefix_config(&source_root, "/opt/elda");
    write_prefix_config(&target_root, "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "phase8-base-tool");
    let profile_repo = create_git_make_repo(tempdir.path(), "phase8-core-source");

    write_local_binary_recipe(&source_root, "phase8-base-tool", &binary, &[]);
    write_local_profile_recipe(
        &source_root,
        "phase8-core",
        &profile_repo,
        &["phase8-base-tool"],
    );

    run_from_root(
        &source_root,
        CommandRequest::new(
            vec!["pf".to_owned(), "apply".to_owned()],
            vec![
                "phase8-core".to_owned(),
                "--init".to_owned(),
                "dinit".to_owned(),
                "--native-arch".to_owned(),
                "amd64".to_owned(),
                "--foreign-arch".to_owned(),
                "i386".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf apply should succeed on source root");

    let export_path = tempdir.path().join("phase8.eldastate");
    run_from_root(
        &source_root,
        CommandRequest::new(
            vec!["state".to_owned(), "export".to_owned()],
            vec![export_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("state export should succeed");

    copy_dir_recursive(
        &source_root.join("etc/elda/recipes"),
        &target_root.join("etc/elda/recipes"),
    )
    .expect("recipes should copy to target root");

    run_from_root(
        &target_root,
        CommandRequest::new(
            vec!["state".to_owned(), "import".to_owned()],
            vec![export_path.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("state import should succeed on target root");

    let profile = run_from_root(
        &target_root,
        CommandRequest::new(
            vec!["pf".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf show should succeed");
    assert!(
        profile
            .details
            .as_ref()
            .and_then(|details| details.get("active_profiles"))
            .and_then(|profiles| profiles.as_array())
            .is_some_and(|profiles| profiles
                .iter()
                .any(|value| value.as_str() == Some("phase8-core")))
    );
    assert_eq!(
        profile
            .details
            .as_ref()
            .and_then(|details| details.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|value| value.as_str()),
        Some("dinit")
    );
    assert!(
        profile
            .details
            .as_ref()
            .and_then(|details| details.get("foreign_arches"))
            .and_then(|arches| arches.as_array())
            .is_some_and(|arches| arches.iter().any(|value| value.as_str() == Some("i386")))
    );
    assert_eq!(
        run_installed_binary(&target_root, "/opt/elda/bin/phase8-base-tool"),
        "binary lane"
    );
}

fn copy_dir_recursive(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
