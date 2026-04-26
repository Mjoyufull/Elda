use serde_json::json;
use std::fs;

use crate::app::{AppContext, DesiredStateDocument, DesiredStatePackage, ParsedInstallRequest};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{load_system_backend_status, pending_triggers};
use elda_repo::list_remotes;

impl AppContext {
    pub(crate) fn handle_ls(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let packages = self.database.list_installed_packages()?;

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} installed package(s).", packages.len()),
            details: Some(json!({ "packages": packages })),
        })
    }

    pub(crate) fn handle_state_show(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let snapshot = self.database.state_snapshot()?;
        let packages = self.database.list_installed_packages()?;
        let backend = load_system_backend_status(self.database.layout())?;

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "reported state for {} installed package(s).",
                packages.len()
            ),
            details: Some(json!({
                "schema_version": snapshot.schema_version,
                "active_state": snapshot.active_state,
                "world": snapshot.world,
                "packages": packages,
                "backend": backend,
            })),
        })
    }

    pub(crate) fn handle_check(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let mut report = self.database.health_report()?;
        let pending_triggers = pending_triggers(self.database.layout())?;
        let backend = load_system_backend_status(self.database.layout())?;
        if !pending_triggers.is_empty() {
            report.issues.push(format!(
                "pending trigger repair exists for {} system trigger(s)",
                pending_triggers.len()
            ));
        }
        let critical_pending = pending_triggers
            .iter()
            .filter(|record| record.critical)
            .count();
        if critical_pending > 0 {
            report.issues.push(format!(
                "pending critical boot trigger repair exists for {critical_pending} system trigger(s)"
            ));
        }

        Ok(CommandReport {
            area: "check",
            status: if report.issues.is_empty() {
                "ok"
            } else {
                "issues"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "reported {} health issue(s) for the current root.",
                report.issues.len(),
            ),
            details: Some(json!({
                "health": report,
                "pending_triggers": pending_triggers,
                "backend": backend,
            })),
        })
    }

    pub(crate) fn handle_state_export(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let output_path = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("state export requires an output path".to_owned()))?
            .clone();
        let snapshot = self.database.state_snapshot()?;
        let installed = self
            .database
            .list_installed_packages()?
            .into_iter()
            .map(|package| DesiredStatePackage {
                pkgname: package.pkgname,
                version: package.version,
                install_reason: package.install_reason,
                package_kind: package.package_kind,
                variant_id: package.variant_id,
                source_kind: package.source_kind,
                remote_name: package.remote_name,
                pinned_version: package.pinned_version,
                held: package.held,
                hold_source: package.hold_source,
            })
            .collect::<Vec<_>>();
        let profile_state = self.resolve_profile_state()?;
        let profile = profile_state.to_desired_profile(self.profile_state_base(&profile_state)?);
        let remotes = list_remotes(&self.database.layout().remotes_dir)?;
        let document = DesiredStateDocument {
            format_version: 1,
            exported_at: "0".to_owned(),
            installation_mode: installation_mode_name(self.database.layout().mode).to_owned(),
            prefix: self.database.layout().prefix.display().to_string(),
            profile,
            remotes,
            world: snapshot.world,
            installed,
        };

        fs::write(&output_path, serde_json::to_vec_pretty(&document)?)?;

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("exported desired state to `{output_path}`."),
            details: Some(json!({ "exported": document })),
        })
    }

    pub(crate) fn handle_state_import(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let input_path = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("state import requires an input path".to_owned()))?;
        let content = fs::read_to_string(input_path)?;
        let document = serde_json::from_str::<DesiredStateDocument>(&content)?;
        let profile_backend_reconciliation = self.plan_profile_backend_state(&document.profile)?;

        if request.dry_run {
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "state import would write {} remote(s) and install {} world target(s).",
                    document.remotes.len(),
                    document.world.len(),
                ),
                details: Some(json!({
                    "plan": {
                        "kind": "state-import",
                        "remotes": document.remotes,
                        "world": document.world,
                        "profile_backend_reconciliation": profile_backend_reconciliation,
                    },
                })),
            });
        }

        self.persist_imported_remotes(&document)?;
        if !document.remotes.is_empty() {
            elda_repo::sync_remotes(
                &self.database.layout().remotes_dir,
                &self.repo_snapshot_path(),
                self.sync_options(&request),
            )?;
        }
        let imported_profile = self.import_profile_state(&document.profile, request.offline)?;
        if !document.world.is_empty() {
            let install_request = ParsedInstallRequest {
                targets: document.world.clone(),
                hard_lane: None,
                preferred_lane: None,
                cli_flag_overrides: Default::default(),
            };
            let plan = self.plan_install_targets(&install_request)?;
            self.validate_install_conflicts(&plan)?;
            self.apply_install_plan(&plan, request.offline)?;
        }

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "imported desired state with {} world target(s).",
                document.world.len(),
            ),
            details: Some(json!({
                "imported": {
                    "remotes": document.remotes,
                    "world": document.world,
                    "profile": document.profile,
                    "profile_backend_reconciliation": imported_profile,
                },
            })),
        })
    }

    pub(crate) fn handle_stub(&self, request: CommandRequest) -> CommandReport {
        CommandReport {
            area: "command",
            status: "unsupported",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "this command exists in the CLI surface but is not implemented yet."
                .to_owned(),
            details: Some(json!({ "implemented": false })),
        }
    }

    fn persist_imported_remotes(&self, document: &DesiredStateDocument) -> Result<(), CoreError> {
        fs::create_dir_all(&self.database.layout().remotes_dir)?;

        for remote in &document.remotes {
            let path = self
                .database
                .layout()
                .remotes_dir
                .join(format!("{}.toml", remote.name));
            let encoded = toml::to_string_pretty(remote)
                .map_err(|error| CoreError::Operator(error.to_string()))?;
            fs::write(path, encoded)?;
        }

        Ok(())
    }
}

fn installation_mode_name(mode: elda_db::InstallationMode) -> &'static str {
    match mode {
        elda_db::InstallationMode::System => "system",
        elda_db::InstallationMode::Prefix => "prefix",
    }
}
