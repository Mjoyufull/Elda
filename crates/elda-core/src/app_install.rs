mod binary_source;
mod dependency;
mod plan;
mod progress;
mod remote_recipe;
mod resolve;
mod review;
pub(crate) mod solver;

use std::io::IsTerminal;

pub(crate) use dependency::constraint::{
    package_satisfies_constraint, parse_dependency_constraint, provides_satisfy_constraint,
};

use serde_json::json;

use crate::app::{AppContext, PlannedInstallAction};
use crate::app_parse::installed_version;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use elda_install::{install_built_package, install_upgraded_package, remove_package_for_upgrade};

use progress::{
    install_progress_for_completed, install_progress_for_existing, install_progress_for_plan,
    planned_activation_backend,
};

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
            let activation_backend = planned_activation_backend(self.database.layout().mode);
            let actions = install_plan
                .iter()
                .map(|action| Self::install_action_json(action, activation_backend))
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

        self.review_generated_metadata_if_needed(&request, &install_plan)?;
        let installs = self.apply_install_plan(&install_plan, request.offline, request.output_mode)?;
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
        output_mode: OutputMode,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut installs = Vec::with_capacity(install_plan.len());
        let mutation_policy = self.mutation_policy();

        for action in install_plan {
            render_live_progress(
                action,
                "starting",
                output_mode,
                self.config.display.human_detail.as_str(),
            );
            let decision = install_execution_decision(action);

            if let Some(installed) = &action.already_installed
                && !decision.needs_change
            {
                if action.install_reason == "explicit" && installed.install_reason != "explicit" {
                    self.database
                        .set_install_reason(&action.package_name, "explicit")?;
                }

                installs.push(json!({
                    "target": action.target,
                    "selected_lane": action.resolved.selected_lane,
                    "selected_source_kind": action.resolved.selected_source_kind,
                    "persisted_source_kind": action.resolved.persisted_source_kind,
                    "source_ref": action.resolved.source_ref,
                    "generated_metadata_path": action.resolved.generated_recipe_dir,
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
                    "activation_backend": installed
                        .activation_backend
                        .clone()
                        .unwrap_or_else(|| planned_activation_backend(self.database.layout().mode).to_owned()),
                    "progress": install_progress_for_existing(
                        action,
                        installed
                            .activation_backend
                            .as_deref()
                            .unwrap_or(planned_activation_backend(self.database.layout().mode)),
                    ),
                    "action": decision.change_kind,
                }));
                continue;
            }

            let mut built = self.build_resolved_target(&action.resolved, offline)?;
            render_live_progress(
                action,
                "built payload",
                output_mode,
                self.config.display.human_detail.as_str(),
            );
            built.package.dependencies = Self::planned_dependency_records(&action.dependencies);
            let mut replaced = Vec::new();
            for package_name in &action.replaced_packages {
                replaced.push(remove_package_for_upgrade(
                    &self.database,
                    package_name,
                    &mutation_policy,
                )?);
            }
            let install = if let Some(installed) = &action.already_installed {
                if installed.held {
                    return Err(CoreError::Operator(format!(
                        "cannot change `{}` while it is held; clear the hold first",
                        action.package_name
                    )));
                }
                let candidate_version = format!(
                    "{}:{}-{}",
                    action.resolved.recipe.package.epoch,
                    action.resolved.recipe.package.version,
                    action.resolved.recipe.package.rel,
                );
                if let Some(pinned_version) = &installed.pinned_version
                    && pinned_version != &candidate_version
                {
                    return Err(CoreError::Operator(format!(
                        "cannot change `{}` while it is pinned to {}; clear the pin first",
                        action.package_name, pinned_version
                    )));
                }

                remove_package_for_upgrade(&self.database, &action.package_name, &mutation_policy)?;
                install_upgraded_package(
                    &self.database,
                    &built.package,
                    &action.install_reason,
                    installed.pinned_version.clone(),
                    installed.held,
                    installed.hold_source.clone(),
                    &mutation_policy,
                )?
            } else {
                install_built_package(
                    &self.database,
                    &built.package,
                    &action.install_reason,
                    None,
                    false,
                    None,
                    &mutation_policy,
                )?
            };
            render_live_progress(
                action,
                "activated and recorded state",
                output_mode,
                self.config.display.human_detail.as_str(),
            );
            installs.push(json!({
                "target": action.target,
                "selected_lane": built.resolved.selected_lane,
                "selected_source_kind": built.resolved.selected_source_kind,
                "persisted_source_kind": built.resolved.persisted_source_kind,
                "source_ref": built.resolved.source_ref,
                "generated_metadata_path": built.resolved.generated_recipe_dir,
                "remote_name": built.resolved.remote_name,
                "binary_source_verification": built.resolved.binary_source_verification,
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
                "activation_backend": install.activation_backend,
                "flag_state": {
                    "variant_id": built.resolved.flag_state.variant_id,
                    "effective_flags": built.resolved.flag_state.effective_flags,
                },
                "progress": install_progress_for_completed(action, &built.package, &install),
                "action": decision.change_kind,
            }));
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(installs)
    }

    fn install_action_json(
        action: &PlannedInstallAction,
        activation_backend: &str,
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
        "persisted_source_kind": action.resolved.persisted_source_kind,
        "source_ref": action.resolved.source_ref,
        "generated_metadata_path": action.resolved.generated_recipe_dir,
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

fn render_live_progress(
    action: &PlannedInstallAction,
    status: &str,
    output_mode: OutputMode,
    detail_level: &str,
) {
    if output_mode != OutputMode::Human
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return;
    }
    match detail_level {
        "minimal" => eprintln!("[install] {}: {status}", action.package_name),
        "verbose" => {
            eprintln!(
                "[install:running] {} ({}/{})",
                action.package_name, action.resolved.selected_lane, action.resolved.selected_source_kind
            );
            eprintln!(
                "  -> {status} | source={} | remote={}",
                action.resolved.selected_source_kind,
                action.resolved.remote_name.as_deref().unwrap_or("local")
            );
        }
        _ => {
            eprintln!(
                "[install:running] {} ({}/{})",
                action.package_name, action.resolved.selected_lane, action.resolved.selected_source_kind
            );
            eprintln!("  -> {status}");
        }
    }
}

struct InstallExecutionDecision {
    needs_change: bool,
    change_kind: &'static str,
}

fn install_execution_decision(action: &PlannedInstallAction) -> InstallExecutionDecision {
    let Some(installed) = &action.already_installed else {
        return InstallExecutionDecision {
            needs_change: true,
            change_kind: if !action.replaced_packages.is_empty()
                && action.install_reason == "explicit"
            {
                "install-replacing"
            } else if action.is_weak {
                "install-recommended"
            } else if action.install_reason == "explicit" {
                "install-explicit"
            } else {
                "install-dependency"
            },
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
        change_kind: if needs_change {
            if action.install_reason == "explicit" {
                "upgrade-explicit"
            } else {
                "upgrade-dependency"
            }
        } else {
            "keep-installed"
        },
    }
}
