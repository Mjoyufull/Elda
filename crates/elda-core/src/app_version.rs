use crate::app::AppContext;
use crate::error::CoreError;
use crate::version::{cli_version_line, version_details};
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_types::OutputMode;

impl AppContext {
    pub(crate) fn handle_version(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let details = version_details();
        Ok(CommandReport {
            area: "version",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: cli_version_line(),
            details: Some(details),
        })
    }
}

pub(crate) fn render_version_report(report: &CommandReport) -> Option<String> {
    if report.area != "version" {
        return None;
    }
    match report.output_mode {
        OutputMode::Json => None,
        _ => Some(crate::version::version_details_human_lines().join("\n")),
    }
}
