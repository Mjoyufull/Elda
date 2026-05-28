use std::fs;

use serde_json::json;

use crate::app::AppContext;
use crate::app_ci::CiWorkspacePaths;
use crate::app_ci::workspace::{copy_dir_recursive, remove_path_if_exists};
use crate::app_publish::finalize::read_index_packages;
use crate::app_publish::plan::parse_flag_value;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_publish_promote(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        if request.dry_run {
            return Ok(CommandReport {
                area: "publish",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: "dry-run promote would copy staging index and artifacts into stable."
                    .to_owned(),
                details: None,
            });
        }

        self.database.bootstrap()?;
        let from = parse_flag_value(&request, "--from").ok_or_else(|| {
            CoreError::Operator("`publish promote` requires `--from <channel>`".to_owned())
        })?;
        let to = parse_flag_value(&request, "--to").ok_or_else(|| {
            CoreError::Operator("`publish promote` requires `--to <channel>`".to_owned())
        })?;
        let layout = self.database.layout();
        let source = CiWorkspacePaths::for_channel(layout, &from);
        let destination = CiWorkspacePaths::for_channel(layout, &to);
        destination.ensure_exists()?;

        if source.published_dir.is_dir() {
            for entry in fs::read_dir(&source.published_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path == source.artifacts_dir {
                    continue;
                }
                let target = destination.published_dir.join(entry.file_name());
                remove_path_if_exists(&target)?;
                if path.is_dir() {
                    copy_dir_recursive(&path, &target)?;
                } else {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&path, &target)?;
                }
            }
        }

        if source.artifacts_dir.is_dir() {
            fs::create_dir_all(&destination.artifacts_dir)?;
            for entry in fs::read_dir(&source.artifacts_dir)? {
                let entry = entry?;
                let target = destination.artifacts_dir.join(entry.file_name());
                if entry.path().is_dir() {
                    copy_dir_recursive(&entry.path(), &target)?;
                } else if !target.exists() {
                    fs::copy(entry.path(), &target)?;
                }
            }
        }

        let index_path = if destination.index_zst_path().is_file() {
            destination.index_zst_path()
        } else {
            destination.index_path.clone()
        };
        let count = read_index_packages(&index_path)
            .map(|packages| packages.len())
            .unwrap_or(0);

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("promoted channel `{from}` into `{to}` ({count} index row(s))."),
            details: Some(json!({
                "from": from,
                "to": to,
                "published_dir": destination.published_dir,
            })),
        })
    }
}
