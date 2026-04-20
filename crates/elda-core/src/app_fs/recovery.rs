use super::*;

impl AppContext {
    pub(crate) fn handle_recover(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let report = recover_pending_transactions(&self.database)?;

        Ok(CommandReport {
            area: "recovery",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("recovered {} pending journal(s).", report.recovered.len()),
            details: Some(json!({ "recovery": report })),
        })
    }

    pub(crate) fn handle_rollback(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let target = request.operands.first().map(String::as_str);
        if request.dry_run {
            let plan = rollback_plan(&self.database, target)?;
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!("planned rollback to `{}`.", plan.to_state),
                details: Some(json!({ "plan": plan })),
            });
        }

        let report = rollback_state(&self.database, target)?;
        let _ = self.reconcile_cache_policy()?;
        Ok(CommandReport {
            area: "rollback",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("rolled back to archived state `{}`.", report.to_state),
            details: Some(json!({ "rollback": report })),
        })
    }

    pub(crate) fn handle_fix_triggers(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let profile = self.resolve_profile_state()?;
        let desired = profile.to_desired_profile(self.profile_state_base(&profile)?);
        let profile_backend_repair = self.apply_profile_backend_state(&desired)?;
        let runtime_view = self.profile_runtime_view(&profile)?;
        let trigger_report = repair_triggers(&self.database)?;
        let pending_count =
            runtime_view.pending_handler_transitions.len() + trigger_report.pending.len();

        Ok(CommandReport {
            area: "ops",
            status: if pending_count == 0 { "ok" } else { "pending" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if pending_count == 0 {
                "the current backend has no pending trigger handlers.".to_owned()
            } else {
                format!(
                    "reported {pending_count} pending system-change handler(s) for the current backend."
                )
            },
            details: Some(json!({
                "pending_handlers": &runtime_view.pending_handler_transitions,
                "provider_asset_repair": profile_backend_repair.clone(),
                "profile_backend_repair": profile_backend_repair,
                "trigger_repair": trigger_report,
                "provider_families": &runtime_view.provider_families,
                "required_activation_class": runtime_view.required_activation_class,
                "backend": runtime_view.backend,
            })),
        })
    }
}
