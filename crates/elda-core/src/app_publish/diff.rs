use std::collections::BTreeSet;

use serde_json::{Value, json};

use crate::app::AppContext;
use crate::app_ci::CiWorkspacePaths;
use crate::app_publish::finalize::read_index_packages;
use crate::app_publish::plan::{parse_flag_value, publish_targets};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_publish_diff(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let channel =
            parse_flag_value(&request, "--channel").unwrap_or_else(|| "stable".to_owned());
        let workspace = CiWorkspacePaths::for_channel(self.database.layout(), &channel);
        let current_path = if workspace.index_zst_path().is_file() {
            workspace.index_zst_path()
        } else {
            workspace.index_path.clone()
        };
        if !current_path.is_file() {
            return Err(CoreError::Operator(format!(
                "no index found for channel `{channel}` under {}",
                workspace.published_dir.display()
            )));
        }

        let previous_path = publish_targets(&request, &["--channel"])
            .first()
            .map(|value| std::path::PathBuf::from(value.as_str()));
        let current = read_index_packages(&current_path)?;
        let previous = match previous_path {
            Some(path) => read_index_packages(&path)?,
            None => Vec::new(),
        };

        let current_names = index_pkg_keys(&current);
        let previous_names = index_pkg_keys(&previous);
        let added: Vec<_> = current_names.difference(&previous_names).cloned().collect();
        let removed: Vec<_> = previous_names.difference(&current_names).cloned().collect();

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "index diff for channel `{channel}`: {} added, {} removed.",
                added.len(),
                removed.len()
            ),
            details: Some(json!({
                "channel": channel,
                "added": added,
                "removed": removed,
                "current_count": current.len(),
                "previous_count": previous.len(),
            })),
        })
    }
}

fn index_pkg_keys(records: &[Value]) -> BTreeSet<String> {
    records
        .iter()
        .filter_map(|record| {
            let name = record.get("pkgname")?.as_str()?;
            let arch = record.get("arch").and_then(Value::as_str).unwrap_or("any");
            Some(format!("{name}:{arch}"))
        })
        .collect()
}
