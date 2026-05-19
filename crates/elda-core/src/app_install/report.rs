mod interbuild;

use serde_json::json;

use crate::app::{PlannedInstallAction, ResolvedDependencyPlan};
use crate::app_parse::installed_version;
use elda_db::InstalledPackageDetails;

use interbuild::interbuild_details;

use super::progress::{
    install_progress_for_existing, install_progress_for_plan, planned_activation_backend,
};

pub(crate) struct InstallExecutionDecision {
    pub(crate) needs_change: bool,
    pub(crate) change_kind: &'static str,
}

pub(crate) fn planned_install_action_json(
    action: &PlannedInstallAction,
    activation_backend: &str,
    link_option_mode: crate::config::LinkOptionMode,
) -> serde_json::Value {
    let decision = install_execution_decision(action);

    json!({
        "action": decision.change_kind,
        "target": action.target,
        "package": action.package_name,
        "version": format!(
            "{}:{}-{}",
            action.resolved.recipe.package.epoch,
            action.resolved.recipe.package.version,
            action.resolved.recipe.package.rel,
        ),
        "selected_lane": action.resolved.selected_lane,
        "source_kind": action.resolved.selected_source_kind,
        "selected_source_kind": action.resolved.selected_source_kind,
        "persisted_source_kind": action.resolved.persisted_source_kind,
        "source_ref": action.resolved.source_ref,
        "generated_metadata_path": action.resolved.generated_recipe_dir,
        "source_options": action.resolved.source_options,
        "selected_source_option": action.resolved.selected_source_option,
        "link_option_mode": link_option_mode,
        "remote_name": action.resolved.remote_name,
        "binary_source_verification": action.resolved.binary_source_verification,
        "variant_id": action.resolved.flag_state.variant_id,
        "install_reason": action.install_reason,
        "requested_by": action.requested_by,
        "dependency_kind": action.dependency_kind,
        "raw_expr": action.raw_expr,
        "is_weak": action.is_weak,
        "provider_group": action.provider_group,
        "replaced_packages": action.replaced_packages,
        "already_installed": action.already_installed.is_some(),
        "needs_change": decision.needs_change,
        "activation_backend": activation_backend,
        "effective_flags": action.resolved.flag_state.effective_flags,
        "progress": install_progress_for_plan(action, activation_backend),
        "dependencies": dependency_json(&action.dependencies),
        "interbuild": interbuild_details(&action.resolved),
    })
}

pub(crate) fn already_installed_json(
    action: &PlannedInstallAction,
    installed: &InstalledPackageDetails,
    installed_paths: usize,
    fallback_activation_backend: &str,
    link_option_mode: crate::config::LinkOptionMode,
) -> serde_json::Value {
    let decision = install_execution_decision(action);
    let activation_backend = installed
        .activation_backend
        .clone()
        .unwrap_or_else(|| fallback_activation_backend.to_owned());

    json!({
        "target": action.target,
        "selected_lane": action.resolved.selected_lane,
        "selected_source_kind": action.resolved.selected_source_kind,
        "persisted_source_kind": action.resolved.persisted_source_kind,
        "source_ref": action.resolved.source_ref,
        "generated_metadata_path": action.resolved.generated_recipe_dir,
        "source_options": action.resolved.source_options,
        "selected_source_option": action.resolved.selected_source_option,
        "link_option_mode": link_option_mode,
        "remote_name": action.resolved.remote_name,
        "binary_source_verification": action.resolved.binary_source_verification,
        "package": {
            "package_name": installed.pkgname,
            "epoch": installed.epoch,
            "pkgver": installed.pkgver,
            "pkgrel": installed.pkgrel,
            "variant_id": installed.variant_id,
            "source_kind": installed.source_kind,
        },
        "install": {
            "package_name": action.package_name,
            "state_id": installed.state_id,
            "installed_paths": installed_paths,
        },
        "status": "already-installed",
        "install_reason": action.install_reason,
        "requested_by": action.requested_by,
        "dependency_kind": action.dependency_kind,
        "is_weak": action.is_weak,
        "provider_group": action.provider_group,
        "replaced_packages": action.replaced_packages,
        "flag_state": {
            "variant_id": action.resolved.flag_state.variant_id,
            "effective_flags": action.resolved.flag_state.effective_flags,
        },
        "activation_backend": activation_backend,
        "progress": install_progress_for_existing(action, &activation_backend),
        "interbuild": interbuild_details(&action.resolved),
        "action": decision.change_kind,
    })
}

pub(crate) fn interbuild_details_for_report(
    resolved: &crate::app::ResolvedInstallTarget,
) -> Option<serde_json::Value> {
    interbuild_details(resolved)
}

pub(crate) fn install_execution_decision(
    action: &PlannedInstallAction,
) -> InstallExecutionDecision {
    let Some(installed) = &action.already_installed else {
        return InstallExecutionDecision {
            needs_change: true,
            change_kind: new_install_change_kind(action),
        };
    };
    let candidate_version = format!(
        "{}:{}-{}",
        action.resolved.recipe.package.epoch,
        action.resolved.recipe.package.version,
        action.resolved.recipe.package.rel,
    );
    let needs_change = installed_version(installed) != candidate_version
        || installed.variant_id != Some(action.resolved.flag_state.variant_id.clone())
        || installed.source_kind != action.resolved.persisted_source_kind;

    InstallExecutionDecision {
        needs_change,
        change_kind: if needs_change && action.install_reason == "explicit" {
            "upgrade-explicit"
        } else if needs_change {
            "upgrade-dependency"
        } else {
            "keep-installed"
        },
    }
}

pub(crate) fn fallback_activation_backend(mode: elda_db::InstallationMode) -> &'static str {
    planned_activation_backend(mode)
}

fn dependency_json(dependencies: &[ResolvedDependencyPlan]) -> Vec<serde_json::Value> {
    dependencies
        .iter()
        .map(|dependency| {
            json!({
                "target": dependency.target,
                "dependency_kind": dependency.dependency_kind,
                "raw_expr": dependency.raw_expr,
                "is_weak": dependency.is_weak,
                "provider_group": dependency.provider_group,
            })
        })
        .collect()
}

fn new_install_change_kind(action: &PlannedInstallAction) -> &'static str {
    if !action.replaced_packages.is_empty() && action.install_reason == "explicit" {
        "install-replacing"
    } else if action.is_weak {
        "install-recommended"
    } else if action.install_reason == "explicit" {
        "install-explicit"
    } else {
        "install-dependency"
    }
}
