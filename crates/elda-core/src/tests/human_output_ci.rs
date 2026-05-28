use crate::{CommandReport, render_human};
use elda_types::ExitStatus;
use serde_json::json;

use super::*;

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

#[test]
fn human_search_render_lists_numbered_matches_with_description() {
    let report = CommandReport {
        area: "search",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["search".to_owned()],
        operands: vec!["fsel".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "found 2 synced package match(es).".to_owned(),
        details: Some(json!({
            "query": "fsel",
            "regex": false,
            "interactive": false,
            "results": [
                {
                    "remote_name": "aur",
                    "pkgname": "fsel",
                    "epoch": 0,
                    "pkgver": "3.4.1",
                    "pkgrel": 1,
                    "description": "Fast TUI app launcher and fuzzy finder"
                },
                {
                    "remote_name": "aur",
                    "pkgname": "fselect",
                    "epoch": 0,
                    "pkgver": "0.10.0",
                    "pkgrel": 1,
                    "summary": "Find files with SQL-like queries"
                }
            ]
        })),
    };

    let rendered = render_human(&report);
    assert!(rendered.contains("query    fsel"));
    assert!(rendered.contains("matches  2"));
    assert!(rendered.contains("src  package"));
    assert!(rendered.contains("[L]  aur/fsel"));
    assert!(rendered.contains("0:3.4.1-1"));
    assert!(rendered.contains("Fast TUI app launcher and fuzzy finder"));
    assert!(rendered.contains("[L]  aur/fselect"));
    assert!(rendered.contains("Find files with SQL-like queries"));
}
