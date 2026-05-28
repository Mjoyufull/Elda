use std::path::Path;

use serde_json::{Value, json};

use crate::app::AppContext;
use crate::{CommandReport, CommandRequest, CoreError, ExitStatus};

impl AppContext {
    pub(crate) fn handle_appimage_inspect(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let path_arg = request.operands.first().ok_or_else(|| {
            CoreError::Operator("appimage inspect requires a path to an AppImage file".to_owned())
        })?;
        let path = Path::new(path_arg);
        let inspect = elda_appimage::inspect_appimage(path)
            .map_err(|err| CoreError::Operator(err.to_string()))?;

        let desktop_count = inspect.desktop_candidates.len();
        let summary = format!(
            "Type {} AppImage — SquashFS payload at byte {} (`{}`, {} `.desktop` entr{})",
            inspect.generation,
            inspect.squashfs_offset,
            path.display(),
            desktop_count,
            if desktop_count == 1 { "y" } else { "ies" },
        );

        let payload = serde_json::to_value(&inspect).unwrap_or(Value::Null);

        Ok(CommandReport {
            area: "appimage",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary,
            details: Some(json!({
                "appimage_inspect": payload,
            })),
        })
    }
}
