use super::support::*;
use super::*;

#[test]
fn ad_hoc_git_branch_upgrade_tracks_persisted_source_ref() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "branch-up-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url.clone()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("git install should succeed");
    let installed_v1 = installed_git_details(tempdir.path(), "branch-up-tool");

    fs::write(
        repo_dir.join("branch-up-tool"),
        "#!/bin/sh\necho make tool v2\n",
    )
    .expect("updated script should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "second"]);
    let repo_v2 = git_head_commit(&repo_dir);

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["branch-up-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("dry-run upgrade should succeed");
    let action =
        dry_run.details.as_ref().expect("details should exist")["plan"]["actions"][0].clone();

    assert_eq!(action["action"], "upgrade-target");
    assert_eq!(
        action["installed_repo_commit"].as_str(),
        installed_v1.repo_commit.as_deref()
    );
    assert_eq!(
        action["candidate_repo_commit"].as_str(),
        Some(repo_v2.as_str())
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["branch-up-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("upgrade should succeed");

    let installed_v2 = installed_git_details(tempdir.path(), "branch-up-tool");
    assert_eq!(installed_v2.repo_commit.as_deref(), Some(repo_v2.as_str()));
    assert_eq!(installed_v2.source_ref.as_deref(), Some(repo_url.as_str()));
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/branch-up-tool"),
        "make tool v2"
    );
}

#[test]
fn ad_hoc_git_tag_install_does_not_auto_advance_on_plain_upgrade() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "tag-pin-tool");
    run_git(&repo_dir, &["tag", "v1.0.0"]);
    let repo_url = format!("file://{}", repo_dir.display());

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url.clone(), "--to-tag=v1.0.0".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("tag install should succeed");
    let installed_v1 = installed_git_details(tempdir.path(), "tag-pin-tool");

    fs::write(
        repo_dir.join("tag-pin-tool"),
        "#!/bin/sh\necho tagged tool v2\n",
    )
    .expect("updated script should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "second"]);
    run_git(&repo_dir, &["tag", "v2.0.0"]);

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["tag-pin-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("dry-run upgrade should succeed");
    let actions = dry_run.details.as_ref().expect("details should exist")["plan"]["actions"]
        .as_array()
        .expect("actions should be an array");

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["action"], "keep-installed");
    assert_eq!(actions[0]["blocked_reason"], "git-ref-pinned");
    assert_eq!(actions[0]["ad_hoc_git_moving"], false);
    assert_eq!(
        actions[0]["source_ref"].as_str(),
        Some(format!("{repo_url}#tag:v1.0.0").as_str())
    );

    let human_dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["tag-pin-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("human dry-run upgrade should succeed");
    let rendered = crate::render_human(&human_dry_run);
    assert!(rendered.contains("policy:: git-ref-pinned"));
    assert!(rendered.contains("git ref:: pinned"));

    assert_eq!(
        installed_git_details(tempdir.path(), "tag-pin-tool").repo_commit,
        installed_v1.repo_commit
    );
}

#[test]
fn ad_hoc_git_upgrade_can_explicitly_switch_to_new_tag() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "tag-switch-tool");
    run_git(&repo_dir, &["tag", "v1.0.0"]);
    let repo_url = format!("file://{}", repo_dir.display());

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url.clone(), "--to-tag=v1.0.0".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("tag install should succeed");

    fs::write(
        repo_dir.join("tag-switch-tool"),
        "#!/bin/sh\necho tag switch v2\n",
    )
    .expect("updated script should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "second"]);
    run_git(&repo_dir, &["tag", "v2.0.0"]);
    let repo_v2 = git_head_commit(&repo_dir);

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["tag-switch-tool".to_owned(), "--to-tag=v2.0.0".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("dry-run upgrade should succeed");
    let action =
        dry_run.details.as_ref().expect("details should exist")["plan"]["actions"][0].clone();

    assert_eq!(action["action"], "upgrade-target");
    assert_eq!(
        action["candidate_repo_commit"].as_str(),
        Some(repo_v2.as_str())
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["tag-switch-tool".to_owned(), "--to-tag=v2.0.0".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("tag switch upgrade should succeed");

    let installed = installed_git_details(tempdir.path(), "tag-switch-tool");
    assert_eq!(installed.repo_commit.as_deref(), Some(repo_v2.as_str()));
    assert_eq!(
        installed.source_ref.as_deref(),
        Some(format!("{repo_url}#tag:v2.0.0").as_str())
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/tag-switch-tool"),
        "tag switch v2"
    );
}

#[test]
fn ad_hoc_git_downgrade_can_explicitly_switch_to_older_tag() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "tag-down-tool");
    run_git(&repo_dir, &["tag", "v1.0.0"]);
    fs::write(
        repo_dir.join("tag-down-tool"),
        "#!/bin/sh\necho tag downgrade v2\n",
    )
    .expect("updated script should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "second"]);
    run_git(&repo_dir, &["tag", "v2.0.0"]);
    let repo_url = format!("file://{}", repo_dir.display());

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec![repo_url.clone(), "--to-tag=v2.0.0".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("v2 install should succeed");

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["tag-down-tool".to_owned(), "--to-tag=v1.0.0".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("source-ref downgrade dry-run should succeed");
    assert_eq!(
        dry_run.details.as_ref().expect("details should exist")["plan"]["kind"],
        "source-ref-downgrade"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["downgrade".to_owned()],
            vec!["tag-down-tool".to_owned(), "--to-tag=v1.0.0".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("source-ref downgrade should succeed");

    let installed = installed_git_details(tempdir.path(), "tag-down-tool");
    assert_eq!(
        installed.source_ref.as_deref(),
        Some(format!("{repo_url}#tag:v1.0.0").as_str())
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/tag-down-tool"),
        "make tool"
    );
}

#[derive(Debug, Clone)]
struct InstalledGitDetails {
    source_ref: Option<String>,
    repo_commit: Option<String>,
}

fn installed_git_details(root: &std::path::Path, package_name: &str) -> InstalledGitDetails {
    let installed = run_from_root(
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
    .expect("installed details should exist");

    InstalledGitDetails {
        source_ref: installed
            .get("source_ref")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        repo_commit: installed
            .get("repo_commit")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
    }
}
