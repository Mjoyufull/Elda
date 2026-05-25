mod binary_source;
mod dependency;
pub(crate) mod git_picker;
mod install_confirm;
mod interbuild_review;
mod plan;
mod preflight;
mod progress;
mod progress_emit;
mod provider_choice;
mod remote_recipe;
mod report;
mod resolve;
mod review;
mod review_metadata;
mod review_recheck;
pub(crate) mod solver;
mod source_options;
mod transaction_gate;

pub(crate) use dependency::constraint::{
    package_satisfies_constraint, parse_dependency_constraint, provides_satisfy_constraint,
};
pub(crate) use resolve::{ResolutionReport, parse_ad_hoc_git_source_ref, set_git_ref};

use serde_json::json;

use crate::app::{AppContext, PlannedInstallAction};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use elda_install::{install_built_package, install_upgraded_package, remove_package_for_upgrade};

use preflight::install_preflight_report;
use progress::{install_progress_for_completed, planned_activation_backend};
use progress_emit::{
    emit_acquire_and_build_done, emit_already_installed_frame, emit_frame_blocked,
    emit_frame_start, emit_install_completed, emit_step_started,
};
use report::{
    already_installed_json, fallback_activation_backend, install_execution_decision,
    interbuild_details_for_report, planned_install_action_json,
};
use source_options::apply_interactive_source_option_selection;
use transaction_gate::confirm_install_transaction;

impl AppContext {
    pub(crate) fn handle_install(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let bootstrap = self.database.bootstrap()?;
        let mut parsed = self.parse_install_request(&request)?;
        git_picker::apply_pick_tag_selection(self, &request, &mut parsed)?;
        let mut install_plan = self.plan_install_targets(&parsed, Some(&request))?;
        let selected = apply_interactive_source_option_selection(
            &request,
            &parsed,
            &install_plan,
            self.config.metadata.link_option_mode,
        )?;
        let parsed = selected.unwrap_or(parsed);
        if parsed.source_option.is_some() {
            install_plan = self.plan_install_targets(&parsed, Some(&request))?;
        }
        self.validate_install_conflicts(&install_plan)?;

        if request.dry_run {
            let preflight = install_preflight_report(self, &request, &install_plan)?;
            let activation_backend = planned_activation_backend(self.database.layout().mode);
            let actions = install_plan
                .iter()
                .map(|action| {
                    planned_install_action_json(
                        action,
                        activation_backend,
                        self.config.metadata.link_option_mode,
                    )
                })
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
                    "preflight": preflight,
                    "plan": {
                        "kind": "install",
                        "link_option_mode": self.config.metadata.link_option_mode,
                        "actions": actions,
                    },
                })),
            });
        }

        self.review_generated_metadata_if_needed(&request, &install_plan)?;
        confirm_install_transaction(self, &request, &install_plan)?;
        let installs = self.apply_install_plan(&install_plan, &request)?;
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
                "link_option_mode": self.config.metadata.link_option_mode,
                "installs": installs,
            })),
        })
    }

    pub(crate) fn apply_install_plan(
        &self,
        install_plan: &[PlannedInstallAction],
        request: &CommandRequest,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut installs = Vec::with_capacity(install_plan.len());
        let mutation_policy = self.mutation_policy();
        let offline = request.offline;
        let stream_child_output = request.output_mode == OutputMode::Human && !request.no_stream;

        for action in install_plan {
            let frame = self.next_frame_id();
            let sink = self.progress_sink();
            emit_frame_start(sink, frame, action);
            let decision = install_execution_decision(action);

            if let Some(installed) = &action.already_installed
                && !decision.needs_change
            {
                if action.install_reason == "explicit" && installed.install_reason != "explicit" {
                    self.database
                        .set_install_reason(&action.package_name, "explicit")?;
                }

                let fallback_backend = fallback_activation_backend(self.database.layout().mode);
                let installed_paths = self.database.package_files(&action.package_name)?.len();
                emit_already_installed_frame(sink, frame, action);
                installs.push(already_installed_json(
                    action,
                    installed,
                    installed_paths,
                    fallback_backend,
                    self.config.metadata.link_option_mode,
                ));
                continue;
            }

            emit_step_started(sink, frame, "acquire-source", "acquire source", None, true);
            if stream_child_output {
                emit_step_started(
                    sink,
                    frame,
                    "build-inner",
                    "build source",
                    Some(format!(
                        "{}/{}",
                        action.resolved.selected_lane, action.resolved.selected_source_kind
                    )),
                    false,
                );
            }
            let build_line_hook = stream_child_output.then(|| {
                let sink = self.progress_sink_arc();
                std::sync::Arc::new(move |line: &str| {
                    use crate::progress::{ProgressEvent, ProgressUnit};
                    sink.emit(ProgressEvent::StepProgress {
                        frame,
                        step: "build-inner",
                        label: line.to_owned(),
                        current: 0,
                        total: None,
                        unit: ProgressUnit::Items,
                    });
                }) as std::sync::Arc<dyn Fn(&str) + Send + Sync>
            });
            let mut built = match self.build_resolved_target(
                &action.resolved,
                offline,
                stream_child_output,
                build_line_hook,
            ) {
                Ok(built) => built,
                Err(error) => {
                    emit_frame_blocked(
                        sink,
                        frame,
                        "acquire-source",
                        "acquire source",
                        error.to_string(),
                    );
                    return Err(error);
                }
            };
            emit_acquire_and_build_done(sink, frame, action, &built.package);
            built.package.dependencies = Self::planned_dependency_records(&action.dependencies);
            let mut replaced = Vec::new();
            for package_name in &action.replaced_packages {
                replaced.push(remove_package_for_upgrade(
                    &self.database,
                    package_name,
                    &mutation_policy,
                )?);
            }
            emit_step_started(sink, frame, "activate", "activate", None, true);
            let install = if let Some(installed) = &action.already_installed {
                if installed.held {
                    let reason = format!(
                        "cannot change `{}` while it is held; clear the hold first",
                        action.package_name
                    );
                    emit_frame_blocked(sink, frame, "activate", "activate", reason.clone());
                    return Err(CoreError::Operator(reason));
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
                    let reason = format!(
                        "cannot change `{}` while it is pinned to {}; clear the pin first",
                        action.package_name, pinned_version
                    );
                    emit_frame_blocked(sink, frame, "activate", "activate", reason.clone());
                    return Err(CoreError::Operator(reason));
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
            emit_install_completed(sink, frame, action, &install);
            installs.push(json!({
                "target": action.target,
                "selected_lane": built.resolved.selected_lane,
                "selected_source_kind": built.resolved.selected_source_kind,
                "persisted_source_kind": built.resolved.persisted_source_kind,
                "source_ref": built.resolved.source_ref,
                "generated_metadata_path": built.resolved.generated_recipe_dir,
                "source_options": built.resolved.source_options,
                "selected_source_option": built.resolved.selected_source_option,
                "link_option_mode": self.config.metadata.link_option_mode,
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
                "interbuild": built.package.interbuild.clone().map(serde_json::to_value).transpose()?.or_else(|| interbuild_details_for_report(&built.resolved)),
                "action": decision.change_kind,
            }));
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(installs)
    }
}
