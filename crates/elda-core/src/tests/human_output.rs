use crate::{CommandReport, render_human};
use elda_types::ExitStatus;
use serde_json::json;

use super::support::*;
use super::*;

#[test]
fn human_install_dry_run_renders_structured_sections() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "render-plan-tool");
    write_local_binary_recipe(tempdir.path(), "render-plan-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["render-plan-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("dry-run install should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("Target\n  requested: render-plan-tool"));
    assert!(rendered.contains("  backend: prefix-copy"));
    assert!(rendered.contains("Resolution\n  selected package: render-plan-tool"));
    assert!(rendered.contains("Plan\n  install-explicit render-plan-tool"));
    assert!(rendered.contains("Progress\n  render-plan-tool:\n    planned fetch-binary:"));
    assert!(!rendered.contains("\"plan\""));
}

#[test]
fn human_install_success_renders_result_block() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let binary = create_vendor_binary(tempdir.path(), "render-result-tool");
    write_local_binary_recipe(tempdir.path(), "render-result-tool", &binary, &[]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["render-result-tool".to_owned()],
            OutputMode::Human,
            false,
        ),
    )
    .expect("install should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("Target\n  requested: render-result-tool"));
    assert!(rendered.contains("Plan\n  install-explicit render-result-tool"));
    assert!(rendered.contains("    done activate: backend prefix-copy"));
    assert!(rendered.contains("Progress\n  render-result-tool:\n    done fetch-binary:"));
    assert!(
        rendered.contains("Result\n  render-result-tool 0:0.1.0-1 -> installed")
            && rendered.contains("backend prefix-copy")
    );
    assert!(rendered.contains("Log\n  path:"));
    assert!(!rendered.contains("\"installs\""));
}

#[test]
fn human_direct_git_dry_run_surfaces_generated_metadata_path() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "render-git-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url],
            OutputMode::Human,
            true,
        ),
    )
    .expect("dry-run install should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("generated metadata: "));
    assert!(rendered.contains("/etc/elda/recipes/render-git-tool"));
}

#[test]
fn human_install_render_includes_snapshot_summary_when_present() {
    let report = CommandReport {
        area: "install",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["i".to_owned()],
        operands: vec!["system-tool".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "installed 1 target(s) into the current Elda root.".to_owned(),
        details: Some(json!({
            "layout": {
                "mode": "system",
                "prefix": "/usr"
            },
            "installs": [{
                "action": "install-explicit",
                "package": {
                    "package_name": "system-tool",
                    "epoch": 0,
                    "pkgver": "0.1.0",
                    "pkgrel": 1
                },
                "selected_lane": "binary",
                "selected_source_kind": "github_release",
                "status": "installed",
                "install_reason": "explicit",
                "activation_backend": "linux-copy",
                "progress": [{
                    "step": "snapshot-hooks",
                    "status": "done",
                    "detail": "2 request(s) via snapper, 2 captured"
                }],
                "install": {
                    "state_id": "system-1",
                    "installed_paths": 42,
                    "snapshots": [
                        { "phase": "pre-activation", "tool": "snapper", "status": "captured", "snapshot_id": "1" },
                        { "phase": "post-activation", "tool": "snapper", "status": "captured", "snapshot_id": "2" }
                    ]
                }
            }]
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("  backend: linux-copy"));
    assert!(
        rendered.contains(
            "system-tool:\n    done snapshot-hooks: 2 request(s) via snapper, 2 captured"
        )
    );
    assert!(rendered.contains("snapshots 2 via snapper, 2 captured"));
}

#[test]
fn human_ci_pr_render_surfaces_target_branch_and_review_metadata() {
    let report = CommandReport {
        area: "ci",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["ci".to_owned(), "pr".to_owned()],
        operands: vec!["123-origin-tool".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "created hosted review for `123-origin-tool`.".to_owned(),
        details: Some(json!({
            "submission_id": "123-origin-tool",
            "mode": "pr",
            "state": "submitted-for-review",
            "branch_name": "elda/origin-tool",
            "target_branch": "stable",
            "remote_name": "upstream",
            "pushed_ref": "refs/heads/elda/origin-tool",
            "review_kind": "github-pr",
            "review_id": "42",
            "pr_url": "https://github.com/yoka-ci/pkgs/pull/42"
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains(
        "Reference
  submission: 123-origin-tool"
    ));
    assert!(rendered.contains("  target branch: stable"));
    assert!(rendered.contains("  remote: upstream"));
    assert!(rendered.contains("  review kind: github-pr"));
    assert!(rendered.contains("  review id: 42"));
    assert!(rendered.contains("  review URL: https://github.com/yoka-ci/pkgs/pull/42"));
}

#[test]
fn human_ci_logs_render_surfaces_log_content() {
    let report = CommandReport {
        area: "ci",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["ci".to_owned(), "logs".to_owned()],
        operands: vec!["123-origin-tool".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "reported logs for `123-origin-tool`.".to_owned(),
        details: Some(json!({
            "submission_id": "123-origin-tool",
            "state": "published",
            "attempts": 2,
            "log_path": "/tmp/ci-origin-tool.log",
            "content": "scheduler_start attempt=2
        scheduler_complete attempt=2"
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains(
        "Log
  submission: 123-origin-tool"
    ));
    assert!(rendered.contains("  attempts: 2"));
    assert!(rendered.contains("  path: /tmp/ci-origin-tool.log"));
    assert!(rendered.contains(
        "Content
  scheduler_start attempt=2"
    ));
    assert!(rendered.contains("  scheduler_complete attempt=2"));
}

#[test]
fn human_ci_submission_render_surfaces_remote_publication_details() {
    let report = CommandReport {
        area: "ci",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["ci".to_owned(), "sub".to_owned()],
        operands: vec!["origin-tool".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "registered ci submission `123-origin-tool` and pushed it to `origin`.".to_owned(),
        details: Some(json!({
            "submission": {
                "id": "123-origin-tool",
                "requested_targets": ["origin-tool"],
                "packages": ["origin-tool"],
                "branch_name": "elda/origin-tool",
                "target_branch": "main",
                "mode": "pr",
                "state": "submitted-for-review",
                "immediate": false,
                "batch_name": null,
                "created_at": 1,
                "updated_at": 1,
                "attempts": 2,
                "planned_layers": 3,
                "completed_layers": 3,
                "queued_at": 1,
                "started_at": 2,
                "completed_at": 3,
                "last_error": null,
                "issues": [],
                "published_packages": [],
                "lock_path": null,
                "index_path": null,
                "signature_path": null,
                "log_path": "/tmp/ci-origin-tool.log",
                "packages_repo_path": "/tmp/ci/pkgs",
                "trusted_key_fingerprint": null,
                "repo_commit": "abc123",
                "remote_name": "origin",
                "remote_url": "https://github.com/yoka-ci/pkgs.git",
                "pushed_ref": "refs/heads/elda/origin-tool",
                "pushed_commit": "abc123",
                "pushed_at": 1
            }
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("Submission\n  id: 123-origin-tool"));
    assert!(rendered.contains("  state: submitted-for-review"));
    assert!(rendered.contains("Remote\n  remote: origin"));
    assert!(
        rendered.contains("  url: https://github.com/yoka-ci/pkgs.git")
            && rendered.contains("  ref: refs/heads/elda/origin-tool")
    );
    assert!(rendered.contains("Artifacts\n  packages repo: /tmp/ci/pkgs"));
}
