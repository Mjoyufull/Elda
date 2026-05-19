use super::support::*;
use super::*;

#[test]
fn install_fails_when_candidate_conflicts_with_installed_package() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let alpha_binary = create_vendor_binary(tempdir.path(), "alpha-tool");
    let beta_binary = create_vendor_binary(tempdir.path(), "beta-tool");
    write_local_binary_recipe(tempdir.path(), "alpha-tool", &alpha_binary, &[]);
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "beta-tool",
        &beta_binary,
        "0.1.0",
        "{}",
        "{}",
        "{}",
    );
    let beta_recipe_path = tempdir.path().join("etc/elda/recipes/beta-tool/pkg.lua");
    fs::write(
        &beta_recipe_path,
        format!(
            "pkg = {{\n  name = \"beta-tool\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"beta-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{ \"alpha-tool\" }},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
            binary = beta_binary.display(),
            sha256 = sha256_file(&beta_binary),
        ),
    )
    .expect("conflicting recipe should be written");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["alpha-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("conflicting install should fail");
    assert!(
        error
            .to_string()
            .contains("conflicts with installed package `alpha-tool`")
    );
}

#[test]
fn pin_and_hold_block_upgrades_until_cleared() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("policy-index.toml");
    let v1_binary = create_script_binary(tempdir.path(), "policy-tool-v1", "policy v1");
    write_remote_index_with_version(&index_path, "policy-tool", &v1_binary, "1.0.0");

    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["policy-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pin".to_owned()],
            vec!["policy-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pin should succeed");

    let v2_binary = create_script_binary(tempdir.path(), "policy-tool-v2", "policy v2");
    write_remote_index_with_version(&index_path, "policy-tool", &v2_binary, "1.1.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("second sync should succeed");

    let pinned_upgrade = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("upgrade should succeed");
    assert!(
        pinned_upgrade
            .details
            .and_then(|details| details.get("actions").cloned())
            .and_then(|actions| actions.as_array().cloned())
            .is_some_and(|actions| actions.iter().any(|action| {
                action
                    .get("blocked_reason")
                    .and_then(|reason| reason.as_str())
                    .is_some_and(|reason| reason == "pinned-version")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/policy-tool"),
        "policy v1"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["unpin".to_owned()],
            vec!["policy-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("unpin should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["hold".to_owned()],
            vec![
                "policy-tool".to_owned(),
                "--source".to_owned(),
                "main".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("hold should succeed");

    let v3_binary = create_script_binary(tempdir.path(), "policy-tool-v3", "policy v3");
    write_remote_index_with_version(&index_path, "policy-tool", &v3_binary, "1.2.0");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("third sync should succeed");

    let held_upgrade = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("held upgrade should succeed");
    assert!(
        held_upgrade
            .details
            .and_then(|details| details.get("actions").cloned())
            .and_then(|actions| actions.as_array().cloned())
            .is_some_and(|actions| actions.iter().any(|action| {
                action
                    .get("blocked_reason")
                    .and_then(|reason| reason.as_str())
                    .is_some_and(|reason| reason == "held")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/policy-tool"),
        "policy v1"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["unhold".to_owned()],
            vec!["policy-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("unhold should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["u".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("final upgrade should succeed");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/policy-tool"),
        "policy v3"
    );
}

#[test]
fn install_replaces_installed_package_from_same_origin() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let alpha_binary = create_vendor_binary(tempdir.path(), "alpha-tool");
    let beta_binary = create_vendor_binary(tempdir.path(), "beta-tool");

    write_local_binary_recipe(tempdir.path(), "alpha-tool", &alpha_binary, &[]);
    write_policy_recipe(
        tempdir.path(),
        "beta-tool",
        &beta_binary,
        "{}",
        &["alpha-tool"],
        &["alpha-tool"],
        &[],
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["alpha-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("alpha install should succeed");

    let dry_run = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("replacement install dry run should succeed");
    let rendered = crate::render_human(&dry_run);
    assert!(
        rendered.contains("replaces alpha-tool"),
        "human install plan should name replaced packages:\n{rendered}"
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("replacement install should succeed");

    assert!(!tempdir.path().join("opt/elda/bin/alpha-tool").exists());
    assert!(tempdir.path().join("opt/elda/bin/beta-tool").exists());

    let ls = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["ls".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("ls should succeed");
    let packages = ls
        .details
        .as_ref()
        .and_then(|details| details.get("packages"))
        .and_then(|packages| packages.as_array())
        .cloned()
        .expect("packages should be listed");

    assert!(packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("beta-tool")
    }));
    assert!(!packages.iter().any(|package| {
        package.get("pkgname").and_then(|value| value.as_str()) == Some("alpha-tool")
    }));
}

#[test]
fn install_rejects_replacement_that_breaks_reverse_dependencies() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let alpha_binary = create_vendor_binary(tempdir.path(), "alpha-tool");
    let gamma_binary = create_vendor_binary(tempdir.path(), "gamma-tool");
    let beta_binary = create_vendor_binary(tempdir.path(), "beta-tool");

    write_local_binary_recipe(tempdir.path(), "alpha-tool", &alpha_binary, &[]);
    write_local_binary_recipe(tempdir.path(), "gamma-tool", &gamma_binary, &["alpha-tool"]);
    write_policy_recipe(
        tempdir.path(),
        "beta-tool",
        &beta_binary,
        "{}",
        &["alpha-tool"],
        &["alpha-tool"],
        &[],
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["gamma-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("gamma install should succeed");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("replacement should fail when it breaks installed deps");

    assert!(
        error
            .to_string()
            .contains("installed package `gamma-tool` depends on `alpha-tool`")
    );
}

#[test]
fn install_rejects_replacement_across_source_boundaries() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let index_path = tempdir.path().join("replace-index.toml");
    let alpha_binary = create_script_binary(tempdir.path(), "alpha-remote-v1", "alpha remote");
    let beta_binary = create_vendor_binary(tempdir.path(), "beta-tool");

    write_remote_index_with_version(&index_path, "alpha-tool", &alpha_binary, "1.0.0");
    register_fixture_remote(tempdir.path(), "main", &index_path);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["alpha-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("remote alpha install should succeed");

    write_policy_recipe(
        tempdir.path(),
        "beta-tool",
        &beta_binary,
        "{}",
        &["alpha-tool"],
        &["alpha-tool"],
        &[],
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("cross-origin replacement should fail");

    assert!(error.to_string().contains("across source boundaries"));
}

fn write_policy_recipe(
    root: &std::path::Path,
    name: &str,
    binary_source: &std::path::Path,
    depends_lua: &str,
    conflicts: &[&str],
    replaces: &[&str],
    provides: &[&str],
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"{name}\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {provides},\n  conflicts = {conflicts},\n  replaces = {replaces},\n  conffiles = {{}},\n}}\n",
            name = name,
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
            depends = depends_lua,
            provides = string_array_lua(provides),
            conflicts = string_array_lua(conflicts),
            replaces = string_array_lua(replaces),
        ),
    )
    .expect("policy recipe should be written");
}
