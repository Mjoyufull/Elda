use super::*;

use crate::app::ParsedUpgradeRequest;

impl AppContext {
    pub(super) fn parse_upgrade_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedUpgradeRequest, CoreError> {
        let mut targets = Vec::new();
        let mut refresh_weak_deps = self.config.defaults.refresh_weak_deps;

        for operand in &request.operands {
            match operand.as_str() {
                "--refresh-weak-deps" => refresh_weak_deps = true,
                _ => targets.push(operand.clone()),
            }
        }

        Ok(ParsedUpgradeRequest {
            targets,
            refresh_weak_deps,
        })
    }
}
