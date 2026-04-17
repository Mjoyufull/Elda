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

        Ok(CommandReport {
            area: "ops",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "the current backend has no pending trigger handlers.".to_owned(),
            details: Some(json!({
                "pending_handlers": [],
                "backend": "prefix-copy",
            })),
        })
    }
}
