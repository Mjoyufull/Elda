use super::support::*;
use super::*;

use std::io::Cursor;

use serde_json::Value;

#[test]
fn ci_run_publishes_a_signed_remote_that_sync_and_install_can_consume() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let source_repo = create_git_make_repo(tempdir.path(), "ci-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-tool");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-tool");

    let publish = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "run".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci run should succeed");

    let submission = publish
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("ci run should report a submission");
    let index_path = submission
        .get("index_path")
        .and_then(|value| value.as_str())
        .expect("ci run should report an index path");
    let packages_repo_path = submission
        .get("packages_repo_path")
        .and_then(|value| value.as_str())
        .expect("ci run should report a packages repo path");
    let lock_path = submission
        .get("lock_path")
        .and_then(|value| value.as_str())
        .expect("ci run should report a compressed lock path");
    let trusted_key = submission
        .get("trusted_key_fingerprint")
        .and_then(|value| value.as_str())
        .expect("ci run should report a trusted key fingerprint");
    let published_package = submission
        .get("published_packages")
        .and_then(|value| value.as_array())
        .and_then(|packages| packages.first())
        .expect("ci run should report a published package");

    assert!(lock_path.ends_with("lock-v1.json.zst"));

    let lock_bytes = fs::read(lock_path).expect("compressed lock should exist");
    let lock_json = zstd::decode_all(Cursor::new(lock_bytes)).expect("lock should decompress");
    let lock_document: Value =
        serde_json::from_slice(&lock_json).expect("lock json should deserialize");
    assert_eq!(
        lock_document.get("format_version").and_then(Value::as_u64),
        Some(1)
    );
    assert!(
        lock_document
            .get("packages")
            .and_then(Value::as_array)
            .is_some_and(|packages| !packages.is_empty())
    );

    let signature_path = published_package
        .get("signature_path")
        .and_then(|value| value.as_str())
        .expect("published package should report a signature sidecar");
    let sbom_path = published_package
        .get("sbom_path")
        .and_then(|value| value.as_str())
        .expect("published package should report an sbom sidecar");
    let attestation_path = published_package
        .get("attestation_path")
        .and_then(|value| value.as_str())
        .expect("published package should report an attestation sidecar");
    assert!(std::path::Path::new(signature_path).is_file());
    assert!(std::path::Path::new(sbom_path).is_file());
    assert!(std::path::Path::new(attestation_path).is_file());

    let published_index: Value =
        serde_json::from_slice(&fs::read(index_path).expect("published index should exist"))
            .expect("published index should deserialize");
    let indexed_package = published_index
        .get("packages")
        .and_then(Value::as_array)
        .and_then(|packages| packages.first())
        .expect("published index should contain a package");
    let expected_sbom_url = format!("file://{sbom_path}");
    let expected_attestation_url = format!("file://{attestation_path}");
    assert_eq!(
        indexed_package.get("sbom_url").and_then(Value::as_str),
        Some(expected_sbom_url.as_str())
    );
    assert_eq!(
        indexed_package
            .get("attestation_url")
            .and_then(Value::as_str),
        Some(expected_attestation_url.as_str())
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                format!("local-ci=file://{index_path}"),
                "--trust".to_owned(),
                "pinned".to_owned(),
                "--trusted-key".to_owned(),
                trusted_key.to_owned(),
                "--packages-url".to_owned(),
                format!("file://{packages_repo_path}"),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote add should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    let default_install = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("default published install should succeed");
    assert!(
        default_install
            .details
            .as_ref()
            .and_then(|details| details.get("installs"))
            .and_then(|installs| installs.as_array())
            .is_some_and(|installs| installs.iter().any(|install| {
                install
                    .get("selected_lane")
                    .and_then(|lane| lane.as_str())
                    .is_some_and(|lane| lane == "binary")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/ci-tool"),
        "binary lane"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");
    let source_install = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("source override install should succeed");
    assert!(
        source_install
            .details
            .as_ref()
            .and_then(|details| details.get("installs"))
            .and_then(|installs| installs.as_array())
            .is_some_and(|installs| installs.iter().any(|install| {
                install
                    .get("selected_lane")
                    .and_then(|lane| lane.as_str())
                    .is_some_and(|lane| lane == "source")
            }))
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["rm".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remove should succeed");
    let binary_install = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ib".to_owned()],
            vec!["ci-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("binary override install should succeed");
    assert!(
        binary_install
            .details
            .as_ref()
            .and_then(|details| details.get("installs"))
            .and_then(|installs| installs.as_array())
            .is_some_and(|installs| installs.iter().any(|install| {
                install
                    .get("selected_lane")
                    .and_then(|lane| lane.as_str())
                    .is_some_and(|lane| lane == "binary")
            }))
    );
}

#[test]
fn ci_pr_reports_compare_url_when_origin_is_configured() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let source_repo = create_git_make_repo(tempdir.path(), "ci-pr-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-pr-tool");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-pr-tool");

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-pr-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_id = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("id"))
        .and_then(|value| value.as_str())
        .expect("submission id should be reported")
        .to_owned();
    let packages_repo_path = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("packages_repo_path"))
        .and_then(|value| value.as_str())
        .expect("packages repo path should be reported")
        .to_owned();

    run_git(
        std::path::Path::new(&packages_repo_path),
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/yoka-ci/pkgs.git",
        ],
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "pr".to_owned()],
            vec![submission_id],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci pr should succeed");

    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("pr_url"))
            .and_then(|value| value.as_str()),
        Some("https://github.com/yoka-ci/pkgs/compare/main...elda%2Fci-pr-tool?expand=1")
    );
}

#[test]
fn ci_sub_pushes_submission_branch_to_origin_when_configured() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[submission]\nmode = \"pr\"\n",
    );

    let source_repo = create_git_make_repo(tempdir.path(), "ci-origin-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-origin-tool");
    let origin_repo = create_bare_git_remote(tempdir.path(), "ci-origin-remote");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-origin-tool");

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-origin-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("ci sub dry-run should succeed");
    let packages_repo_path = dry_run
        .details
        .as_ref()
        .and_then(|details| details.get("workspace"))
        .and_then(|workspace| workspace.get("packages_repo_dir"))
        .and_then(|value| value.as_str())
        .expect("workspace repo path should be reported")
        .to_owned();
    let origin_path = origin_repo.display().to_string();
    run_git(
        std::path::Path::new(&packages_repo_path),
        &["remote", "add", "origin", &origin_path],
    );

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-origin-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_record = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("submission should be reported");
    let pushed_ref = submission_record
        .get("pushed_ref")
        .and_then(|value| value.as_str())
        .expect("pushed ref should be recorded");
    let pushed_commit = submission_record
        .get("pushed_commit")
        .and_then(|value| value.as_str())
        .expect("pushed commit should be recorded");

    assert_eq!(
        submission_record
            .get("state")
            .and_then(|value| value.as_str()),
        Some("submitted-for-review")
    );
    assert_eq!(
        submission_record
            .get("remote_name")
            .and_then(|value| value.as_str()),
        Some("origin")
    );
    assert_eq!(
        submission_record
            .get("remote_url")
            .and_then(|value| value.as_str()),
        Some(origin_path.as_str())
    );
    assert_eq!(pushed_ref, "refs/heads/elda/ci-origin-tool");

    let remote_commit = std::process::Command::new("git")
        .arg("-C")
        .arg(&origin_repo)
        .args(["rev-parse", pushed_ref])
        .output()
        .expect("git rev-parse should launch");
    assert!(remote_commit.status.success());
    assert_eq!(
        String::from_utf8_lossy(&remote_commit.stdout).trim(),
        pushed_commit
    );
}

#[test]
fn ci_sub_push_mode_updates_origin_main() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[submission]\nmode = \"push\"\n",
    );

    let source_repo = create_git_make_repo(tempdir.path(), "ci-push-mode-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-push-mode-tool");
    let origin_repo = create_bare_git_remote(tempdir.path(), "ci-push-mode-remote");
    write_dual_lane_recipe(tempdir.path(), &source_repo, &binary, "ci-push-mode-tool");

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-push-mode-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("ci sub dry-run should succeed");
    let packages_repo_path = dry_run
        .details
        .as_ref()
        .and_then(|details| details.get("workspace"))
        .and_then(|workspace| workspace.get("packages_repo_dir"))
        .and_then(|value| value.as_str())
        .expect("workspace repo path should be reported")
        .to_owned();
    let origin_path = origin_repo.display().to_string();
    run_git(
        std::path::Path::new(&packages_repo_path),
        &["remote", "add", "origin", &origin_path],
    );

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-push-mode-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_record = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("submission should be reported");
    let pushed_commit = submission_record
        .get("pushed_commit")
        .and_then(|value| value.as_str())
        .expect("pushed commit should be recorded");

    assert_eq!(
        submission_record
            .get("state")
            .and_then(|value| value.as_str()),
        Some("pushed")
    );
    assert_eq!(
        submission_record
            .get("mode")
            .and_then(|value| value.as_str()),
        Some("push")
    );
    assert_eq!(
        submission_record
            .get("pushed_ref")
            .and_then(|value| value.as_str()),
        Some("refs/heads/main")
    );

    let remote_commit = std::process::Command::new("git")
        .arg("-C")
        .arg(&origin_repo)
        .args(["rev-parse", "refs/heads/main"])
        .output()
        .expect("git rev-parse should launch");
    assert!(remote_commit.status.success());
    assert_eq!(
        String::from_utf8_lossy(&remote_commit.stdout).trim(),
        pushed_commit
    );
}

#[test]
fn ci_sub_uses_configured_remote_and_base_branch() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        r#"
[submission]
mode = "push"
remote_name = "upstream"
base_branch = "stable"
"#,
    );

    let source_repo = create_git_make_repo(tempdir.path(), "ci-configured-remote-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-configured-remote-tool");
    let origin_repo = create_bare_git_remote(tempdir.path(), "ci-configured-remote");
    write_dual_lane_recipe(
        tempdir.path(),
        &source_repo,
        &binary,
        "ci-configured-remote-tool",
    );

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-configured-remote-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("ci sub dry-run should succeed");
    let packages_repo_path = dry_run
        .details
        .as_ref()
        .and_then(|details| details.get("workspace"))
        .and_then(|workspace| workspace.get("packages_repo_dir"))
        .and_then(|value| value.as_str())
        .expect("workspace repo path should be reported")
        .to_owned();
    let origin_path = origin_repo.display().to_string();
    run_git(
        std::path::Path::new(&packages_repo_path),
        &["remote", "add", "upstream", &origin_path],
    );

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-configured-remote-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_record = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .expect("submission should be reported");

    assert_eq!(
        submission_record
            .get("remote_name")
            .and_then(|value| value.as_str()),
        Some("upstream")
    );
    assert_eq!(
        submission_record
            .get("remote_url")
            .and_then(|value| value.as_str()),
        Some(origin_path.as_str())
    );
    assert_eq!(
        submission_record
            .get("pushed_ref")
            .and_then(|value| value.as_str()),
        Some("refs/heads/stable")
    );

    let remote_commit = std::process::Command::new("git")
        .arg("-C")
        .arg(&origin_repo)
        .args(["rev-parse", "refs/heads/stable"])
        .output()
        .expect("git rev-parse should launch");
    assert!(remote_commit.status.success());
}
