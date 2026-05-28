mod artifacts;
mod ci;
mod ci_batch;
mod ci_submission;
mod forge;
mod model;
mod publish;
mod publish_plan;
mod qa;
mod qa_support;
mod remote_push;
mod review;
mod scheduler;
mod store;
pub(crate) mod workspace;

pub(crate) use publish::publish_workspace;
pub(crate) use publish_plan::{PlannedCiPackage, PlannedCiWork, plan_ci_work};
pub(crate) use workspace::CiWorkspacePaths;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest};

impl AppContext {
    pub(crate) fn handle_ci_namespace(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        ci::handle_ci_namespace(self, request)
    }

    pub(crate) fn handle_forge_namespace(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        forge::handle_forge_namespace(self, request)
    }

    pub(crate) fn handle_qa_namespace(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        qa::handle_qa_namespace(self, request)
    }
}
