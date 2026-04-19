mod binary_source;
mod dependency;
mod plan;
mod remote_recipe;
mod resolve;

pub(crate) use dependency::constraint::{
    package_satisfies_constraint, parse_dependency_constraint, provides_satisfy_constraint,
};

use serde_json::json;

use crate::app::{AppContext, PlannedInstallAction};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{install_built_package, remove_package_for_upgrade};

impl AppContext {
    pub(crate) fn handle_install(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let bootstrap = self.database.bootstrap()?;
        let parsed = self.parse_install_request(&request)?;
        let install_plan = self.plan_install_targets(&parsed)?;
        self.validate_install_conflicts(&install_plan)?;

        if request.dry_run {
            let actions = install_plan
                .iter()
                .map(Self::install_action_json)
                .collect::<Vec<_>>();

            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "install plan contains {} package action(s) across {} requested target(s).",
                    actions.len(),
                    parsed.targets.len(),
                ),
                details: Some(json!({
                    "schema_version": bootstrap.schema_version,
                    "created_database": bootstrap.created_database,
                    "layout": self.database.layout(),
                    "plan": {
                        "kind": "install",
                        "actions": actions,
                    },
                })),
            });
        }

        let installs = self.apply_install_plan(&install_plan, request.offline)?;
        Ok(CommandReport {
            area: "install",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "installed {} target(s) into the current Elda root.",
                installs.len(),
            ),
            details: Some(json!({
                "schema_version": bootstrap.schema_version,
                "created_database": bootstrap.created_database,
                "layout": self.database.layout(),
                "installs": installs,
            })),
        })
    }

    pub(crate) fn apply_install_plan(
        &self,
        install_plan: &[PlannedInstallAction],
        offline: bool,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut installs = Vec::with_capacity(install_plan.len());

        for action in install_plan {
            if let Some(installed) = &action.already_installed {
                if action.install_reason == "explicit" && installed.install_reason != "explicit" {
                    self.database
                        .set_install_reason(&action.package_name, "explicit")?;
                }

                installs.push(json!({
                    "target": action.target,
                    "selected_lane": action.resolved.selected_lane,
                    "selected_source_kind": action.resolved.selected_source_kind,
                    "persisted_source_kind": action.resolved.persisted_source_kind,
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
                        "installed_paths": self.database.package_files(&action.package_name)?.len(),
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
                }));
                continue;
            }

            let mut built = self.build_resolved_target(&action.resolved, offline)?;
            built.package.dependencies = Self::planned_dependency_records(&action.dependencies);
            let mut replaced = Vec::new();
            for package_name in &action.replaced_packages {
                replaced.push(remove_package_for_upgrade(&self.database, package_name)?);
            }
            let install = install_built_package(
                &self.database,
                &built.package,
                &action.install_reason,
                None,
                false,
                None,
            )?;
            installs.push(json!({
                "target": action.target,
                "selected_lane": built.resolved.selected_lane,
                "selected_source_kind": built.resolved.selected_source_kind,
                "persisted_source_kind": built.resolved.persisted_source_kind,
                "package": built.package,
                "install": install,
                "replacements": replaced,
                "status": "installed",
                "install_reason": action.install_reason,
                "requested_by": action.requested_by,
                "dependency_kind": action.dependency_kind,
                "is_weak": action.is_weak,
                "provider_group": action.provider_group,
                "replaced_packages": action.replaced_packages,
                "flag_state": {
                    "variant_id": built.resolved.flag_state.variant_id,
                    "effective_flags": built.resolved.flag_state.effective_flags,
                },
            }));
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(installs)
    }

    fn install_action_json(action: &PlannedInstallAction) -> serde_json::Value {
        json!({
            "action": match (
                &action.already_installed,
                action.replaced_packages.is_empty(),
                action.is_weak,
                action.install_reason.as_str(),
            ) {
                (Some(_), _, _, _) => "keep-installed",
                (None, false, _, "explicit") => "install-replacing",
                (None, _, true, _) => "install-recommended",
                (None, _, false, "explicit") => "install-explicit",
                _ => "install-dependency",
            },
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
            "persisted_source_kind": action.resolved.persisted_source_kind,
            "source_ref": action.resolved.source_ref,
            "variant_id": action.resolved.flag_state.variant_id,
            "install_reason": action.install_reason,
            "requested_by": action.requested_by,
            "dependency_kind": action.dependency_kind,
            "raw_expr": action.raw_expr,
            "is_weak": action.is_weak,
            "provider_group": action.provider_group,
            "replaced_packages": action.replaced_packages,
            "already_installed": action.already_installed.is_some(),
            "effective_flags": action.resolved.flag_state.effective_flags,
            "dependencies": action
                .dependencies
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
                .collect::<Vec<_>>(),
        })
    }
}
