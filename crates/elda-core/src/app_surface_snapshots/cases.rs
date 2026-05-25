use elda_types::ExitStatus;
use serde_json::{Value, json};

use crate::{CommandReport, OutputMode};

fn report(
    area: &'static str,
    status: &'static str,
    path: &[&str],
    details: Value,
) -> CommandReport {
    report_with_options(area, status, path, details, false)
}

fn report_with_options(
    area: &'static str,
    status: &'static str,
    path: &[&str],
    details: Value,
    dry_run: bool,
) -> CommandReport {
    CommandReport {
        area,
        status,
        exit_status: ExitStatus::Success,
        command_path: path.iter().map(|part| (*part).to_owned()).collect(),
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run,
        summary: format!("surface snapshot for {area}"),
        details: Some(details),
    }
}

fn report_with_operands(
    area: &'static str,
    status: &'static str,
    path: &[&str],
    operands: &[&str],
    details: Value,
) -> CommandReport {
    CommandReport {
        area,
        status,
        exit_status: ExitStatus::Success,
        command_path: path.iter().map(|part| (*part).to_owned()).collect(),
        operands: operands.iter().map(|part| (*part).to_owned()).collect(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: format!("surface snapshot for {area}"),
        details: Some(details),
    }
}

pub(super) fn snapshot_cases() -> Vec<(&'static str, CommandReport)> {
    vec![
        (
            "search",
            report(
                "search",
                "ok",
                &["search"],
                json!({
                    "query": "fsel",
                    "results": [
                        {"remote_name": "aur", "pkgname": "fsel", "epoch": 0, "pkgver": "3.4.1", "pkgrel": 1, "source_kind": "interbuild", "description": "Fast TUI app launcher and fuzzy finder"},
                        {"remote_name": "core", "pkgname": "fselect", "epoch": 0, "pkgver": "0.10.0", "pkgrel": 1, "source_kind": "repo_binary", "summary": "Find files with SQL-like queries"}
                    ]
                }),
            ),
        ),
        (
            "files",
            report(
                "state",
                "ok",
                &["files"],
                json!({
                    "package": "fsel",
                    "files": [
                        {"path": "/usr/bin/fsel", "path_kind": "file"},
                        {"path": "/usr/share/doc/fsel/README.md", "path_kind": "file"},
                        {"path": "/usr/share/applications/fsel.desktop", "path_kind": "file"}
                    ]
                }),
            ),
        ),
        (
            "files-search",
            report(
                "state",
                "ok",
                &["files", "search"],
                json!({
                    "query": "desktop",
                    "matches": [
                        {"pkgname": "fsel", "path": "/usr/share/applications/fsel.desktop", "path_kind": "file"},
                        {"pkgname": "cosmic-shell", "path": "/usr/share/wayland-sessions/cosmic.desktop", "path_kind": "file"}
                    ]
                }),
            ),
        ),
        (
            "files-owner",
            report(
                "state",
                "ok",
                &["files", "owner"],
                json!({
                    "path": "/usr/bin/fsel",
                    "owners": [{"pkgname": "fsel", "version": "0:3.4.1-1"}]
                }),
            ),
        ),
        (
            "remove-result",
            report(
                "remove",
                "ok",
                &["rm"],
                json!({
                    "removals": [
                        {"package_name": "fsel", "removed_paths": 7},
                        {"package_name": "unused-lib", "removed_paths": 3}
                    ]
                }),
            ),
        ),
        (
            "upgrade-result",
            report(
                "upgrade",
                "ok",
                &["u"],
                json!({
                    "actions": [
                        {"action": "upgrade", "target": "fsel", "candidate_version": "0:3.4.2-1"},
                        {"action": "keep-installed", "target": "glibc", "candidate_version": ""}
                    ],
                    "upgrades": [
                        {"package_name": "fsel", "installed_paths": 7, "status": "installed"}
                    ]
                }),
            ),
        ),
        (
            "git-releases",
            report(
                "git",
                "ok",
                &["git", "releases"],
                json!({
                    "git_releases": {
                        "repo": "Mjoyufull/fsel",
                        "releases": [{
                            "tag": "v3.4.1",
                            "normalized_version": "0:3.4.1-1",
                            "version_confidence": "stable-semver",
                            "recommended_asset": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                            "assets": [{"name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz", "kind": "payload", "format": "tar-gz", "compatibility": "native-exact", "score": 31}]
                        }]
                    }
                }),
            ),
        ),
        (
            "appimage-inspect",
            report(
                "appimage",
                "ok",
                &["appimage", "inspect"],
                json!({
                    "appimage_inspect": {
                        "path": "/tmp/Foo.AppImage",
                        "generation": 2,
                        "squashfs_offset": 4096,
                        "primary_desktop_path": "foo.desktop",
                        "desktop_name": "Foo",
                        "desktop_exec_original": "Foo %U",
                        "desktop_icon_raw": "foo",
                        "apprun_path": "AppRun",
                        "desktop_candidates": ["foo.desktop"],
                        "icon_candidates": ["usr/share/icons/hicolor/256x256/apps/foo.png"],
                        "metainfo_candidates": ["usr/share/metainfo/foo.appdata.xml"],
                        "fuse_note": "FUSE is not required for read-only inspection."
                    }
                }),
            ),
        ),
        (
            "qa-lint",
            report(
                "qa",
                "ok",
                &["qa", "lint"],
                json!({
                    "lint": {
                        "recipes": ["fsel", "cosmic-shell"],
                        "issues": [{"recipe": "fsel", "message": "missing maintainer"}]
                    }
                }),
            ),
        ),
        (
            "forge-search",
            report(
                "forge",
                "ok",
                &["forge", "search"],
                json!({
                    "query": "fsel",
                    "results": [
                        {"pkgname": "fsel", "source": "local-index", "packages_repo_path": "packages/fsel"}
                    ]
                }),
            ),
        ),
        (
            "vendor-recipe",
            report(
                "vendor",
                "ok",
                &["vendor", "add"],
                json!({
                    "vendor": {
                        "package_name": "fsel-bin",
                        "recipe_dir": "/etc/elda/recipes/fsel-bin",
                        "source_kind": "github_release",
                        "source_url": "https://github.com/Mjoyufull/fsel/releases",
                        "asset": "fsel-linux-x86_64.tar.gz",
                        "binary": "fsel",
                        "sha256": "abc123"
                    }
                }),
            ),
        ),
        (
            "sync-result",
            report(
                "sync",
                "ok",
                &["sync"],
                json!({
                    "sync": {
                        "snapshot_path": "/var/lib/elda/snapshots/core.json",
                        "offline": false,
                        "remote_count": 2,
                        "package_count": 128,
                        "verified_remote_count": 1,
                        "failed_remote_count": 1,
                        "remotes": [
                            {"name": "core", "package_count": 120, "verified": true, "stale": false},
                            {"name": "aur-lite", "package_count": 8, "verified": false, "stale": true, "issue": "signature missing"}
                        ],
                        "package_deltas": [
                            {"remote_name": "core", "added_count": 3, "removed_count": 1, "kept_count": 117, "previous_count": 118, "current_count": 120, "added_packages": ["fsel"], "removed_packages": ["old-fsel"]}
                        ]
                    }
                }),
            ),
        ),
        (
            "remote-list",
            report(
                "remote",
                "ok",
                &["rmt", "ls"],
                json!({
                    "remotes": [
                        {"name": "core", "index_url": "https://example.invalid/core.idx", "channel": "stable", "priority": 10, "enabled": true, "trust": "pinned"},
                        {"name": "aur-lite", "index_url": "https://example.invalid/aur", "channel": "stable-7d", "priority": 50, "enabled": true, "trust": "tofu"}
                    ]
                }),
            ),
        ),
        (
            "profile-plan",
            report(
                "profile",
                "planned",
                &["pf", "apply"],
                json!({
                    "plan": {
                        "kind": "apply",
                        "previous_active_profiles": ["base"],
                        "next_active_profiles": ["base", "desktop"],
                        "install_actions": [{"package": "desktop"}]
                    }
                }),
            ),
        ),
        (
            "config-pending",
            report(
                "config",
                "ok",
                &["config", "pending"],
                json!({
                    "config": {
                        "pending": [
                            {"package": "fsel", "path": "/etc/fsel/config.toml", "state": "eldanew"}
                        ]
                    }
                }),
            ),
        ),
        (
            "trigger-list",
            report(
                "trigger",
                "ok",
                &["trigger", "ls"],
                json!({
                    "triggers": {
                        "backend": "linux-copy",
                        "system_mode": true,
                        "pending": [{"name": "ldconfig", "reason": "shared-library change", "output_path": "/var/lib/elda/triggers/ldconfig.log"}],
                        "last_run": [{"name": "desktop-database", "reason": "success"}],
                        "boot_status": {"managed_inputs": ["/usr/lib"], "pending_triggers": ["ldconfig"]}
                    }
                }),
            ),
        ),
        (
            "policy-pin",
            report(
                "policy",
                "ok",
                &["pin"],
                json!({
                    "package": "fsel",
                    "version": "0:3.4.1-1",
                    "pinned_version": "0:3.4.1-1",
                    "held": false
                }),
            ),
        ),
        (
            "daemon-status",
            report(
                "daemon",
                "ok",
                &["daemon", "status"],
                json!({
                    "snapshot_present": true,
                    "snapshot_path": "/var/lib/elda/daemon/snapshot.json",
                    "snapshot": {"packages": 128}
                }),
            ),
        ),
        (
            "config-diff",
            report(
                "config",
                "ok",
                &["config", "diff"],
                json!({
                    "config_diff": {
                        "package": "fsel",
                        "path": "/etc/fsel/config.toml",
                        "live_path": "/etc/fsel/config.toml",
                        "sidecar_path": "/etc/fsel/config.toml.eldanew",
                        "sidecar_kind": "eldanew",
                        "changed": true,
                        "diff": ["- old = true", "+ old = false"]
                    }
                }),
            ),
        ),
        (
            "trigger-info",
            report(
                "trigger",
                "ok",
                &["trigger", "info"],
                json!({
                    "trigger": {
                        "name": "ldconfig",
                        "backend": "linux-copy",
                        "known": true,
                        "pending": {"name": "ldconfig", "reason": "shared-library change"},
                        "last_run": {"name": "ldconfig", "reason": "success"},
                        "boot_path": true,
                        "critical": true,
                        "output_path": "/var/lib/elda/triggers/ldconfig.log",
                        "output": "ok"
                    }
                }),
            ),
        ),
        (
            "migration-adoption",
            report(
                "migration",
                "ok",
                &["adopt"],
                json!({
                    "adoption": {
                        "pkgname": "fsel",
                        "source_pm": "pacman",
                        "version": {"epoch": 0, "pkgver": "3.4.1", "pkgrel": 1, "raw": "3.4.1-1"},
                        "arch": "x86_64",
                        "files": ["/usr/bin/fsel", "/usr/share/doc/fsel/README.md"],
                        "dependencies": ["glibc"]
                    }
                }),
            ),
        ),
        (
            "verify",
            report(
                "verify",
                "ok",
                &["verify"],
                json!({
                    "verify_report": {
                        "packages": ["fsel"],
                        "checked_paths": 42,
                        "issues": [{"package": "fsel", "path": "/usr/bin/fsel", "kind": "missing", "detail": "not on disk"}]
                    }
                }),
            ),
        ),
        (
            "check",
            report(
                "check",
                "ok",
                &["check"],
                json!({
                    "health": {"issues": ["stale remote aur-lite", "pending trigger ldconfig"]},
                    "pending_triggers": [{"name": "ldconfig"}],
                    "backend": {"kind": "linux-copy", "mode": "system"}
                }),
            ),
        ),
        (
            "doctor",
            report(
                "doctor",
                "ok",
                &["doctor"],
                json!({
                    "mode": "system",
                    "root": "/",
                    "prefix": "/usr",
                    "counts": {"installed_packages": 128, "remotes": 2},
                    "issues": [{"severity": "warn", "message": "remote aur-lite signature missing"}]
                }),
            ),
        ),
        (
            "state-ls",
            report(
                "state",
                "ok",
                &["ls"],
                json!({
                    "packages": [
                        {"pkgname": "fsel", "version": "0:3.4.1-1", "install_reason": "explicit", "source_kind": "interbuild", "held": false},
                        {"pkgname": "foot", "version": "0:1.19.0-1", "install_reason": "dependency", "source_kind": "local_recipe", "held": true, "hold_source": "operator", "pinned_version": "0:1.19.0-1"}
                    ]
                }),
            ),
        ),
        (
            "cache-list",
            report(
                "cache",
                "ok",
                &["cache", "ls"],
                json!({
                    "caches": [
                        {"name": "lan", "base_url": "https://cache.example.invalid", "priority": 10, "enabled": true}
                    ]
                }),
            ),
        ),
        (
            "hold-policy",
            report(
                "policy",
                "ok",
                &["hold"],
                json!({
                    "package": "fsel",
                    "version": "0:3.4.1-1",
                    "held": true,
                    "hold_source": "operator"
                }),
            ),
        ),
        (
            "recovery",
            report(
                "recovery",
                "ok",
                &["recovery"],
                json!({
                    "recovery": {
                        "recovered": [
                            {"journal_id": "j-1", "package_name": "fsel", "action": "install", "status": "recovered"}
                        ]
                    }
                }),
            ),
        ),
        (
            "info",
            report(
                "info",
                "ok",
                &["info"],
                json!({
                    "package": "fsel",
                    "recipe": {
                        "source": "local",
                        "package": {
                            "name": "fsel",
                            "epoch": 0,
                            "version": "3.4.1",
                            "rel": 1,
                            "arch": ["x86_64"],
                            "kind": "normal",
                            "source": {"kind": "git", "fields": {"url": {"string": "https://example.invalid/fsel.git"}}},
                            "depends": [],
                            "makedepends": [],
                            "checkdepends": [],
                            "recommends": [],
                            "suggests": [],
                            "supplements": [],
                            "enhances": [],
                            "provides": [],
                            "conflicts": [],
                            "replaces": [],
                            "conffiles": []
                        }
                    }
                }),
            ),
        ),
        (
            "flags",
            report(
                "flags",
                "ok",
                &["flags"],
                json!({
                    "package": "fsel",
                    "selected_lane": "source",
                    "source_kind": "git",
                    "flag_state": {
                        "variant_id": "default",
                        "customized": false,
                        "default_flags": {"docs": true},
                        "effective_flags": {"docs": false}
                    }
                }),
            ),
        ),
        (
            "recipe-show",
            report(
                "recipe",
                "ok",
                &["rc", "show"],
                json!({
                    "show": {
                        "package": "fsel",
                        "selected_source": "local",
                        "local": {
                            "recipe": {
                                "path": "/etc/elda/recipes/fsel/pkg.lua",
                                "package": {
                                    "name": "fsel",
                                    "epoch": 0,
                                    "version": "3.4.1",
                                    "rel": 1,
                                    "kind": "normal",
                                    "depends": []
                                }
                            }
                        }
                    }
                }),
            ),
        ),
        (
            "recipe-check",
            report(
                "recipe",
                "ok",
                &["rc", "check"],
                json!({
                    "check": {
                        "recipes": ["fsel"],
                        "issues": [{"recipe": "fsel", "severity": "warning", "message": "missing maintainer"}]
                    }
                }),
            ),
        ),
        (
            "recipe-diff",
            report(
                "recipe",
                "ok",
                &["rc", "diff"],
                json!({
                    "diff": {
                        "package": "fsel",
                        "local": {"present": true},
                        "synced": {"present": true, "version": "0:3.4.2-1"},
                        "changes": [{"field": "version", "local": "0:3.4.1-1", "synced": "0:3.4.2-1", "changed": true}]
                    }
                }),
            ),
        ),
        (
            "publish-ready",
            report(
                "recipe",
                "ok",
                &["rc", "publish-ready"],
                json!({
                    "publish_ready": {
                        "package": "fsel",
                        "ready": false,
                        "blockers": ["missing signature"],
                        "warnings": ["upstream URL is example.invalid"]
                    }
                }),
            ),
        ),
        (
            "ci-submission",
            report(
                "ci",
                "ok",
                &["ci", "sub"],
                json!({
                    "submission": {
                        "id": "123-fsel",
                        "state": "published",
                        "mode": "direct",
                        "branch_name": "elda/fsel",
                        "remote_name": "origin",
                        "packages_repo_path": "/tmp/ci/pkgs"
                    }
                }),
            ),
        ),
        (
            "ci-batch",
            report(
                "ci",
                "ok",
                &["ci", "batch"],
                json!({
                    "batch": {
                        "name": "nightly",
                        "state": "running",
                        "packages": ["fsel", "foot"],
                        "last_submission_id": "123-fsel"
                    }
                }),
            ),
        ),
        (
            "ci-status",
            report(
                "ci",
                "ok",
                &["ci", "status"],
                json!({
                    "pending_count": 1,
                    "state_counts": {"queued": 1, "published": 2},
                    "submissions": [{
                        "id": "123-fsel",
                        "state": "queued",
                        "mode": "direct",
                        "branch_name": "elda/fsel",
                        "attempts": 0,
                        "completed_layers": 0,
                        "planned_layers": 2
                    }]
                }),
            ),
        ),
        (
            "ci-logs",
            report(
                "ci",
                "ok",
                &["ci", "logs"],
                json!({
                    "submission_id": "123-fsel",
                    "state": "published",
                    "attempts": 2,
                    "log_path": "/tmp/ci-fsel.log",
                    "content": "scheduler_start attempt=2\nscheduler_complete attempt=2"
                }),
            ),
        ),
        (
            "upgrade-plan",
            report(
                "plan",
                "planned",
                &["u"],
                json!({
                    "plan": {
                        "kind": "upgrade",
                        "actions": [{"action": "upgrade", "target": "fsel", "candidate_version": "0:3.4.2-1"}]
                    }
                }),
            ),
        ),
        (
            "remove-plan",
            report(
                "plan",
                "planned",
                &["rm"],
                json!({
                    "plan": {
                        "kind": "remove",
                        "actions": [{"action": "remove", "target": "fsel"}]
                    }
                }),
            ),
        ),
        (
            "failure-blocked",
            CommandReport {
                area: "install",
                status: "blocked",
                exit_status: ExitStatus::ResolutionFailure,
                command_path: vec!["i".to_owned()],
                operands: vec!["missing-tool".to_owned()],
                output_mode: OutputMode::Human,
                dry_run: false,
                summary: "install blocked: repository snapshot missing".to_owned(),
                details: Some(json!({
                    "blocked": "repository snapshot missing",
                    "kind": "resolution failure",
                    "command_path": ["i"],
                    "operands": ["missing-tool"],
                    "next_action": "run `elda sync`",
                    "offline": true
                })),
            },
        ),
        (
            "extension",
            report(
                "extension",
                "ok",
                &["ext", "ls"],
                json!({
                    "total": 2,
                    "enabled": 1,
                    "extensions": [{"name": "forge-helper", "kind": "lua", "version": "0.1.0", "enabled": true}]
                }),
            ),
        ),
        (
            "host-scan",
            report(
                "host",
                "ok",
                &["host", "scan"],
                json!({
                    "scan": {
                        "packages": [
                            {"package": "fsel", "status": "ready", "blockers": []},
                            {"package": "foot", "status": "blocked", "blockers": ["missing recipe"]}
                        ]
                    }
                }),
            ),
        ),
        (
            "state-list",
            report_with_operands(
                "state",
                "ok",
                &["list"],
                &["fsel"],
                json!({
                    "packages": [{
                        "pkgname": "fsel",
                        "version": "0:3.4.1-1",
                        "arch": "x86_64",
                        "install_reason": "explicit",
                        "source_kind": "interbuild",
                        "remote_name": "core",
                        "held": false,
                        "package_kind": "normal"
                    }]
                }),
            ),
        ),
        (
            "maint",
            report(
                "maint",
                "ok",
                &["maint", "status"],
                json!({
                    "modules": [{"name": "sync", "status": "ok"}],
                    "actions": [{"module": "cache", "status": "stale"}]
                }),
            ),
        ),
        (
            "init",
            report(
                "init",
                "ok",
                &["init"],
                json!({
                    "config_created": "created",
                    "config_path": "/etc/elda/config.toml",
                    "created": [{"label": "state", "path": "/var/lib/elda"}]
                }),
            ),
        ),
        (
            "review",
            report(
                "review",
                "changed",
                &["review", "diff"],
                json!({
                    "package": "fsel",
                    "recipe_path": "/etc/elda/recipes/fsel",
                    "stamps": [{"package": "fsel", "review_kind": "changed", "recipe_path": "/etc/elda/recipes/fsel/pkg.lua"}]
                }),
            ),
        ),
        (
            "install-plan",
            report_with_options(
                "plan",
                "planned",
                &["i"],
                json!({
                    "plan": {
                        "kind": "install",
                        "actions": [{
                            "target": "fsel",
                            "package": "fsel",
                            "version": "0:3.4.1-1",
                            "install_reason": "explicit",
                            "selected_lane": "binary",
                            "selected_source_kind": "repo_binary",
                            "persisted_source_kind": "repo_binary",
                            "activation_backend": "prefix-copy"
                        }]
                    },
                    "layout": {"prefix": "/usr", "mode": "prefix-copy"},
                    "preflight": {"space_bytes": 65536}
                }),
                true,
            ),
        ),
        (
            "downgrade-plan",
            report(
                "plan",
                "planned",
                &["downgrade"],
                json!({
                    "plan": {
                        "kind": "downgrade",
                        "package": "fsel",
                        "installed_version": "0:3.4.2-1",
                        "candidate": {"epoch": 0, "pkgver": "3.4.1", "pkgrel": 1}
                    }
                }),
            ),
        ),
        (
            "rollback-plan",
            report(
                "plan",
                "planned",
                &["rollback"],
                json!({
                    "plan": {
                        "kind": "rollback",
                        "to_state": "state-abc",
                        "from_state": "state-def",
                        "removed_packages": ["foot"],
                        "restored_packages": ["fsel"]
                    }
                }),
            ),
        ),
        (
            "state-import-plan",
            report(
                "plan",
                "planned",
                &["state", "import"],
                json!({
                    "plan": {
                        "kind": "state-import",
                        "remotes": [{"name": "core"}],
                        "world": ["fsel", "foot"]
                    }
                }),
            ),
        ),
        (
            "state-export",
            report(
                "state",
                "ok",
                &["state", "export"],
                json!({
                    "exported": {
                        "format_version": 1,
                        "installation_mode": "system",
                        "prefix": "/usr",
                        "remotes": [{"name": "core"}],
                        "world": ["fsel"],
                        "installed": [{"pkgname": "fsel"}]
                    }
                }),
            ),
        ),
        (
            "state-import",
            report(
                "state",
                "ok",
                &["state", "import"],
                json!({
                    "imported": {
                        "remotes": [{"name": "core"}],
                        "world": ["fsel"],
                        "profile": {"active_profiles": ["base"]}
                    }
                }),
            ),
        ),
        (
            "remote-trust",
            report(
                "remote",
                "ok",
                &["rmt", "trust"],
                json!({
                    "remote": {"name": "core", "index_url": "https://example.invalid/core.idx", "channel": "stable"},
                    "trust_command": true,
                    "trust_report": {
                        "trust": "pinned",
                        "rotation_policy": "manual",
                        "payload_verification": "required",
                        "configured_trusted_keys": ["DEADBEEF"],
                        "snapshot_present": true,
                        "snapshot_verified": true,
                        "selected_key": "DEADBEEF"
                    }
                }),
            ),
        ),
        (
            "remote-preview",
            report(
                "remote",
                "ok",
                &["rmt", "preview"],
                json!({
                    "remote": {"name": "aur-lite", "index_url": "https://example.invalid/aur"},
                    "preview": {
                        "discovered_count": 12,
                        "included_count": 10,
                        "excluded_count": 2,
                        "parseable_count": 10,
                        "kind": "interemote",
                        "parser": "lua-index",
                        "source_kind": "git",
                        "packages": [{"name": "fsel", "version": "0:3.4.1-1", "package_path": "packages/fsel"}]
                    }
                }),
            ),
        ),
        (
            "remote-detail",
            report(
                "remote",
                "ok",
                &["rmt", "info"],
                json!({
                    "info": true,
                    "remote": {"name": "core", "index_url": "https://example.invalid/core.idx", "channel": "stable", "priority": 10},
                    "snapshot": {"present": true, "verified": true, "package_count": 128, "stale": false},
                    "indexed_packages": ["fsel", "foot"],
                    "installed_packages": [{"pkgname": "fsel", "version": "0:3.4.1-1", "install_reason": "explicit"}]
                }),
            ),
        ),
        (
            "git-tags",
            report(
                "git",
                "ok",
                &["git", "tags"],
                json!({
                    "git_tags": {
                        "target": "fsel",
                        "tags": [{"tag": "v3.4.1", "object": "abc123", "normalized_version": "0:3.4.1-1", "version_confidence": "stable-semver"}]
                    }
                }),
            ),
        ),
        (
            "metadata-add",
            report(
                "metadata",
                "ok",
                &["metadata", "add"],
                json!({
                    "metadata_add": {
                        "link_option_mode": "priority",
                        "targets": [{
                            "target": "fsel",
                            "recipe_name": "fsel",
                            "recipe_dir": "/etc/elda/recipes/fsel",
                            "pkg_lua": "/etc/elda/recipes/fsel/pkg.lua",
                            "selected_lane": "binary",
                            "selected_source_kind": "github_release",
                            "publish_ready": false,
                            "fields": [{"field": "upstream", "confidence": "high"}]
                        }]
                    }
                }),
            ),
        ),
        (
            "recipe-catalog",
            report(
                "recipe",
                "ok",
                &["rc", "ls"],
                json!({
                    "catalog": {
                        "recipes_dir": "/etc/elda/recipes",
                        "local_recipes": ["fsel"],
                        "local_entries": [{
                            "pkgname": "fsel",
                            "version": "0:3.4.1-1",
                            "source": "local_recipe",
                            "description": "Fast launcher",
                            "upstream": "https://example.invalid/fsel",
                            "licenses": ["MIT"]
                        }],
                        "synced_packages": [],
                        "synced_entries": []
                    }
                }),
            ),
        ),
        (
            "publish",
            report(
                "publish",
                "ok",
                &["publish", "plan"],
                json!({
                    "channel": "stable",
                    "requested_targets": ["fsel"],
                    "packages": [{"package": "fsel", "layer": "packages", "recipe_path": "/etc/elda/recipes/fsel"}]
                }),
            ),
        ),
        ("version", report("version", "ok", &["version"], json!({}))),
    ]
}

pub(super) const SNAPSHOT_REQUIRED: &[&str] = &[
    "search",
    "files",
    "files-search",
    "files-owner",
    "remove-result",
    "upgrade-result",
    "git-releases",
    "appimage-inspect",
    "qa-lint",
    "forge-search",
    "vendor-recipe",
    "sync-result",
    "remote-list",
    "profile-plan",
    "config-pending",
    "trigger-list",
    "policy-pin",
    "daemon-status",
    "config-diff",
    "trigger-info",
    "migration-adoption",
    "verify",
    "check",
    "doctor",
    "state-ls",
    "cache-list",
    "hold-policy",
    "recovery",
    "info",
    "flags",
    "recipe-show",
    "recipe-check",
    "recipe-diff",
    "publish-ready",
    "ci-submission",
    "ci-batch",
    "ci-status",
    "ci-logs",
    "upgrade-plan",
    "remove-plan",
    "failure-blocked",
    "extension",
    "host-scan",
    "state-list",
    "maint",
    "init",
    "review",
    "install-plan",
    "downgrade-plan",
    "rollback-plan",
    "state-import-plan",
    "state-export",
    "state-import",
    "remote-trust",
    "remote-preview",
    "remote-detail",
    "git-tags",
    "metadata-add",
    "recipe-catalog",
    "publish",
    "version",
];
