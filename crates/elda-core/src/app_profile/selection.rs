use std::collections::BTreeSet;

use serde_json::json;

use crate::app::{AppContext, ParsedInstallRequest, PlannedInstallAction, ResolvedProfileState};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::remove_package;

use super::policy::profile_policy_json;
use super::selection_request::{
    ParsedProfileSelectionRequest, ensure_active_profiles_exist, next_foreign_arches,
    parse_profile_selection_request,
};
use super::{dedupe_preserve_order, empty_to};

impl AppContext {
    pub(crate) fn handle_profile_apply(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let parsed = parse_profile_selection_request(&request, "pf apply")?;
        self.database.bootstrap()?;
        let current = self.resolve_profile_state()?;

        self.execute_profile_selection(
            request,
            current,
            dedupe_preserve_order(parsed.profiles.clone()),
            parsed,
            "profile-apply",
            "applied",
        )
    }

    pub(crate) fn handle_profile_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let parsed = parse_profile_selection_request(&request, "pf add")?;
        self.database.bootstrap()?;
        let current = self.resolve_profile_state()?;
        let mut active_profiles = current.active_profiles.clone();
        active_profiles.extend(parsed.profiles.iter().cloned());

        self.execute_profile_selection(
            request,
            current,
            dedupe_preserve_order(active_profiles),
            parsed,
            "profile-add",
            "updated",
        )
    }

    pub(crate) fn handle_profile_remove(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let parsed = parse_profile_selection_request(&request, "pf rm")?;
        self.database.bootstrap()?;
        let current = self.resolve_profile_state()?;
        ensure_active_profiles_exist(&current.active_profiles, &parsed.profiles)?;
        let removed = parsed.profiles.iter().cloned().collect::<BTreeSet<_>>();
        let active_profiles = current
            .active_profiles
            .iter()
            .filter(|profile| !removed.contains(*profile))
            .cloned()
            .collect::<Vec<_>>();

        self.execute_profile_selection(
            request,
            current,
            active_profiles,
            parsed,
            "profile-remove",
            "updated",
        )
    }

    fn execute_profile_selection(
        &self,
        request: CommandRequest,
        current: ResolvedProfileState,
        active_profiles: Vec<String>,
        parsed: ParsedProfileSelectionRequest,
        plan_kind: &str,
        status_summary: &str,
    ) -> Result<CommandReport, CoreError> {
        let install_request = ParsedInstallRequest {
            targets: active_profiles.clone(),
            hard_lane: None,
            preferred_lane: None,
            cli_flag_overrides: Default::default(),
        };
        let install_plan = self.plan_install_targets(&install_request)?;
        self.validate_profile_apply_plan(&install_plan)?;
        self.validate_install_conflicts(&install_plan)?;
        let removed_profile_anchors = self.profile_anchors_to_remove(
            &current.active_profiles,
            &active_profiles,
            &install_plan,
        )?;
        let declared_policy = self.resolve_requested_profile_policy(&install_plan)?;
        let next_profile = ResolvedProfileState {
            active_profiles: active_profiles.clone(),
            native_arch: parsed
                .native_arch
                .clone()
                .or(declared_policy.native_arch.clone())
                .unwrap_or_else(|| current.native_arch.clone()),
            foreign_arches: next_foreign_arches(&current, &parsed, &declared_policy),
            init: parsed
                .init
                .clone()
                .or(declared_policy.init.clone())
                .unwrap_or_else(|| current.init.clone()),
        };
        let desired =
            next_profile.to_desired_profile(active_profiles.first().cloned().unwrap_or_default());
        let runtime_view = self.profile_runtime_view(&next_profile)?;

        if request.dry_run {
            return Ok(CommandReport {
                area: "profile",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "planned machine-profile update with {} active anchor(s).",
                    desired.active_profiles.len()
                ),
                details: Some(json!({
                    "plan": self.profile_selection_plan_json(
                        plan_kind,
                        &current,
                        &desired,
                        &declared_policy,
                        &install_plan,
                        &removed_profile_anchors,
                        &runtime_view,
                    ),
                })),
            });
        }

        let installs = self.apply_install_plan(&install_plan, request.offline)?;
        for action in install_plan
            .iter()
            .filter(|action| action.install_reason == "explicit")
        {
            self.database
                .set_install_reason(&action.package_name, "base")?;
        }

        let removed = self.remove_profile_anchors(&removed_profile_anchors)?;
        self.write_profile_state(&desired)?;
        let runtime_view = self.profile_runtime_view(&next_profile)?;

        Ok(CommandReport {
            area: "profile",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "{status_summary} machine profile state with {} active anchor(s).",
                desired.active_profiles.len()
            ),
            details: Some(json!({
                "previous_active_profiles": current.active_profiles,
                "next_active_profiles": desired.active_profiles,
                "previous_native_arch": current.native_arch,
                "next_native_arch": desired.native_arch,
                "previous_init": empty_to(current.init, "unset".to_owned()),
                "next_init": empty_to(desired.init.clone(), "unset".to_owned()),
                "previous_foreign_arches": current.foreign_arches,
                "next_foreign_arches": desired.foreign_arches,
                "declared_profile_policy": profile_policy_json(&declared_policy),
                "install_actions": installs,
                "removed_profile_anchors": removed,
                "provider_families": &runtime_view.provider_families,
                "pending_handler_transitions": &runtime_view.pending_handler_transitions,
                "required_activation_class": runtime_view.required_activation_class,
                "handler_backend": runtime_view.backend,
            })),
        })
    }

    fn validate_profile_apply_plan(
        &self,
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

    fn profile_anchors_to_remove(
        &self,
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
            let reverse_dependencies = self.database.reverse_dependencies(&anchor, false)?;
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

    fn remove_profile_anchors(
        &self,
        removed_profile_anchors: &[String],
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut removed = Vec::new();

        for anchor in removed_profile_anchors {
            if self.database.installed_package(anchor)?.is_some() {
                removed.push(json!(remove_package(&self.database, anchor)?));
            }
        }

        Ok(removed)
    }

    fn profile_selection_plan_json(
        &self,
        plan_kind: &str,
        current: &ResolvedProfileState,
        desired: &crate::app::DesiredStateProfile,
        declared_policy: &super::policy::ProfilePolicyResolution,
        install_plan: &[PlannedInstallAction],
        removed_profile_anchors: &[String],
        runtime_view: &super::system_changes::ProfileRuntimeView,
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
            "provider_families": &runtime_view.provider_families,
            "pending_handler_transitions": &runtime_view.pending_handler_transitions,
            "required_activation_class": runtime_view.required_activation_class,
            "handler_backend": runtime_view.backend,
        })
    }
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
