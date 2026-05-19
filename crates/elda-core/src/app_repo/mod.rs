mod info;
mod info_visibility;
mod remote;
mod remote_add;
mod remote_bundle;
mod search;

use std::path::PathBuf;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use elda_repo::{CacheDocument, SyncOptions, list_caches, load_snapshot, save_cache, sync_remotes};

impl AppContext {
    pub(crate) fn repo_snapshot_path(&self) -> PathBuf {
        self.database.layout().db_dir.join("repo-snapshot.json")
    }

    pub(crate) fn handle_cache_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let cache = save_cache(
            &self.database.layout().caches_dir,
            self.parse_cache_add_request(&request)?,
        )?;

        Ok(CommandReport {
            area: "cache",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("registered cache `{}`.", cache.name),
            details: Some(json!({ "cache": cache })),
        })
    }

    pub(crate) fn handle_cache_list(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let mut caches = list_caches(&self.database.layout().caches_dir)?;
        caches.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.name.cmp(&right.name))
        });
        let policy = self.cache_policy_report()?;

        Ok(CommandReport {
            area: "cache",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} configured cache(s).", caches.len()),
            details: Some(json!({
                "caches": caches,
                "policy": policy,
            })),
        })
    }

    pub(crate) fn handle_sync(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        use crate::OutputMode;
        use crate::progress::{FrameOutcome, ProgressEvent};

        self.database.bootstrap()?;
        let stream_sync = !request.no_stream && request.output_mode == OutputMode::Human;
        let frame = stream_sync.then(|| self.next_frame_id());
        if let Some(frame) = frame {
            self.progress_sink().emit(ProgressEvent::FrameStart {
                frame,
                title: "Sync remotes".to_owned(),
                subject: None,
            });
        }

        let mut sync_options = self.sync_options(&request);
        if let Some(frame) = frame {
            let sink = std::sync::Arc::clone(&self.progress);
            sync_options.progress = Some(std::sync::Arc::new(move |event| {
                use elda_repo::RemoteSyncEvent;
                match event {
                    RemoteSyncEvent::RemoteStart { name } => {
                        sink.emit(ProgressEvent::StepStarted {
                            frame,
                            step: "remote",
                            label: name,
                            detail: Some("fetching remote index".to_owned()),
                            live_spinner: true,
                        });
                    }
                    RemoteSyncEvent::RemoteDone {
                        name,
                        package_count,
                        stale,
                        issue,
                    } => {
                        let summary = issue.unwrap_or_else(|| {
                            if stale {
                                format!("{package_count} packages (stale snapshot)")
                            } else {
                                format!("{package_count} packages")
                            }
                        });
                        sink.emit(ProgressEvent::StepDone {
                            frame,
                            step: "remote",
                            label: name,
                            summary: Some(summary),
                        });
                    }
                }
            }));
        }

        let sync = sync_remotes(
            &self.database.layout().remotes_dir,
            &self.repo_snapshot_path(),
            sync_options,
        )?;

        if let Some(frame) = frame {
            self.progress_sink().emit(ProgressEvent::FrameEnd {
                frame,
                outcome: if sync.failed_remote_count == 0 {
                    FrameOutcome::Ok
                } else {
                    FrameOutcome::Blocked
                },
                summary: Some(format!(
                    "{} remote(s), {} package(s)",
                    sync.remote_count, sync.package_count
                )),
            });
        }

        Ok(CommandReport {
            area: "sync",
            status: sync_status(&sync),
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "synced {} package(s) from {} remote(s).",
                sync.package_count, sync.remote_count,
            ),
            details: Some(json!({ "sync": sync })),
        })
    }

    pub(crate) fn handle_daemon_status(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let snapshot_path = self.repo_snapshot_path();
        let snapshot = load_snapshot(&snapshot_path).ok();

        Ok(CommandReport {
            area: "daemon",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "reported current daemon-facing snapshot state.".to_owned(),
            details: Some(json!({
                "snapshot_present": snapshot_path.exists(),
                "snapshot_path": snapshot_path,
                "snapshot": snapshot,
            })),
        })
    }

    pub(crate) fn handle_daemon_refresh(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let sync = sync_remotes(
            &self.database.layout().remotes_dir,
            &self.repo_snapshot_path(),
            self.sync_options(&request),
        )?;

        Ok(CommandReport {
            area: "daemon",
            status: sync_status(&sync),
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("refreshed {} remote snapshot(s).", sync.remote_count),
            details: Some(json!({ "sync": sync })),
        })
    }
}

impl AppContext {
    pub(crate) fn sync_options(&self, request: &CommandRequest) -> SyncOptions {
        SyncOptions {
            offline: request.offline,
            allow_initial_tofu: request.output_mode == OutputMode::Human,
            accept_rotated_keys: request.accept_rotated_keys.clone(),
            target_remotes: sync_target_remotes(request),
            allowed_git_protocols: self.config.git.allowed_protocols.clone(),
            progress: None,
        }
    }

    fn parse_cache_add_request(
        &self,
        request: &CommandRequest,
    ) -> Result<CacheDocument, CoreError> {
        let input = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("cache add requires `<name>=<url>`".to_owned()))?;
        let mut priority = 100_u32;
        let mut operands = request.operands.iter().skip(1);

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--priority" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--priority` requires an unsigned integer".to_owned())
                    })?;
                    priority = value.parse::<u32>().map_err(|_| {
                        CoreError::Operator(format!(
                            "invalid cache priority `{value}`; expected an unsigned integer"
                        ))
                    })?;
                }
                other => {
                    return Err(CoreError::Operator(format!(
                        "unrecognized `cache add` operand `{other}`"
                    )));
                }
            }
        }

        let (name, base_url) = parse_named_cache_input(input)?;
        Ok(CacheDocument {
            name,
            base_url,
            priority,
            enabled: true,
        })
    }
}

fn sync_target_remotes(request: &CommandRequest) -> Vec<String> {
    if request.command_path.as_slice() == ["sync"] {
        request.operands.clone()
    } else {
        Vec::new()
    }
}

fn parse_named_cache_input(input: &str) -> Result<(String, String), CoreError> {
    if let Some((name, url)) = input.split_once('=') {
        let name = sanitize_cache_name(name);
        if name.is_empty() {
            return Err(CoreError::Operator(
                "cache name must not be empty before `=`".to_owned(),
            ));
        }
        if url.trim().is_empty() {
            return Err(CoreError::Operator(
                "cache url must not be empty after `=`".to_owned(),
            ));
        }
        return Ok((name, url.trim().to_owned()));
    }

    if !looks_like_cache_url(input) {
        return Err(CoreError::Operator(
            "cache add requires `<name>=<url>` or a bare URL".to_owned(),
        ));
    }

    Ok((derive_cache_name(input), input.trim().to_owned()))
}

fn looks_like_cache_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://")
}

fn derive_cache_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let tail = trimmed.rsplit('/').next().unwrap_or("cache");
    let sanitized = sanitize_cache_name(tail);

    if sanitized.is_empty() {
        "cache".to_owned()
    } else {
        sanitized
    }
}

fn sanitize_cache_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn sync_status(sync: &elda_repo::SyncReport) -> &'static str {
    if sync.failed_remote_count == 0 {
        "ok"
    } else {
        "issues"
    }
}
