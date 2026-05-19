use serde_json::json;

use crate::app::AppContext;
use crate::app_confirm::confirm_mutation;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_repo::{
    inspect_remote_trust, list_remotes, load_remote, load_snapshot,
    preview_interemote_with_protocols, remove_remote, save_remote,
};

impl AppContext {
    pub(crate) fn handle_remote_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let input = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("rmt add requires `<name>=<url>`".to_owned()))?;
        let remote_document = self.parse_remote_add_request(&request, input)?;
        let existing = load_remote(&self.database.layout().remotes_dir, &remote_document.name)?;
        if existing.is_some() && !request_has_flag(&request, "--replace") {
            return Err(CoreError::Operator(format!(
                "remote `{}` already exists; pass `--replace` to overwrite it",
                remote_document.name
            )));
        }
        confirm_mutation(
            &request,
            &format!(
                "Register remote `{}` at {}?",
                remote_document.name, remote_document.index_url
            ),
        )?;
        if request.dry_run {
            return Ok(CommandReport {
                area: "remote",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!(
                    "would register remote `{}` at {}.",
                    remote_document.name, remote_document.index_url
                ),
                details: Some(json!({
                    "remote": remote_document,
                    "registered": false,
                    "replaced_existing": existing.is_some(),
                })),
            });
        }
        let remote = save_remote(&self.database.layout().remotes_dir, remote_document)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("registered remote `{}`.", remote.name),
            details: Some(json!({ "remote": remote })),
        })
    }

    pub(crate) fn handle_remote_list(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let mut remotes = list_remotes(&self.database.layout().remotes_dir)?;
        remotes.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} configured remote(s).", remotes.len()),
            details: Some(json!({ "remotes": remotes })),
        })
    }

    pub(crate) fn handle_remote_info(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("rmt info requires a remote name".to_owned()))?;
        let remote = self.load_registered_remote(name)?;
        let snapshot_record = self.snapshot_remote_record(&remote.name)?;
        let indexed_packages = self.snapshot_package_names(&remote.name)?;
        let installed_packages = self.installed_from_remote(&remote.name)?;
        let remote_kind = remote_kind(
            &remote.index_url,
            snapshot_record
                .as_ref()
                .map(|record| record.source.as_str()),
        );
        let trust_report = inspect_remote_trust(&self.repo_snapshot_path(), &remote)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("remote `{}` is configured.", remote.name),
            details: Some(json!({
                "remote": remote,
                "kind": remote_kind,
                "snapshot": snapshot_record,
                "trust_report": trust_report,
                "indexed_packages": indexed_packages,
                "installed_packages": installed_packages,
                "info": true,
            })),
        })
    }

    pub(crate) fn handle_remote_preview(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("rmt preview requires a remote name".to_owned()))?;
        let remote = self.load_registered_remote(name)?;
        if !looks_like_interemote_url(&remote.index_url) {
            return Err(CoreError::Operator(
                "rmt preview currently supports interemote git remotes; use `elda sync` for signed index remotes".to_owned(),
            ));
        }
        let preview =
            preview_interemote_with_protocols(&remote, &self.config.git.allowed_protocols)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "previewed {} package(s) from remote `{}`.",
                preview.included_count, remote.name,
            ),
            details: Some(json!({ "remote": remote, "preview": preview })),
        })
    }

    pub(crate) fn handle_remote_trust(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("rmt trust requires a remote name".to_owned()))?;
        let remote = self.load_registered_remote(name)?;
        let snapshot_record = self.snapshot_remote_record(&remote.name)?;
        let remote_kind = remote_kind(
            &remote.index_url,
            snapshot_record
                .as_ref()
                .map(|record| record.source.as_str()),
        );
        let trust_report = inspect_remote_trust(&self.repo_snapshot_path(), &remote)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("inspected trust state for remote `{}`.", remote.name),
            details: Some(json!({
                "remote": remote,
                "kind": remote_kind,
                "snapshot": snapshot_record,
                "trust_report": trust_report,
                "trust_command": true,
            })),
        })
    }

    pub(crate) fn handle_remote_set_enabled(
        &self,
        request: CommandRequest,
        enabled: bool,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request.operands.first().ok_or_else(|| {
            let command = if enabled { "enable" } else { "disable" };
            CoreError::Operator(format!("rmt {command} requires a remote name"))
        })?;
        let mut remote = self.load_registered_remote(name)?;
        remote.enabled = enabled;
        let remote = save_remote(&self.database.layout().remotes_dir, remote)?;
        let verb = if enabled { "enabled" } else { "disabled" };

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("{verb} remote `{}`.", remote.name),
            details: Some(json!({ "remote": remote, "enabled_changed": enabled })),
        })
    }

    pub(crate) fn handle_remote_set_priority(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request.operands.first().ok_or_else(|| {
            CoreError::Operator("rmt set-priority requires a remote name".to_owned())
        })?;
        let priority = request
            .operands
            .get(1)
            .ok_or_else(|| CoreError::Operator("rmt set-priority requires a priority".to_owned()))?
            .parse::<u32>()
            .map_err(|_| {
                CoreError::Operator(
                    "rmt set-priority priority must be an unsigned integer".to_owned(),
                )
            })?;
        let mut remote = self.load_registered_remote(name)?;
        remote.priority = priority;
        let remote = save_remote(&self.database.layout().remotes_dir, remote)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("set remote `{}` priority to {priority}.", remote.name),
            details: Some(json!({ "remote": remote, "priority_changed": priority })),
        })
    }

    pub(crate) fn handle_remote_remove(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("rmt rm requires a remote name".to_owned()))?
            .clone();
        let remote_before_remove = self.load_registered_remote(&name)?;
        let snapshot_record = self.snapshot_remote_record(&remote_before_remove.name)?;
        let indexed_packages = self.snapshot_package_names(&remote_before_remove.name)?;
        let installed_packages = self.installed_from_remote(&remote_before_remove.name)?;
        confirm_mutation(
            &request,
            &format!(
                "Remove remote `{}` ({} indexed, {} installed packages still reference it)?",
                remote_before_remove.name,
                indexed_packages.len(),
                installed_packages.len()
            ),
        )?;
        if request.dry_run {
            return Ok(CommandReport {
                area: "remote",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("would remove remote `{name}`."),
                details: Some(json!({
                    "remote": remote_before_remove,
                    "removed": false,
                    "snapshot": snapshot_record,
                    "indexed_packages": indexed_packages,
                    "installed_packages": installed_packages,
                })),
            });
        }
        let remote = remove_remote(&self.database.layout().remotes_dir, &name)?;

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("removed remote `{}`.", remote.name),
            details: Some(json!({
                "remote": remote,
                "removed": true,
                "snapshot": snapshot_record,
                "indexed_packages": indexed_packages,
                "installed_packages": installed_packages,
            })),
        })
    }

    fn load_registered_remote(&self, name: &str) -> Result<elda_repo::RemoteDocument, CoreError> {
        load_remote(&self.database.layout().remotes_dir, name)?
            .ok_or_else(|| CoreError::Operator(format!("remote `{name}` is not registered")))
    }

    fn snapshot_remote_record(
        &self,
        remote_name: &str,
    ) -> Result<Option<elda_repo::SyncedRemoteRecord>, CoreError> {
        match load_snapshot(&self.repo_snapshot_path()) {
            Ok(snapshot) => Ok(snapshot
                .remotes
                .into_iter()
                .find(|remote| remote.name == remote_name)),
            Err(elda_repo::RepoError::SnapshotMissing) => Ok(None),
            Err(error) => Err(CoreError::from(error)),
        }
    }

    fn snapshot_package_names(&self, remote_name: &str) -> Result<Vec<String>, CoreError> {
        match load_snapshot(&self.repo_snapshot_path()) {
            Ok(snapshot) => {
                let mut names = snapshot
                    .packages
                    .into_iter()
                    .filter(|package| package.remote_name == remote_name)
                    .map(|package| package.pkgname)
                    .collect::<Vec<_>>();
                names.sort();
                names.dedup();
                Ok(names)
            }
            Err(elda_repo::RepoError::SnapshotMissing) => Ok(Vec::new()),
            Err(error) => Err(CoreError::from(error)),
        }
    }

    fn installed_from_remote(
        &self,
        remote_name: &str,
    ) -> Result<Vec<elda_db::InstalledPackageRecord>, CoreError> {
        let installed = self
            .database
            .list_installed_packages()?
            .into_iter()
            .filter(|package| package.remote_name.as_deref() == Some(remote_name))
            .collect::<Vec<_>>();
        Ok(installed)
    }
}

fn looks_like_interemote_url(url: &str) -> bool {
    let normalized = url.trim_end_matches('/');
    !(normalized.ends_with(".toml")
        || normalized.ends_with(".json")
        || normalized.ends_with(".idx"))
}

fn remote_kind(index_url: &str, synced_source: Option<&str>) -> &'static str {
    if synced_source == Some("interemote") || looks_like_interemote_url(index_url) {
        "interemote"
    } else {
        "index"
    }
}

fn request_has_flag(request: &CommandRequest, flag: &str) -> bool {
    request.operands.iter().any(|operand| operand == flag)
}
