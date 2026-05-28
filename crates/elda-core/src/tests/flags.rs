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

#[test]
fn cardinality_one_of_blocks_install_when_unsatisfied() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "gpu-tool");
    write_cardinality_recipe(tempdir.path(), &repo_dir, "gpu-tool");

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["gpu-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect_err("install should fail because no GPU flag is selected");

    assert!(
        error.to_string().contains("requires exactly one of"),
        "expected one-of cardinality error, got: {error}"
    );
}

#[test]
fn cardinality_one_of_passes_with_explicit_flag_selection() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "gpu-tool");
    write_cardinality_recipe(tempdir.path(), &repo_dir, "gpu-tool");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["gpu-tool".to_owned(), "--use=+intel".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("install with explicit flag should plan");
    assert_eq!(report.area, "plan");
}

#[test]
fn conditional_dependency_is_skipped_without_flag() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "conditional-tool");
    write_conditional_dep_recipe(tempdir.path(), &repo_dir, "conditional-tool");

    // No --use, the conditional dep on `phantom-pkg` should not be triggered.
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["conditional-tool".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("install plan should succeed without conditional dep");
    let actions = report
        .details
        .and_then(|details| details.get("plan").cloned())
        .and_then(|plan| plan.get("actions").cloned())
        .and_then(|actions| actions.as_array().cloned())
        .unwrap_or_default();
    assert!(
        !actions.iter().any(|action| {
            action
                .get("target")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value == "phantom-pkg")
        }),
        "conditional dependency should not appear when its flag is off, got: {actions:?}"
    );
}

#[test]
fn conditional_dependency_is_required_when_flag_enabled() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "conditional-tool");
    write_conditional_dep_recipe(tempdir.path(), &repo_dir, "conditional-tool");

    // With +wayland the conditional dep on `phantom-pkg` becomes required and
    // the planner must fail closed because the package is unknown.
    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["conditional-tool".to_owned(), "--use=+wayland".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect_err("conditional dep should be required when flag is enabled");

    let message = error.to_string();
    assert!(
        message.contains("phantom-pkg"),
        "expected error to mention conditional dep `phantom-pkg`, got: {message}"
    );
}

#[test]
fn atom_versioned_package_flag_overrides_apply_only_when_version_matches() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[flags.package.\"flagged-tool>=1\"]\nwayland = true\n",
    );
    let repo_dir = create_git_cargo_repo(tempdir.path(), "flagged-tool");
    let binary_source = create_vendor_binary(tempdir.path(), "flagged-tool");
    write_flagged_dual_lane_recipe(tempdir.path(), &repo_dir, &binary_source, "flagged-tool");

    // The recipe is version 0.1.0; `>= 1` should not match, so wayland stays
    // disabled and the variant_id remains "default".
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fl".to_owned(), "check".to_owned()],
            vec!["flagged-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("fl check should succeed");
    let variant = report
        .details
        .as_ref()
        .and_then(|details| details.get("flag_state"))
        .and_then(|state| state.get("variant_id"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    assert_eq!(variant.as_deref(), Some("default"));

    // Switching the atom to `>= 0.1.0` should match the recipe and enable
    // wayland, producing a customized variant id.
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[flags.package.\"flagged-tool>=0.1.0\"]\nwayland = true\n",
    );
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fl".to_owned(), "check".to_owned()],
            vec!["flagged-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("fl check should succeed for matching atom");
    let variant = report
        .details
        .as_ref()
        .and_then(|details| details.get("flag_state"))
        .and_then(|state| state.get("variant_id"))
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    assert!(
        variant.as_deref().is_some_and(|value| value != "default"),
        "expected non-default variant for matching atom-versioned override"
    );
}

#[test]
fn flag_descriptions_are_surfaced_in_fl_check_output() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_cargo_repo(tempdir.path(), "gpu-tool");
    write_cardinality_recipe(tempdir.path(), &repo_dir, "gpu-tool");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["fl".to_owned(), "check".to_owned()],
            vec!["gpu-tool".to_owned(), "--use=+intel".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("fl check should succeed when cardinality is satisfied");

    let descriptions = report
        .details
        .as_ref()
        .and_then(|details| details.get("flag_state"))
        .and_then(|state| state.get("descriptions"))
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    assert!(
        descriptions
            .get("intel")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.contains("Intel"))
    );

    let groups = report
        .details
        .as_ref()
        .and_then(|details| details.get("flag_state"))
        .and_then(|state| state.get("cardinality_groups"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(groups.iter().any(|group| {
        group.get("name").and_then(|value| value.as_str()) == Some("gpu")
            && group.get("kind").and_then(|value| value.as_str()) == Some("one-of")
    }));
}

#[test]
fn upgrade_with_rebuild_variant_drift_rebuilds_drifted_packages() {
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

    let plan = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["--rebuild-variant-drift".to_owned()],
            OutputMode::Json,
            true,
        ),
    )
    .expect("rebuild-variant-drift dry run should succeed");
    let actions = plan
        .details
        .as_ref()
        .and_then(|details| details.get("plan"))
        .and_then(|plan| plan.get("actions"))
        .and_then(|actions| actions.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(actions.iter().any(|action| {
        action.get("target").and_then(|value| value.as_str()) == Some("flagged-tool")
            && action
                .get("variant_changed")
                .and_then(|value| value.as_bool())
                == Some(true)
            && action.get("needs_change").and_then(|value| value.as_bool()) == Some(true)
    }));

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["u".to_owned()],
            vec!["--rebuild-variant-drift".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("rebuild-variant-drift run should succeed");
    let upgrades = report
        .details
        .as_ref()
        .and_then(|details| details.get("upgrades"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(upgrades.iter().any(|entry| {
        entry.get("target").and_then(|value| value.as_str()) == Some("flagged-tool")
    }));
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

fn write_cardinality_recipe(root: &std::path::Path, repo_dir: &std::path::Path, name: &str) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{ kind = \"git\", url = \"file://{repo}\", branch = \"main\" }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  flags_default = {{ intel = false, nvidia = false, radeon = false }},\n  flags_allowed = {{ intel = true, nvidia = true, radeon = true }},\n  flags_descriptions = {{\n    intel = \"Enable Intel GPU support\",\n    nvidia = \"Enable NVIDIA GPU support\",\n    radeon = \"Enable Radeon GPU support\",\n  }},\n  flags_required_one_of = {{ gpu = {{ \"intel\", \"nvidia\", \"radeon\" }} }},\n}}\n",
            name = name,
            repo = repo_dir.display(),
        ),
    )
    .expect("cardinality pkg.lua should be written");
}

fn write_conditional_dep_recipe(root: &std::path::Path, repo_dir: &std::path::Path, name: &str) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{ kind = \"git\", url = \"file://{repo}\", branch = \"main\" }},\n  depends = {{\n    {{ name = \"phantom-pkg\", when = \"+wayland\" }},\n  }},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  flags_default = {{ wayland = false }},\n  flags_allowed = {{ wayland = true }},\n}}\n",
            name = name,
            repo = repo_dir.display(),
        ),
    )
    .expect("conditional dep pkg.lua should be written");
}
