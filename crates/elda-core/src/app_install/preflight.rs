use std::path::Path;

use rustix::fs::statvfs;
use serde_json::{Value, json};

use super::install_confirm::estimate_post_build_bytes;
use super::report::install_execution_decision;
use crate::CommandRequest;
use crate::app::{AppContext, PlannedInstallAction};
use crate::app_config_queue::pending_config_count;
use crate::app_review_memory::install_review_plan_summary;
use crate::error::CoreError;
use elda_install::pending_triggers;

pub(super) fn install_preflight_report(
    app: &AppContext,
    request: &CommandRequest,
    actions: &[PlannedInstallAction],
) -> Result<Value, CoreError> {
    let layout = app.database.layout();
    let existing_bytes = existing_managed_bytes(app, actions)?;
    let replaced_bytes = replaced_managed_bytes(app, actions)?;
    let changing_actions = actions
        .iter()
        .filter(|action| install_execution_decision(action).needs_change)
        .count();
    let review = install_review_plan_summary(&layout.data_dir, actions)?;
    let pending_config_files = pending_config_count(app)?;
    let weak_dependencies = actions.iter().filter(|action| action.is_weak).count();
    let build_lane_actions = actions
        .iter()
        .filter(|action| action.resolved.selected_lane == "source")
        .count();
    let net_managed_delta = existing_bytes.saturating_sub(replaced_bytes);
    let (estimated_new_bytes, estimate_method) = estimate_post_build_bytes(app, actions);
    let estimated_post_build_managed = net_managed_delta.saturating_add(estimated_new_bytes);
    let temporary_build_deps = temporary_build_dependency_names(actions);
    let missing_release_keys = missing_release_trust_keys(app, actions);

    Ok(json!({
        "actions": actions.len(),
        "changing_actions": changing_actions,
        "replacement_targets": actions.iter().map(|action| action.replaced_packages.len()).sum::<usize>(),
        "known_existing_managed_bytes": existing_bytes,
        "known_replaced_managed_bytes": replaced_bytes,
        "net_reinstall_managed_bytes": net_managed_delta,
        "estimated_new_payload_bytes": estimated_new_bytes,
        "estimated_post_build_managed_bytes": estimated_post_build_managed,
        "payload_size_estimate_method": estimate_method,
        "reinstall_size_delta_bytes": net_managed_delta,
        "weak_dependencies": weak_dependencies,
        "source_lane_actions": build_lane_actions,
        "temporary_build_dependencies": {
            "policy": "build-only dependencies are removed after successful source builds when the solver marks them as non-world packages",
            "planned_build_lane_actions": build_lane_actions,
            "packages": temporary_build_deps,
        },
        "candidate_size_status": if estimated_new_bytes > 0 {
            "estimated-from-cached-payloads"
        } else if build_lane_actions > 0 {
            "unknown-until-source-build"
        } else {
            "binary-or-no-change"
        },
        "root_free_bytes": free_bytes(&layout.root_dir),
        "cache_free_bytes": free_bytes(&layout.cache_pkg_dir),
        "tmp_free_bytes": free_bytes(&layout.tmp_dir),
        "review": review,
        "pending_config_files": pending_config_files,
        "conffiles": {
            "pending_queue": pending_config_files,
            "policy": "managed configuration files use .eldanew / .eldasave sidecars until resolved with config apply/keep",
        },
        "triggers": trigger_preflight(app)?,
        "snapshot_intent": snapshot_preflight(app, request),
        "source_keys": source_key_preflight(app, &missing_release_keys),
        "shared_path_policy": shared_path_policy(),
        "privilege": privilege_preflight(request),
        "policy": "dry-run preflight reports managed-byte estimates, filesystem space, review memory, configuration debt, trigger posture, and activation policy before source fetch/build",
    }))
}

fn trigger_preflight(app: &AppContext) -> Result<Value, CoreError> {
    let pending = pending_triggers(app.database.layout())?;
    Ok(json!({
        "pending_repair": pending,
        "activation_refresh": [
            "ldconfig",
            "desktop_db",
            "icon_cache",
            "font_cache",
            "depmod",
            "initramfs",
        ],
        "note": "activation resolves the concrete trigger set from installed paths; use `elda trigger run <name>` to repair one trigger",
    }))
}

fn snapshot_preflight(app: &AppContext, request: &CommandRequest) -> Value {
    let tool = app.config.defaults.snapshot_tool.trim();
    json!({
        "configured_tool": tool,
        "will_request": request.system_mode && !tool.is_empty() && tool != "none",
        "policy": "snapshot requests are recorded in activation journals when configured",
    })
}

fn source_key_preflight(app: &AppContext, missing_release_keys: &[String]) -> Value {
    json!({
        "release_keys_configured": app.config.trust.release_keys.len(),
        "missing_for_plan": missing_release_keys,
        "acquisition": if missing_release_keys.is_empty() {
            "no additional release keys required for this plan"
        } else {
            "import missing keys into config trust.release_keys or pin trusted remote keys before install"
        },
        "policy": "release payload verification uses Elda-configured trust.release_keys; missing keys fail closed only when signature verification is required",
    })
}

pub(super) fn temporary_build_dependency_names(actions: &[PlannedInstallAction]) -> Vec<String> {
    let mut names = Vec::new();
    for action in actions {
        if !install_execution_decision(action).needs_change {
            continue;
        }
        for dependency in &action.dependencies {
            if dependency.dependency_kind == "build" {
                names.push(dependency.dependency_name.clone());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

pub(super) fn missing_release_trust_keys(
    app: &AppContext,
    actions: &[PlannedInstallAction],
) -> Vec<String> {
    let configured: std::collections::BTreeSet<_> = app
        .config
        .trust
        .release_keys
        .iter()
        .map(String::as_str)
        .collect();
    let mut missing = Vec::new();
    for action in actions {
        if !install_execution_decision(action).needs_change {
            continue;
        }
        if let Some(verification) = &action.resolved.binary_source_verification
            && verification.payload_signature.is_some()
        {
            for key in &verification.trusted_public_keys {
                if !configured.contains(key.as_str()) {
                    missing.push(key.clone());
                }
            }
        }
    }
    missing.sort();
    missing.dedup();
    missing
}

pub(super) fn estimate_cached_source_tree_bytes(
    app: &AppContext,
    actions: &[PlannedInstallAction],
) -> (u64, &'static str) {
    use super::report::install_execution_decision;

    let cache_src = &app.database.layout().cache_src_dir;
    let mut total = 0_u64;
    let mut hits = 0_usize;
    let mut candidates = 0_usize;

    for action in actions {
        if !install_execution_decision(action).needs_change {
            continue;
        }
        if action.resolved.selected_lane != "source" {
            continue;
        }
        candidates += 1;
        let package_name = &action.package_name;
        let direct = cache_src.join(package_name);
        let mut package_bytes = 0_u64;
        if direct.is_dir() {
            package_bytes = package_bytes.saturating_add(directory_size(&direct));
        }
        let remote_root = cache_src.join("remote-recipes");
        if remote_root.is_dir() {
            package_bytes =
                package_bytes.saturating_add(sum_named_subtrees(&remote_root, package_name));
        }
        if package_bytes > 0 {
            total = total.saturating_add(package_bytes);
            hits += 1;
        }
    }

    if hits == 0 {
        return (0, "no-cached-source-tree-yet");
    }
    if hits < candidates {
        return (total, "partial-cached-source-tree-sizes");
    }
    (total, "cached-source-tree-sizes")
}

fn directory_size(path: &std::path::Path) -> u64 {
    let mut total = 0_u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            total = total.saturating_add(directory_size(&entry_path));
        } else if let Ok(metadata) = entry.metadata() {
            total = total.saturating_add(metadata.len());
        }
    }
    total
}

fn sum_named_subtrees(root: &std::path::Path, package_name: &str) -> u64 {
    let mut total = 0_u64;
    let Ok(remotes) = std::fs::read_dir(root) else {
        return 0;
    };
    for remote in remotes.flatten() {
        let Ok(commits) = std::fs::read_dir(remote.path()) else {
            continue;
        };
        for commit in commits.flatten() {
            let package_dir = commit.path().join(package_name);
            if package_dir.is_dir() {
                total = total.saturating_add(directory_size(&package_dir));
            }
        }
    }
    total
}

pub(super) fn estimate_planned_payload_bytes(
    app: &AppContext,
    actions: &[PlannedInstallAction],
) -> (u64, &'static str) {
    let cache_dir = &app.database.layout().cache_pkg_dir;
    let mut total = 0_u64;
    let mut hits = 0_usize;
    let mut candidates = 0_usize;

    for action in actions {
        if !install_execution_decision(action).needs_change {
            continue;
        }
        candidates += 1;
        let arch = action
            .resolved
            .recipe
            .package
            .arch
            .first()
            .map(String::as_str)
            .unwrap_or("any");
        let payload = cache_dir.join(format!(
            "{}-{}-{}-{}.pkg.tar.zst",
            action.package_name,
            action.resolved.recipe.package.version,
            action.resolved.recipe.package.rel,
            arch
        ));
        if let Ok(metadata) = std::fs::metadata(&payload) {
            total = total.saturating_add(metadata.len());
            hits += 1;
        }
    }

    if hits == 0 {
        return (0, "no-cached-payload-yet");
    }
    if hits < candidates {
        return (total, "partial-cached-payload-sizes");
    }
    (total, "cached-payload-sizes")
}

fn shared_path_policy() -> Value {
    json!({
        "terminfo": "reuse unmanaged /usr/share/terminfo and /usr/lib/terminfo entries without claiming ownership",
        "trigger_backed_caches": "activation may refresh shared caches through native triggers without false ownership",
        "collisions": "app-owned executable/library/config collisions still fail closed",
    })
}

fn privilege_preflight(request: &CommandRequest) -> Value {
    json!({
        "system_mode_requested": request.system_mode,
        "escalation": if request.system_mode {
            "live /usr activation may require configured privilege provider before mutation"
        } else {
            "prefix activation does not require system privilege escalation"
        },
    })
}

fn existing_managed_bytes(
    app: &AppContext,
    actions: &[PlannedInstallAction],
) -> Result<u64, CoreError> {
    actions.iter().try_fold(0_u64, |total, action| {
        if action.already_installed.is_some() {
            Ok(total.saturating_add(package_managed_bytes(app, &action.package_name)?))
        } else {
            Ok(total)
        }
    })
}

fn replaced_managed_bytes(
    app: &AppContext,
    actions: &[PlannedInstallAction],
) -> Result<u64, CoreError> {
    actions.iter().try_fold(0_u64, |total, action| {
        action
            .replaced_packages
            .iter()
            .try_fold(total, |sum, package| {
                Ok(sum.saturating_add(package_managed_bytes(app, package)?))
            })
    })
}

fn package_managed_bytes(app: &AppContext, package: &str) -> Result<u64, CoreError> {
    Ok(app
        .database
        .package_files(package)?
        .into_iter()
        .map(|entry| entry.size)
        .sum())
}

fn free_bytes(path: &Path) -> Option<u64> {
    let stats = statvfs(path).ok()?;
    Some(stats.f_bavail.saturating_mul(stats.f_frsize.max(1)))
}
