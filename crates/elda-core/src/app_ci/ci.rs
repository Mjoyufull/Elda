use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest};

pub(crate) fn handle_ci_namespace(
    app: &AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    match request.command_path.as_slice() {
        [namespace, command] if namespace == "ci" && command == "sub" => {
            app.handle_ci_submit(request, false, None)
        }
        [namespace, command] if namespace == "ci" && command == "run" => {
            if request.operands.is_empty() {
                app.handle_ci_scheduler_run(request)
            } else {
                app.handle_ci_submit(request, true, None)
            }
        }
        [namespace, command] if namespace == "ci" && command == "status" => {
            app.handle_ci_status(request)
        }
        [namespace, command] if namespace == "ci" && command == "pr" => app.handle_ci_pr(request),
        [namespace, command] if namespace == "ci" && command == "retry" => {
            app.handle_ci_retry(request)
        }
        [namespace, command] if namespace == "ci" && command == "logs" => {
            app.handle_ci_logs(request)
        }
        [namespace, batch, command]
            if namespace == "ci" && batch == "batch" && command == "new" =>
        {
            app.handle_ci_batch_new(request)
        }
        [namespace, batch, command]
            if namespace == "ci" && batch == "batch" && command == "add" =>
        {
            app.handle_ci_batch_add(request)
        }
        [namespace, batch, command]
            if namespace == "ci" && batch == "batch" && command == "push" =>
        {
            app.handle_ci_batch_push(request)
        }
        _ => Err(CoreError::Operator("unsupported ci request".to_owned())),
    }
}

impl AppContext {
    pub(crate) fn handle_daemon_run(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let refresh = self.handle_daemon_refresh(request.clone())?;
        Ok(CommandReport {
            area: "daemon",
            status: refresh.status,
            exit_status: refresh.exit_status,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "ran one foreground daemon refresh pass in the current slice.".to_owned(),
            details: refresh.details,
        })
    }
}
