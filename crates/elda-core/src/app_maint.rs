use serde_json::json;

use crate::app::AppContext;
use crate::app_confirm::confirm_mutation;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{load_system_backend_status, pending_triggers, repair_triggers};
use elda_repo::list_remotes;

impl AppContext {
    pub(crate) fn handle_maint_check(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let layout = self.database.layout();
        let health = self.database.health_report()?;
        let remotes = list_remotes(&layout.remotes_dir)?;
        let pending_trigger_records = pending_triggers(layout)?;
        let backend = load_system_backend_status(layout)?;
        let modules = maint_modules(&health.issues, &pending_trigger_records, &remotes);

        Ok(CommandReport {
            area: "maint",
            status: if modules
                .iter()
                .all(|module| module.get("status") == Some(&json!("ok")))
            {
                "ok"
            } else {
                "issues"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("checked {} maintenance module(s).", modules.len()),
            details: Some(json!({
                "modules": modules,
                "health": health,
                "pending_triggers": pending_trigger_records,
                "backend": backend,
                "remote_count": remotes.len(),
            })),
        })
    }

    pub(crate) fn handle_maint_fix(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let module = request
            .operands
            .first()
            .cloned()
            .unwrap_or_else(|| "all".to_owned());

        confirm_mutation(&request, &format!("Run maintenance fix module `{module}`?"))?;

        let mut actions = Vec::new();
        if module == "all" || module.as_str() == "recovery" {
            let recovery = elda_install::recover_pending_transactions(&self.database)?;
            actions.push(json!({
                "module": "recovery",
                "status": "ok",
                "recovered": recovery.recovered,
            }));
        }
        if module == "all" || module.as_str() == "triggers" {
            let trigger_report = repair_triggers(&self.database)?;
            actions.push(json!({
                "module": "triggers",
                "status": if trigger_report.pending.is_empty() { "ok" } else { "pending" },
                "repaired": trigger_report.repaired,
                "pending": trigger_report.pending,
            }));
        }
        if module == "all" || module.as_str() == "profile" {
            let profile = self.resolve_profile_state()?;
            let desired = profile.to_desired_profile(self.profile_state_base(&profile)?);
            let profile_backend_repair = self.apply_profile_backend_state(&desired)?;
            let trigger_report = repair_triggers(&self.database)?;
            actions.push(json!({
                "module": "profile",
                "status": if trigger_report.pending.is_empty() { "ok" } else { "pending" },
                "profile_backend_repair": profile_backend_repair,
                "pending_triggers": trigger_report.pending,
            }));
        }

        let status = if actions
            .iter()
            .all(|action| action.get("status").and_then(|v| v.as_str()) == Some("ok"))
        {
            "ok"
        } else {
            "pending"
        };

        Ok(CommandReport {
            area: "maint",
            status,
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("ran maintenance fix module `{module}`."),
            details: Some(json!({ "module": module, "actions": actions })),
        })
    }
}

fn maint_modules(
    health_issues: &[String],
    pending_triggers: &[elda_install::PendingTriggerRecord],
    remotes: &[elda_repo::RemoteDocument],
) -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "world",
            "status": if health_issues.is_empty() { "ok" } else { "issues" },
            "issues": health_issues,
        }),
        json!({
            "name": "triggers",
            "status": if pending_triggers.is_empty() { "ok" } else { "pending" },
            "pending": pending_triggers,
        }),
        json!({
            "name": "remotes",
            "status": "ok",
            "configured": remotes.len(),
            "disabled": remotes.iter().filter(|remote| !remote.enabled).count(),
        }),
    ]
}
