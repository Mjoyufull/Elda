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

    assert!(rendered.contains("┌─ install render-plan-tool"));
    assert!(rendered.contains("│  target:: render-plan-tool"));
    assert!(rendered.contains("│  version:: 0:0.1.0-1"));
    assert!(rendered.contains("│  source:: [E] binary/url_archive"));
    assert!(rendered.contains("│  activate:: /opt/elda (prefix) via prefix-copy"));
    assert!(rendered.contains("│  change:: install 1, keep 0, replace 0, weak 0"));
    assert!(rendered.contains("│  safety review::"));
    assert!(rendered.contains("└─ dry run"));
    assert!(!rendered.contains("├─ Target"));
    assert!(!rendered.contains("├─ Progress"));
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
        )
        .with_log_level(Some(1)),
    )
    .expect("install should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("┌─ installed render-result-tool"));
    assert!(rendered.contains("│  target:: render-result-tool"));
    assert!(rendered.contains("│  state:: "));
    assert!(rendered.contains("│  paths:: "));
    assert!(
        !rendered.contains("│  source "),
        "post-action render must not repeat source facts: {rendered}"
    );
    assert!(
        !rendered.contains("│  activate "),
        "post-action render must not repeat activation facts: {rendered}"
    );
    assert!(
        !rendered.contains("├─ Progress"),
        "post-action render must not duplicate the live progression: {rendered}"
    );
    assert!(rendered.contains(
        "Log
  path:"
    ));
    assert!(!rendered.contains("Install Result"));
    assert!(!rendered.contains("install: ok"));
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

    assert!(rendered.contains("metadata::"));
    assert!(rendered.contains("/etc/elda/recipes/render-git-tool"));
    assert!(rendered.contains("source:: [V]"));
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

    assert!(rendered.contains("│  target:: system-tool"));
    assert!(rendered.contains("│  state:: system-1"));
    assert!(rendered.contains("│  paths:: 42"));
    assert!(
        !rendered.contains("│  system-tool:\n│    done snapshot-hooks:"),
        "post-action human render duplicated the live progression: {rendered}"
    );
    assert!(rendered.contains("│  snapshots:: 2 via snapper, 2 captured"));
}

#[test]
fn human_interbuild_plan_surfaces_parser_provenance_and_risk() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_nix_flake_make_repo(tempdir.path(), "render-flake-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "render-flake-tool",
        "nix_flake",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["render-flake-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("interbuild dry-run should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("│  source:: [I]"));
    assert!(rendered.contains("interbuild parser:: nix_flake, no external CLI"));
    assert!(!rendered.contains("├─ Provenance"));
    assert!(!rendered.contains("non-native provenance actions"));
}

#[test]
fn human_failure_report_surfaces_blocker_context_and_action() {
    let report = crate::report_runtime_failure(
        &crate::CoreError::Repo(elda_repo::RepoError::SnapshotMissing),
        &CommandRequest::new(
            vec!["i".to_owned()],
            vec!["missing-tool".to_owned()],
            OutputMode::Human,
            false,
        )
        .with_offline(true),
    );

    assert_eq!(report.exit_status, ExitStatus::ResolutionFailure);
    let rendered = render_human(&report);

    assert!(rendered.contains("install blocked"));
    assert!(rendered.contains("┌─ install blocked: missing-tool"));
    assert!(rendered.contains("├─ Blocked"));
    assert!(rendered.contains("│  kind: resolution failure"));
    assert!(rendered.contains("│  command: elda i missing-tool"));
    assert!(rendered.contains("│  offline: true"));
    assert!(rendered.contains("├─ Action"));
    assert!(rendered.contains("│  run `elda sync`"));
}

#[test]
fn human_interbuild_plan_surfaces_parser_detail_block() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_gentoo_make_overlay(tempdir.path(), "render-gentoo-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "render-gentoo-tool",
        "gentoo_overlay",
        &repo_dir,
        "    package = \"app-misc/render-gentoo-tool\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["render-gentoo-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("interbuild dry-run should succeed");
    let details = report.details.as_ref().expect("details should exist");
    let interbuild = &details["plan"]["actions"][0]["interbuild"];

    assert_eq!(interbuild["parser"], "gentoo_overlay");
    assert_eq!(interbuild["external_cli_required"], false);

    let rendered = render_human(&report);
    assert!(rendered.contains("interbuild parser:: gentoo_overlay, no external CLI"));
    assert!(!rendered.contains("bounded-ebuild-metadata-parser"));
}

#[test]
fn human_interbuild_plan_surfaces_aur_parser_detail_block() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "render-aur-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "render-aur-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["render-aur-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("AUR interbuild dry-run should succeed");
    let details = report.details.as_ref().expect("details should exist");
    let interbuild = &details["plan"]["actions"][0]["interbuild"];

    assert_eq!(interbuild["parser"], "aur_pkgbuild");
    assert_eq!(interbuild["external_cli_required"], false);

    let rendered = render_human(&report);
    assert!(rendered.contains("interbuild parser:: aur_pkgbuild, no external CLI"));
    assert!(!rendered.contains("bounded-pkgbuild-metadata-parser"));
}

#[test]
fn human_interbuild_plan_surfaces_aur_vcs_context() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_vcs_make_repo(tempdir.path(), "render-aur-vcs-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "render-aur-vcs-tool",
        "aur_pkgbuild",
        &repo_dir,
        "    pkgname = \"render-aur-vcs-tool-git\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["render-aur-vcs-tool".to_owned()],
            OutputMode::Human,
            false,
        ),
    )
    .expect("AUR VCS interbuild install should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let aur = &details["installs"][0]["interbuild"]["aur"];
    assert_eq!(aur["vcs_sources"].as_array().map(Vec::len), Some(1));
    assert_eq!(aur["pkgver_function"], true);

    let rendered = render_human(&report);
    assert!(rendered.contains("┌─ installed render-aur-vcs-tool"));
    assert!(rendered.contains("│  paths:: "));
}

#[test]
fn human_interbuild_plan_surfaces_xbps_parser_detail_block() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_xbps_make_repo(tempdir.path(), "render-xbps-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "render-xbps-tool",
        "xbps_template",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["render-xbps-tool".to_owned()],
            OutputMode::Human,
            true,
        ),
    )
    .expect("XBPS interbuild dry-run should succeed");
    let details = report.details.as_ref().expect("details should exist");
    let interbuild = &details["plan"]["actions"][0]["interbuild"];

    assert_eq!(interbuild["parser"], "xbps_template");
    assert_eq!(interbuild["external_cli_required"], false);

    let rendered = render_human(&report);
    assert!(rendered.contains("interbuild parser:: xbps_template, no external CLI"));
    assert!(!rendered.contains("bounded-xbps-template-parser"));
}

#[test]
fn human_metadata_add_can_render_priority_sorted_source_options() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[metadata]\nlink_option_mode = \"list-options\"\nlink_strategy_priority = [\"make\", \"nix_flake\"]\n",
    );
    let repo_dir = create_git_nix_flake_make_repo(tempdir.path(), "render-options-tool");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["add".to_owned()],
            vec![repo_dir.display().to_string()],
            OutputMode::Human,
            false,
        ),
    )
    .expect("metadata add should succeed");

    let rendered = render_human(&report);

    assert!(rendered.contains("source options:"));
    assert!(rendered.contains("* 1. make [git, derived]"));
    assert!(rendered.contains("  2. nix_flake [nix_flake, bounded]"));
}

#[test]
fn human_state_ls_renders_scan_table_without_detail_blocks() {
    let report = CommandReport {
        area: "state",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["ls".to_owned()],
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "listed 2 installed package(s).".to_owned(),
        details: Some(json!({
            "packages": [
                {
                    "pkgname": "bfetch",
                    "version": "0:0.1.0-1",
                    "arch": "amd64",
                    "install_reason": "explicit",
                    "source_kind": "local_recipe",
                    "source_ref": "/etc/elda/recipes/bfetch",
                    "variant_id": "default",
                    "repo_commit": "4bc9d6447f96b12e554fbdaa5ffe6e1b363de9d4",
                    "state_id": "system-1777246755498",
                    "manifest_hash": "498ca50ab1b22eb01c73569d8d8538c1f5e47f45d7a72bd72803a39a3206d8aa",
                    "payload_sha256": "484c25d02516adc3d2ba5bdd53fa1aab5805ad4d8ad9e46b28535582b2fc42f6",
                    "remote_name": null,
                    "pinned_version": null,
                    "held": false,
                    "hold_source": null,
                    "package_kind": "normal"
                },
                {
                    "pkgname": "fsel",
                    "version": "0:3.3.1-1",
                    "arch": "amd64",
                    "install_reason": "dependency",
                    "source_kind": "interbuild",
                    "source_ref": "nix_flake:github:fsel/fsel",
                    "variant_id": "default",
                    "remote_name": "yoka-core",
                    "state_id": "system-1777308937160",
                    "manifest_hash": "deadbeef",
                    "payload_sha256": null,
                    "pinned_version": "3.3.1",
                    "held": true,
                    "hold_source": "operator",
                    "package_kind": "normal"
                }
            ]
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("state ok"));
    assert!(rendered.contains("listed 2 installed package(s)."));
    assert!(rendered.contains("NAME"));
    assert!(rendered.contains("bfetch"));
    assert!(rendered.contains("fsel"));
    assert!(rendered.contains("native"));
    assert!(rendered.contains("yoka-core"));
    assert!(rendered.contains("[pinned]"));

    assert!(!rendered.contains("Name:"));
    assert!(!rendered.contains("Manifest:"));
    assert!(!rendered.contains("\"packages\""));
    assert!(!rendered.contains("\"manifest_hash\""));
}

#[test]
fn human_state_list_renders_per_package_blocks_in_nix_profile_style() {
    let report = CommandReport {
        area: "state",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["list".to_owned()],
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "listed 2 installed package(s).".to_owned(),
        details: Some(json!({
            "packages": [
                {
                    "pkgname": "bfetch",
                    "version": "0:0.1.0-1",
                    "arch": "amd64",
                    "install_reason": "explicit",
                    "source_kind": "local_recipe",
                    "source_ref": "/etc/elda/recipes/bfetch",
                    "variant_id": "default",
                    "repo_commit": "4bc9d6447f96b12e554fbdaa5ffe6e1b363de9d4",
                    "state_id": "system-1777246755498",
                    "manifest_hash": "498ca50ab1b22eb01c73569d8d8538c1f5e47f45d7a72bd72803a39a3206d8aa",
                    "payload_sha256": "484c25d02516adc3d2ba5bdd53fa1aab5805ad4d8ad9e46b28535582b2fc42f6",
                    "remote_name": null,
                    "pinned_version": null,
                    "held": false,
                    "hold_source": null,
                    "package_kind": "normal"
                },
                {
                    "pkgname": "fsel",
                    "version": "0:3.3.1-1",
                    "arch": "amd64",
                    "install_reason": "dependency",
                    "source_kind": "interbuild",
                    "source_ref": "nix_flake:github:fsel/fsel",
                    "variant_id": "default",
                    "remote_name": "yoka-core",
                    "state_id": "system-1777308937160",
                    "manifest_hash": "deadbeef",
                    "payload_sha256": null,
                    "pinned_version": "3.3.1",
                    "held": true,
                    "hold_source": "operator",
                    "package_kind": "normal"
                }
            ]
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("state ok"));
    assert!(rendered.contains("listed 2 installed package(s)."));
    assert!(rendered.contains("[E] bfetch"));
    assert!(rendered.contains("  Version: 0:0.1.0-1"));
    assert!(rendered.contains("  Source ref: /etc/elda/recipes/bfetch"));
    assert!(
        rendered.contains(
            "  Manifest: 498ca50ab1b22eb01c73569d8d8538c1f5e47f45d7a72bd72803a39a3206d8aa"
        )
    );
    assert!(
        rendered.contains(
            "  Payload: 484c25d02516adc3d2ba5bdd53fa1aab5805ad4d8ad9e46b28535582b2fc42f6"
        )
    );

    assert!(rendered.contains("[I] fsel"));
    assert!(rendered.contains("  Remote: yoka-core"));
    assert!(rendered.contains("  Pinned: 3.3.1"));
    assert!(rendered.contains("  Hold: yes (operator)"));

    assert!(!rendered.contains("NAME"));
    assert!(!rendered.contains("\"packages\""));
    assert!(!rendered.contains("\"manifest_hash\""));
}

#[test]
fn human_state_ls_renders_empty_state_without_blocks() {
    let report = CommandReport {
        area: "state",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["ls".to_owned()],
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "listed 0 installed package(s).".to_owned(),
        details: Some(json!({ "packages": [] })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("state ok"));
    assert!(rendered.contains("listed 0 installed package(s)."));
    assert!(rendered.contains("No installed packages."));
    assert!(!rendered.contains("Name:"));
    assert!(!rendered.contains("\"packages\""));
}

#[test]
fn human_recipe_catalog_renders_per_recipe_blocks_with_provenance() {
    let report = CommandReport {
        area: "recipe",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["rc".to_owned(), "ls".to_owned()],
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "1 local recipe(s); 1 synced install name(s).".to_owned(),
        details: Some(json!({
            "catalog": {
                "recipes_dir": "/etc/elda/recipes",
                "local_recipes": ["bfetch"],
                "local_entries": [{
                    "pkgname": "bfetch",
                    "version": "0:0.1.0-1",
                    "source": "local_recipe",
                    "description": "binary fetch tool",
                    "upstream": "https://github.com/example/bfetch",
                    "licenses": ["MIT"]
                }],
                "synced_packages": ["yoka-core/cosmic-shell"],
                "synced_entries": [{
                    "pkgname": "cosmic-shell",
                    "version": "0:1.0.0-1",
                    "source": "synced:yoka-core",
                    "description": "cosmic desktop",
                    "upstream": "https://system76.com",
                    "licenses": ["GPL-3.0"]
                }]
            }
        })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("recipe ok"));
    assert!(rendered.contains("1 local recipe(s); 1 synced install name(s)."));
    assert!(rendered.contains("directory: /etc/elda/recipes"));

    assert!(rendered.contains("Name:           bfetch"));
    assert!(rendered.contains("Provenance:     [E] local_recipe (Native)"));
    assert!(rendered.contains("Description:    binary fetch tool"));
    assert!(rendered.contains("Upstream:       https://github.com/example/bfetch"));
    assert!(rendered.contains("Licenses:       MIT"));

    assert!(rendered.contains("Name:           cosmic-shell"));
    assert!(rendered.contains("Provenance:     [E] synced/yoka-core (Native, remote)"));
    assert!(!rendered.contains("\"catalog\""));
    assert!(!rendered.contains("\"local_recipes\""));
}

#[test]
fn human_render_does_not_emit_raw_json_for_unstyled_reports() {
    let report = CommandReport {
        area: "verify",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: vec!["verify".to_owned()],
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: "verified 0 installed package(s).".to_owned(),
        details: Some(json!({ "verified": [], "secret_field": "should-not-leak" })),
    };

    let rendered = render_human(&report);

    assert!(rendered.contains("Verify"));
    assert!(rendered.contains("verified 0 installed package(s)."));
    assert!(!rendered.contains("\"verified\""));
    assert!(!rendered.contains("secret_field"));
    assert!(!rendered.contains("should-not-leak"));
}
