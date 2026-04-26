use elda_repo::{RemoteDocument, TrustMode, save_remote};

use super::support::*;
use super::*;

#[test]
fn local_recipe_binary_install_records_local_recipe_source_kind() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "local-tool");
    write_local_binary_recipe(tempdir.path(), "local-tool", &binary, &[]);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["local-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        installed_source_kind(tempdir.path(), "local-tool"),
        "local_recipe"
    );
}

#[test]
fn direct_git_install_records_git_source_kind() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "git-tool");
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
    .expect("git install should succeed");

    assert_eq!(installed_source_kind(tempdir.path(), "git-tool"), "git");
}

#[test]
fn synced_binary_install_records_repo_binary_source_kind() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let binary = create_vendor_binary(tempdir.path(), "remote-tool");
    let index_path = write_remote_index(tempdir.path(), "remote-tool", &binary);
    save_remote(
        &tempdir.path().join("etc/elda/remotes.d"),
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_remote_key_fingerprint()],
            allow_stale: false,
            priority: 100,
        },
    )
    .expect("remote should be saved");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["remote-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert_eq!(
        installed_source_kind(tempdir.path(), "remote-tool"),
        "repo_binary"
    );
}

fn installed_source_kind(root: &std::path::Path, package_name: &str) -> String {
    run_from_root(
        root,
        CommandRequest::new(
            vec!["info".to_owned()],
            vec![package_name.to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed")
    .details
    .and_then(|details| details.get("installed").cloned())
    .and_then(|installed| installed.get("source_kind").cloned())
    .and_then(|value| value.as_str().map(str::to_owned))
    .expect("installed source kind should be present")
}
