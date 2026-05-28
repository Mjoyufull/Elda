mod diff;
mod finalize;
mod plan;
mod promote;
mod run;
mod sign;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest};

pub(crate) use plan::publish_plan_for_targets;

pub(crate) fn handle_publish_namespace(
    app: &AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    match request.command_path.as_slice() {
        [namespace, command] if namespace == "publish" && command == "plan" => {
            app.handle_publish_plan(request)
        }
        [namespace, command] if namespace == "publish" && command == "run" => {
            app.handle_publish_run(request)
        }
        [namespace, command] if namespace == "publish" && command == "finalize" => {
            app.handle_publish_finalize(request)
        }
        [namespace, command] if namespace == "publish" && command == "diff" => {
            app.handle_publish_diff(request)
        }
        [namespace, command] if namespace == "publish" && command == "promote" => {
            app.handle_publish_promote(request)
        }
        [namespace, command] if namespace == "publish" && command == "sign" => {
            app.handle_publish_sign(request)
        }
        _ => Err(CoreError::Operator(
            "unsupported publish request".to_owned(),
        )),
    }
}
