use serde::Serialize;
use serde_json::json;
use std::fs;

use crate::app::{AppContext, DesiredStateDocument, DesiredStatePackage, ParsedInstallRequest};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{load_system_backend_status, pending_triggers};
use elda_repo::list_remotes;

impl AppContext {
    pub(crate) fn handle_ls(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.list_installed_packages_report(request, ListOperandMode::FiltersOnly)
    }

    pub(crate) fn handle_list(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.list_installed_packages_report(request, ListOperandMode::FiltersAndPackages)
    }

    fn list_installed_packages_report(
        &self,
        request: CommandRequest,
        operand_mode: ListOperandMode,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let mut packages = self.database.list_installed_packages()?;
        let parsed = parse_list_operands(&request.operands, operand_mode)?;
        apply_list_filters(&mut packages, &parsed.filters);
        if !parsed.packages.is_empty() {
            packages.retain(|package| parsed.packages.contains(&package.pkgname));
        }

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} installed package(s).", packages.len()),
            details: Some(json!({ "packages": packages, "filters": parsed.filters })),
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
            exit_status: if report.issues.is_empty() {
                ExitStatus::Success
            } else {
                ExitStatus::OperatorFailure
            },
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
                source_option: None,
                source_strategy: None,
                git_ref: None,
                git_source_refs: Default::default(),
                git_ref_overrides: Default::default(),
                cli_flag_overrides: Default::default(),
                replace: false,
                exclude: Vec::new(),
                provider_choices: Default::default(),
            };
            let plan = self.plan_install_targets(&install_request, None)?;
            self.validate_install_conflicts(&plan)?;
            self.apply_install_plan(&plan, &request)?;
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
            exit_status: ExitStatus::OperatorFailure,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "this command exists in the CLI surface but is not implemented yet."
                .to_owned(),
            details: Some(json!({
                "implemented": false,
                "blocked": true,
                "action": "use `elda --help` to pick a supported command, or treat this namespace as release-blocked until it has a real handler"
            })),
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

#[derive(Debug, Clone, Copy)]
enum ListOperandMode {
    FiltersOnly,
    FiltersAndPackages,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ListFilters {
    explicit: bool,
    deps: bool,
    held: bool,
    pinned: bool,
    source_kind: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ParsedListOperands {
    packages: Vec<String>,
    filters: ListFilters,
}

fn parse_list_operands(
    operands: &[String],
    mode: ListOperandMode,
) -> Result<ParsedListOperands, CoreError> {
    let command = match mode {
        ListOperandMode::FiltersOnly => "ls",
        ListOperandMode::FiltersAndPackages => "list",
    };

    if matches!(mode, ListOperandMode::FiltersOnly) {
        return Ok(ParsedListOperands {
            packages: Vec::new(),
            filters: parse_list_filters(operands, command)?,
        });
    }

    let mut packages = Vec::new();
    let mut filter_operands = Vec::new();
    let mut iter = operands.iter();
    while let Some(operand) = iter.next() {
        if operand.starts_with("--") {
            filter_operands.push(operand.clone());
            if operand == "--source-kind" {
                if let Some(value) = iter.next() {
                    filter_operands.push(value.clone());
                }
            }
        } else {
            packages.push(operand.clone());
        }
    }

    Ok(ParsedListOperands {
        packages,
        filters: parse_list_filters(&filter_operands, command)?,
    })
}

fn parse_list_filters(operands: &[String], command: &str) -> Result<ListFilters, CoreError> {
    let mut filters = ListFilters::default();
    let mut iter = operands.iter();
    while let Some(operand) = iter.next() {
        match operand.as_str() {
            "--explicit" => filters.explicit = true,
            "--deps" => filters.deps = true,
            "--held" => filters.held = true,
            "--pinned" => filters.pinned = true,
            "--source-kind" => {
                let value = iter.next().ok_or_else(|| {
                    CoreError::Operator(format!("{command} --source-kind requires a value"))
                })?;
                filters.source_kind = Some(value.clone());
            }
            other if other.starts_with("--source-kind=") => {
                filters.source_kind = Some(other.trim_start_matches("--source-kind=").to_owned());
            }
            other => {
                return Err(CoreError::Operator(format!(
                    "unexpected {command} operand or flag `{other}`"
                )));
            }
        }
    }
    if filters.explicit && filters.deps {
        return Err(CoreError::Operator(format!(
            "{command} --explicit and --deps cannot be combined"
        )));
    }
    Ok(filters)
}

fn apply_list_filters(packages: &mut Vec<elda_db::InstalledPackageRecord>, filters: &ListFilters) {
    packages.retain(|package| {
        (!filters.explicit || package.install_reason == "explicit")
            && (!filters.deps || package.install_reason == "dep")
            && (!filters.held || package.held)
            && (!filters.pinned || package.pinned_version.is_some())
            && filters
                .source_kind
                .as_ref()
                .is_none_or(|kind| package.source_kind == *kind)
    });
}

fn installation_mode_name(mode: elda_db::InstallationMode) -> &'static str {
    match mode {
        elda_db::InstallationMode::System => "system",
        elda_db::InstallationMode::Prefix => "prefix",
    }
}
