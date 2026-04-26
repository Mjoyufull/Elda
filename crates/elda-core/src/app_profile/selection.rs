use std::collections::BTreeSet;

use serde_json::json;

use crate::app::{AppContext, ParsedInstallRequest, ResolvedProfileState};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};

use super::policy::profile_policy_json;
use super::selection_plan::{
    profile_anchors_to_remove, profile_selection_plan_json, remove_profile_anchors,
    validate_profile_apply_plan,
};
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

    pub(crate) fn import_profile_state(
        &self,
        desired: &crate::app::DesiredStateProfile,
        offline: bool,
    ) -> Result<serde_json::Value, CoreError> {
        if desired.active_profiles.is_empty() {
            let provider_reconciliation = self.apply_profile_backend_state(desired)?;
            self.write_profile_state(desired)?;
            return Ok(json!({
                "next_active_profiles": desired.active_profiles,
                "provider_reconciliation": provider_reconciliation,
            }));
        }

        let request = CommandRequest::new(
            vec![
                "state".to_owned(),
                "import".to_owned(),
                "profile".to_owned(),
            ],
            Vec::new(),
            OutputMode::Json,
            false,
        )
        .with_offline(offline);
        let parsed = ParsedProfileSelectionRequest {
            profiles: desired.active_profiles.clone(),
            init: (!desired.init.trim().is_empty()).then_some(desired.init.clone()),
            native_arch: (!desired.native_arch.trim().is_empty())
                .then_some(desired.native_arch.clone()),
            foreign_arches: desired.foreign_arches.clone(),
        };
        self.database.bootstrap()?;
        let current = self.resolve_profile_state()?;
        let report = self.execute_profile_selection(
            request,
            current,
            dedupe_preserve_order(desired.active_profiles.clone()),
            parsed,
            "state-import-profile",
            "imported",
        )?;

        Ok(report.details.unwrap_or_else(|| json!({})))
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
        validate_profile_apply_plan(&install_plan)?;
        self.validate_install_conflicts(&install_plan)?;
        let removed_profile_anchors = profile_anchors_to_remove(
            self,
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
        let provider_reconciliation = self.plan_profile_backend_state(&desired)?;
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
                    "plan": profile_selection_plan_json(
                        plan_kind,
                        &current,
                        &desired,
                        &declared_policy,
                        &install_plan,
                        &removed_profile_anchors,
                        &provider_reconciliation,
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

        let removed = remove_profile_anchors(self, &removed_profile_anchors)?;
        let provider_reconciliation = self.apply_profile_backend_state(&desired)?;
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
                "provider_reconciliation": provider_reconciliation,
                "provider_families": &runtime_view.provider_families,
                "pending_handler_transitions": &runtime_view.pending_handler_transitions,
                "required_activation_class": runtime_view.required_activation_class,
                "handler_backend": runtime_view.backend,
            })),
        })
    }
}
