use super::support::*;
use super::*;

#[test]
fn git_tags_lists_and_normalizes_local_repo_tags() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let repo_dir = create_git_make_repo(tempdir.path(), "tagged-tool");
    run_git(&repo_dir, &["tag", "v1.2.3"]);
    run_git(&repo_dir, &["tag", "v1.3.0-rc1"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["git".to_owned(), "tags".to_owned()],
            vec![
                repo_dir.display().to_string(),
                "--max-tags".to_owned(),
                "5".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("git tags should succeed");

    assert_eq!(report.area, "git");
    let details = report.details.as_ref().expect("details should exist");
    let tags = details["git_tags"]["tags"]
        .as_array()
        .expect("tags should be an array");
    assert!(tags.iter().any(|tag| {
        tag["tag"] == "v1.2.3"
            && tag["normalized_version"] == "0:1.2.3-1"
            && tag["version_confidence"] == "stable-semver"
    }));
    assert!(tags.iter().any(|tag| {
        tag["tag"] == "v1.3.0-rc1"
            && tag["normalized_version"] == "0:1.3.0rc1-1"
            && tag["version_confidence"] == "semver-prerelease"
    }));
}

#[test]
fn human_git_tags_renders_operator_dense_tag_rows() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let repo_dir = create_git_make_repo(tempdir.path(), "human-tags-tool");
    run_git(&repo_dir, &["tag", "v2.0.0"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["git".to_owned(), "tags".to_owned()],
            vec![repo_dir.display().to_string()],
            OutputMode::Human,
            false,
        ),
    )
    .expect("git tags should succeed");
    let rendered = crate::render_human(&report);

    assert!(rendered.contains("git ok"));
    assert!(rendered.contains("Git Tags"));
    assert!(rendered.contains("v2.0.0: 0:2.0.0-1 [stable-semver]"));
}

#[test]
fn human_git_tags_renders_joined_release_assets() {
    let report = crate::CommandReport {
        area: "git",
        status: "ok",
        exit_status: elda_types::ExitStatus::Success,
        command_path: vec!["git".to_owned(), "tags".to_owned()],
        operands: vec!["Mjoyufull/fsel".to_owned(), "--with-releases".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "found 1 version candidate(s) for `Mjoyufull/fsel`.".to_owned(),
        details: Some(serde_json::json!({
            "git_tags": {
                "target": "Mjoyufull/fsel",
                "tags": [{
                    "tag": "v3.4.1",
                    "object": "abc123",
                    "normalized_version": "0:3.4.1-1",
                    "version_confidence": "stable-semver"
                }]
            },
            "release_join": {
                "source": "github-api",
                "repo": "Mjoyufull/fsel",
                "joined_tags": [{
                    "tag": "v3.4.1",
                    "has_release": true,
                    "recommended_asset": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                    "asset_count": 2
                }]
            }
        })),
    };

    let rendered = crate::render_human(&report);

    assert!(rendered.contains("release=yes assets=2"));
    assert!(rendered.contains("recommended=fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz"));
}

#[test]
fn versions_alias_lists_git_tag_version_candidates() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let repo_dir = create_git_make_repo(tempdir.path(), "versions-tool");
    run_git(&repo_dir, &["tag", "v3.1.4"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["versions".to_owned()],
            vec![repo_dir.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("versions should succeed");

    let details = report.details.as_ref().expect("details should exist");
    assert_eq!(report.area, "git");
    assert_eq!(details["git_tags"]["tags"][0]["tag"], "v3.1.4");
    assert_eq!(
        details["git_tags"]["tags"][0]["normalized_version"],
        "0:3.1.4-1"
    );
}

#[test]
fn git_tags_honor_configured_tag_policy_defaults() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    fs::write(
        tempdir.path().join("etc/elda/config.toml"),
        r#"[defaults]
prefix = "/opt/elda"
allow_system_mode = true
install_preference = "binary"

[git]
include_prereleases = false
allow_date_versions = false
max_tags = 1
"#,
    )
    .expect("config should be written");
    let repo_dir = create_git_make_repo(tempdir.path(), "policy-tags-tool");
    run_git(&repo_dir, &["tag", "v1.0.0-rc1"]);
    run_git(&repo_dir, &["tag", "20260504"]);
    run_git(&repo_dir, &["tag", "v1.0.0"]);
    run_git(&repo_dir, &["tag", "v2.0.0"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["git".to_owned(), "tags".to_owned()],
            vec![repo_dir.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("git tags should succeed");
    let details = report.details.as_ref().expect("details should exist");
    let tags = details["git_tags"]["tags"]
        .as_array()
        .expect("tags should be an array");

    assert_eq!(details["max_tags"], 1);
    assert_eq!(details["tag_options"]["include_prereleases"], false);
    assert_eq!(details["tag_options"]["allow_date_versions"], false);
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["tag"], "v2.0.0");
}

#[test]
fn human_git_releases_renders_operator_dense_asset_rows() {
    let report = crate::CommandReport {
        area: "git",
        status: "ok",
        exit_status: elda_types::ExitStatus::Success,
        command_path: vec!["git".to_owned(), "releases".to_owned()],
        operands: vec!["Mjoyufull/fsel".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "found 1 release candidate(s) for `Mjoyufull/fsel`.".to_owned(),
        details: Some(serde_json::json!({
            "git_releases": {
                "target": "Mjoyufull/fsel",
                "repo": "Mjoyufull/fsel",
                "source": "github-api",
                "releases": [{
                    "tag": "v3.4.1",
                    "name": "v3.4.1",
                    "normalized_version": "0:3.4.1-1",
                    "version_confidence": "stable-semver",
                    "prerelease": false,
                    "draft": false,
                    "published_at": "2026-04-01T00:00:00Z",
                    "recommended_asset": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                    "assets": [{
                        "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                        "url": "https://example.invalid/fsel.tar.gz",
                        "kind": "payload",
                        "format": "tar-gz",
                        "os": "linux",
                        "arch": "x86_64",
                        "libc": "gnu",
                        "compatibility": "native-exact",
                        "score": 31
                    }]
                }]
            },
            "max_releases": 1
        })),
    };

    let rendered = crate::render_human(&report);

    assert!(rendered.contains("git ok"));
    assert!(rendered.contains("Git Releases"));
    assert!(rendered.contains("v3.4.1: 0:3.4.1-1 [stable-semver]"));
    assert!(rendered.contains("recommended=fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz"));
    assert!(rendered.contains("payload/tar-gz native-exact score=31"));
}

#[test]
fn human_metadata_add_renders_release_option_asset_details() {
    let report = crate::CommandReport {
        area: "metadata",
        status: "ok",
        exit_status: elda_types::ExitStatus::Success,
        command_path: vec!["add".to_owned()],
        operands: vec!["https://github.com/Mjoyufull/fsel".to_owned()],
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "generated metadata for 1 target(s).".to_owned(),
        details: Some(serde_json::json!({
            "metadata_add": {
                "link_option_mode": "list-options",
                "targets": [{
                    "target": "https://github.com/Mjoyufull/fsel",
                    "recipe_name": "fsel",
                    "recipe_dir": "/tmp/elda/recipes/fsel",
                    "pkg_lua": "/tmp/elda/recipes/fsel/pkg.lua",
                    "selected_lane": "binary",
                    "selected_source_kind": "github_release",
                    "publish_ready": true,
                    "source_options": [{
                        "index": 1,
                        "strategy": "git_release",
                        "source_kind": "github_release",
                        "lane": "binary",
                        "confidence": "detected",
                        "summary": "GitHub release asset detected: v3.4.1 / fsel.tar.gz",
                        "selected": true,
                        "tag": "v3.4.1",
                        "asset": "fsel.tar.gz",
                        "compatibility": "native-exact",
                        "checksum_available": true
                    }, {
                        "index": 2,
                        "strategy": "git_source",
                        "source_kind": "git",
                        "lane": "source",
                        "confidence": "derived",
                        "summary": "Build from git source",
                        "selected": false,
                        "checksum_available": false
                    }],
                    "fields": []
                }]
            }
        })),
    };

    let rendered = crate::render_human(&report);

    assert!(rendered.contains("tag=v3.4.1"));
    assert!(rendered.contains("asset=fsel.tar.gz"));
    assert!(rendered.contains("compatibility=native-exact"));
    assert!(rendered.contains("* 1. git_release [github_release, detected]"));
    assert!(rendered.contains("checksum=sha256"));
}
