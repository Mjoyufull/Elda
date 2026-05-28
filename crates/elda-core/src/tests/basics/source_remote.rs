use super::*;

fn make_source_recipe_lua(package_name: &str, source_repo: &std::path::Path) -> String {
    format!(
        "pkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{source_repo}\",\n    branch = \"main\",\n  }},\n  build = {{\n    system = \"make\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
        source_repo = source_repo.display(),
    )
}

fn make_dual_lane_recipe_lua(
    package_name: &str,
    source_repo: &std::path::Path,
    binary_source: &std::path::Path,
) -> String {
    format!(
        "pkg = {{\n  name = \"{package_name}\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    lanes = {{\n      source = {{\n        kind = \"git\",\n        url = \"file://{source_repo}\",\n        branch = \"main\",\n      }},\n      binary = {{\n        kind = \"url_archive\",\n        url = \"file://{binary_source}\",\n        sha256 = \"{binary_sha256}\",\n        rename = \"{package_name}\",\n      }},\n    }},\n  }},\n  build = {{\n    system = \"make\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
        source_repo = source_repo.display(),
        binary_source = binary_source.display(),
        binary_sha256 = sha256_file(binary_source),
    )
}

#[test]
fn synced_source_only_install_builds_from_remote_package_repo() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let source_repo = create_git_make_repo(tempdir.path(), "source-remote-tool");
    let pkg_lua = make_source_recipe_lua("source-remote-tool", &source_repo);
    let package_repo =
        create_package_definition_repo(tempdir.path(), "source-remote-tool", &pkg_lua, &[]);
    let repo_commit = git_head_commit(&package_repo);
    let index_path = tempdir.path().join("source-only-index.toml");
    write_remote_recipe_index(
        &index_path,
        "source-remote-tool",
        &pkg_lua,
        &repo_commit,
        None,
    );

    register_fixture_remote_with_packages(
        tempdir.path(),
        "main",
        &index_path,
        Some(&format!("file://{}", package_repo.display())),
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["source-remote-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("source-only remote install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/source-remote-tool"),
        "make tool"
    );
}

#[test]
fn synced_ig_uses_remote_package_repo_for_source_lane() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let source_repo = create_git_make_repo(tempdir.path(), "dual-remote-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "dual-remote-tool");
    let pkg_lua = make_dual_lane_recipe_lua("dual-remote-tool", &source_repo, &binary_source);
    let package_repo =
        create_package_definition_repo(tempdir.path(), "dual-remote-tool", &pkg_lua, &[]);
    let repo_commit = git_head_commit(&package_repo);
    let index_path = tempdir.path().join("dual-lane-index.toml");
    write_remote_recipe_index(
        &index_path,
        "dual-remote-tool",
        &pkg_lua,
        &repo_commit,
        Some(&binary_source),
    );

    register_fixture_remote_with_packages(
        tempdir.path(),
        "main",
        &index_path,
        Some(&format!("file://{}", package_repo.display())),
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["dual-remote-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("explicit source-lane remote install should succeed");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/dual-remote-tool"),
        "make tool"
    );
}

#[test]
fn synced_source_build_fails_when_remote_has_no_package_repo_url() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let source_repo = create_git_make_repo(tempdir.path(), "missing-packages-url-tool");
    let pkg_lua = make_source_recipe_lua("missing-packages-url-tool", &source_repo);
    let package_repo =
        create_package_definition_repo(tempdir.path(), "missing-packages-url-tool", &pkg_lua, &[]);
    let repo_commit = git_head_commit(&package_repo);
    let index_path = tempdir.path().join("missing-packages-url-index.toml");
    write_remote_recipe_index(
        &index_path,
        "missing-packages-url-tool",
        &pkg_lua,
        &repo_commit,
        None,
    );

    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["missing-packages-url-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("source build should fail without packages_url");

    assert!(error.to_string().contains("does not define `packages_url`"));
}
