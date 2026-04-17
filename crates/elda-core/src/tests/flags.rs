use super::support::*;
use super::*;

#[test]
fn custom_variant_falls_back_to_source_lane_and_records_variant_id() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "flagged-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "flagged-tool");
    write_flagged_dual_lane_recipe(tempdir.path(), &repo_dir, &binary_source, "flagged-tool");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["flagged-tool".to_owned(), "--use=+wayland".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert!(
        report
            .details
            .and_then(|details| details.get("installs").cloned())
            .and_then(|installs| installs.as_array().cloned())
            .is_some_and(|installs| installs.iter().any(|install| {
                install.get("selected_lane").and_then(|lane| lane.as_str()) == Some("source")
                    && install
                        .get("flag_state")
                        .and_then(|state| state.get("variant_id"))
                        .and_then(|value| value.as_str())
                        .is_some_and(|value| value != "default")
            }))
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/flagged-tool"),
        "sample-tool"
    );

    let info = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["info".to_owned()],
            vec!["flagged-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed");
    assert!(
        info.details
            .and_then(|details| details.get("installed").cloned())
            .and_then(|installed| installed.get("variant_id").cloned())
            .and_then(|value| value.as_str().map(str::to_owned))
            .is_some_and(|value| value != "default")
    );
}

#[test]
fn explicit_binary_lane_rejects_custom_variant() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "flagged-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "flagged-tool");
    write_flagged_dual_lane_recipe(tempdir.path(), &repo_dir, &binary_source, "flagged-tool");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ib".to_owned()],
            vec!["flagged-tool".to_owned(), "--use=+wayland".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("explicit binary lane should fail for a custom variant");

    assert!(
        error
            .to_string()
            .contains("binary lane only supports the default variant")
    );
}

#[test]
fn flag_diff_reports_installed_variant_drift() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "flagged-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "flagged-tool");
    write_flagged_dual_lane_recipe(tempdir.path(), &repo_dir, &binary_source, "flagged-tool");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["flagged-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("default install should succeed");

    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[flags.package.flagged-tool]\nwayland = true\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fl".to_owned(), "diff".to_owned()],
            vec!["flagged-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("flag diff should succeed");
    let details = report.details.expect("flag diff should return details");

    assert_eq!(report.area, "flags");
    assert!(
        details
            .get("variant_changed")
            .cloned()
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    );
    assert!(
        details
            .get("changes")
            .cloned()
            .and_then(|changes| changes.as_array().cloned())
            .is_some_and(|changes| changes.iter().any(|change| {
                change.get("flag").and_then(|value| value.as_str()) == Some("wayland")
                    && change.get("effective").and_then(|value| value.as_bool()) == Some(true)
            }))
    );
}

fn write_flagged_dual_lane_recipe(
    root: &std::path::Path,
    repo_dir: &std::path::Path,
    binary_source: &std::path::Path,
    name: &str,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    lanes = {{\n      source = {{\n        kind = \"git\",\n        url = \"file://{repo}\",\n        branch = \"main\",\n      }},\n      binary = {{\n        kind = \"url_archive\",\n        url = \"file://{binary}\",\n        sha256 = \"{sha256}\",\n        rename = \"{name}\",\n      }},\n    }},\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  flags_default = {{\n    wayland = false,\n    x11 = false,\n  }},\n  flags_allowed = {{\n    wayland = true,\n    x11 = true,\n  }},\n  flags_conflicts = {{\n    wayland = {{ \"x11\" }},\n    x11 = {{ \"wayland\" }},\n  }},\n}}\n",
            name = name,
            repo = repo_dir.display(),
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
        ),
    )
    .expect("flagged pkg.lua should be written");
}
