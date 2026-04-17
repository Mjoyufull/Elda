use std::collections::BTreeSet;

use serde_json::json;

use crate::app::{AppContext, ParsedInstallRequest, PlannedInstallAction, ResolvedProfileState};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::remove_package;

use super::{dedupe_preserve_order, empty_to};

impl AppContext {
    pub(crate) fn handle_profile_apply(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = parse_profile_apply_request(&request)?;
        let current = self.resolve_profile_state()?;
        let active_profiles = dedupe_preserve_order(parsed.profiles);
        let next_foreign_arches = if parsed.foreign_arches.is_empty() {
            current.foreign_arches.clone()
        } else {
            dedupe_preserve_order(parsed.foreign_arches)
        };
        let next_init = parsed.init.unwrap_or_else(|| current.init.clone());
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
        let next_profile = ResolvedProfileState {
            active_profiles: active_profiles.clone(),
            native_arch: current.native_arch.clone(),
            foreign_arches: next_foreign_arches.clone(),
            init: next_init.clone(),
        };
        let desired =
            next_profile.to_desired_profile(active_profiles.first().cloned().unwrap_or_default());

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
                    "planned application of {} profile anchor(s).",
                    active_profiles.len()
                ),
                details: Some(json!({
                    "plan": {
                        "kind": "profile-apply",
                        "previous_active_profiles": current.active_profiles,
                        "next_active_profiles": active_profiles,
                        "previous_init": empty_to(current.init, "unset".to_owned()),
                        "next_init": empty_to(next_init, "unset".to_owned()),
                        "previous_foreign_arches": current.foreign_arches,
                        "next_foreign_arches": next_foreign_arches,
                        "install_actions": install_plan
                            .iter()
                            .map(profile_plan_action_json)
                            .collect::<Vec<_>>(),
                        "remove_profile_anchors": removed_profile_anchors,
                        "pending_handler_transitions": [],
                        "required_activation_class": "none",
                    },
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

        let mut removed = Vec::new();
        for anchor in &removed_profile_anchors {
            if self.database.installed_package(anchor)?.is_some() {
                removed.push(remove_package(&self.database, anchor)?);
            }
        }
        self.write_profile_state(&desired)?;

        Ok(CommandReport {
            area: "profile",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "applied {} profile anchor(s) into the current root.",
                desired.active_profiles.len()
            ),
            details: Some(json!({
                "previous_active_profiles": current.active_profiles,
                "next_active_profiles": desired.active_profiles,
                "previous_init": empty_to(current.init, "unset".to_owned()),
                "next_init": empty_to(desired.init.clone(), "unset".to_owned()),
                "previous_foreign_arches": current.foreign_arches,
                "next_foreign_arches": desired.foreign_arches,
                "install_actions": installs,
                "removed_profile_anchors": removed,
                "provider_families": {
                    "init": desired.init,
                },
                "pending_handler_transitions": [],
                "required_activation_class": "none",
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
                    "`pf apply` target `{}` is not a `package_kind = profile` recipe",
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
                    "profile anchor `{anchor}` is still required by the requested profile set; include it explicitly in `pf apply`",
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
}

fn profile_plan_action_json(action: &PlannedInstallAction) -> serde_json::Value {
    json!({
        "target": action.target,
        "package": action.package_name,
        "package_kind": action.resolved.recipe.package.kind,
        "selected_lane": action.resolved.selected_lane,
        "source_kind": action.resolved.selected_source_kind,
        "action": if action.already_installed.is_some() {
            "keep-installed"
        } else {
            "install-base-anchor"
        },
    })
}

#[derive(Debug)]
struct ParsedProfileApplyRequest {
    profiles: Vec<String>,
    init: Option<String>,
    foreign_arches: Vec<String>,
}

fn parse_profile_apply_request(
    request: &CommandRequest,
) -> Result<ParsedProfileApplyRequest, CoreError> {
    let mut profiles = Vec::new();
    let mut init = None;
    let mut foreign_arches = Vec::new();
    let mut operands = request.operands.iter();

    while let Some(operand) = operands.next() {
        match operand.as_str() {
            "--init" => {
                let provider = operands.next().ok_or_else(|| {
                    CoreError::Operator("`pf apply --init` requires one provider value".to_owned())
                })?;
                if provider.trim().is_empty() {
                    return Err(CoreError::Operator(
                        "invalid empty init-provider for `pf apply --init`".to_owned(),
                    ));
                }
                if init.replace(provider.clone()).is_some() {
                    return Err(CoreError::Operator(
                        "`pf apply` accepts at most one `--init` value".to_owned(),
                    ));
                }
            }
            "--foreign-arch" => {
                let arch = operands.next().ok_or_else(|| {
                    CoreError::Operator(
                        "`pf apply --foreign-arch` requires one architecture value".to_owned(),
                    )
                })?;
                if arch.trim().is_empty() {
                    return Err(CoreError::Operator(
                        "invalid empty architecture for `pf apply --foreign-arch`".to_owned(),
                    ));
                }
                foreign_arches.push(arch.clone());
            }
            other if other.starts_with("--") => {
                return Err(CoreError::Operator(format!(
                    "unexpected `pf apply` flag `{other}`"
                )));
            }
            _ => profiles.push(operand.clone()),
        }
    }

    if profiles.is_empty() {
        return Err(CoreError::Operator(
            "`pf apply` requires at least one profile anchor".to_owned(),
        ));
    }

    Ok(ParsedProfileApplyRequest {
        profiles,
        init,
        foreign_arches,
    })
}
