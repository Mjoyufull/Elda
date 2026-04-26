use std::collections::BTreeSet;

use serde_json::json;

use crate::app::{AppContext, PlannedInstallAction, ResolvedProfileState};
use crate::error::CoreError;
use elda_install::remove_package;

use super::policy::profile_policy_json;
use super::system_changes::ProfileRuntimeView;
use super::{dedupe_preserve_order, empty_to};

pub(super) fn validate_profile_apply_plan(
    install_plan: &[PlannedInstallAction],
) -> Result<(), CoreError> {
    let implicit_profile_anchors = install_plan
        .iter()
        .filter(|action| action.install_reason != "explicit")
        .filter(|action| action.resolved.recipe.package.kind == "profile")
        .map(|action| action.package_name.clone())
        .collect::<Vec<_>>();

    if !implicit_profile_anchors.is_empty() {
        return Err(CoreError::Operator(format!(
            "profile apply requires explicit profile anchors; include `{}` directly in the command",
            implicit_profile_anchors.join("`, `"),
        )));
    }

    for action in install_plan
        .iter()
        .filter(|action| action.install_reason == "explicit")
    {
        if action.resolved.recipe.package.kind != "profile" {
            return Err(CoreError::Operator(format!(
                "`pf` target `{}` is not a `package_kind = profile` recipe",
                action.package_name,
            )));
        }
    }

    Ok(())
}

pub(super) fn profile_anchors_to_remove(
    app: &AppContext,
    previous_active_profiles: &[String],
    next_active_profiles: &[String],
    install_plan: &[PlannedInstallAction],
) -> Result<Vec<String>, CoreError> {
    let next_active = next_active_profiles
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let planned_packages = install_plan
        .iter()
        .map(|action| action.package_name.clone())
        .collect::<BTreeSet<_>>();
    let mut removed = Vec::new();

    for anchor in dedupe_preserve_order(previous_active_profiles.to_vec()) {
        if next_active.contains(&anchor) {
            continue;
        }
        if planned_packages.contains(&anchor) {
            return Err(CoreError::Operator(format!(
                "profile anchor `{anchor}` is still required by the requested profile set; include it explicitly in the command",
            )));
        }
        let reverse_dependencies = app.database.reverse_dependencies(&anchor, false)?;
        if !reverse_dependencies.is_empty() {
            let dependents = reverse_dependencies
                .into_iter()
                .map(|dependency| dependency.pkgname)
                .collect::<Vec<_>>()
                .join("`, `");
            return Err(CoreError::Operator(format!(
                "cannot deactivate profile anchor `{anchor}` because it is still required by `{dependents}`",
            )));
        }
        removed.push(anchor);
    }

    Ok(removed)
}

pub(super) fn remove_profile_anchors(
    app: &AppContext,
    removed_profile_anchors: &[String],
) -> Result<Vec<serde_json::Value>, CoreError> {
    let mut removed = Vec::new();
    let mutation_policy = app.mutation_policy();

    for anchor in removed_profile_anchors {
        if app.database.installed_package(anchor)?.is_some() {
            removed.push(json!(remove_package(
                &app.database,
                anchor,
                &mutation_policy
            )?));
        }
    }

    Ok(removed)
}

pub(super) fn profile_selection_plan_json(
    plan_kind: &str,
    current: &ResolvedProfileState,
    desired: &crate::app::DesiredStateProfile,
    declared_policy: &super::policy::ProfilePolicyResolution,
    install_plan: &[PlannedInstallAction],
    removed_profile_anchors: &[String],
    provider_reconciliation: &serde_json::Value,
    runtime_view: &ProfileRuntimeView,
) -> serde_json::Value {
    json!({
        "kind": plan_kind,
        "previous_active_profiles": current.active_profiles,
        "next_active_profiles": desired.active_profiles,
        "previous_native_arch": current.native_arch,
        "next_native_arch": desired.native_arch,
        "previous_init": empty_to(current.init.clone(), "unset".to_owned()),
        "next_init": empty_to(desired.init.clone(), "unset".to_owned()),
        "previous_foreign_arches": current.foreign_arches,
        "next_foreign_arches": desired.foreign_arches,
        "declared_profile_policy": profile_policy_json(declared_policy),
        "install_actions": install_plan
            .iter()
            .map(profile_plan_action_json)
            .collect::<Vec<_>>(),
        "remove_profile_anchors": removed_profile_anchors,
        "provider_reconciliation": provider_reconciliation,
        "provider_families": &runtime_view.provider_families,
        "pending_handler_transitions": &runtime_view.pending_handler_transitions,
        "required_activation_class": runtime_view.required_activation_class,
        "handler_backend": runtime_view.backend,
    })
}

fn profile_plan_action_json(action: &PlannedInstallAction) -> serde_json::Value {
    json!({
        "target": action.target,
        "package": action.package_name,
        "package_kind": action.resolved.recipe.package.kind,
        "selected_lane": action.resolved.selected_lane,
        "source_kind": action.resolved.selected_source_kind,
        "declared_profile_policy": action.resolved.recipe.package.profile,
        "action": if action.already_installed.is_some() {
            "keep-installed"
        } else {
            "install-base-anchor"
        },
    })
}
