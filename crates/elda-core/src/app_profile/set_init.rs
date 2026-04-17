use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::empty_to;

impl AppContext {
    pub(crate) fn handle_profile_set_init(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let provider = parse_init_provider(&request)?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.init.clone();
        profile.init = provider.clone();
        let desired = profile.to_desired_profile(self.profile_state_base(&profile)?);

        if request.dry_run {
            return Ok(CommandReport {
                area: "profile",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "planned init-provider change from `{}` to `{}`.",
                    empty_to(previous.clone(), "unset".to_owned()),
                    provider
                ),
                details: Some(json!({
                    "plan": {
                        "kind": "profile-set-init",
                        "previous_init": empty_to(previous, "unset".to_owned()),
                        "next_init": provider,
                        "provider_reconciliation": {
                            "applied": false,
                            "actions": [],
                            "reason": "dry-run",
                        },
                        "pending_handler_transitions": [],
                        "required_activation_class": "none",
                    },
                })),
            });
        }

        self.write_profile_state(&desired)?;

        Ok(CommandReport {
            area: "profile",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("set the active init-provider family to `{provider}`."),
            details: Some(json!({
                "previous_init": empty_to(previous, "unset".to_owned()),
                "next_init": provider,
                "provider_reconciliation": {
                    "applied": true,
                    "actions": [],
                    "reason": "the current backend slice does not model provider-scoped service assets yet",
                },
                "pending_handler_transitions": [],
                "required_activation_class": "none",
            })),
        })
    }
}

fn parse_init_provider(request: &CommandRequest) -> Result<String, CoreError> {
    match request.operands.as_slice() {
        [provider] if !provider.trim().is_empty() => Ok(provider.clone()),
        [] => Err(CoreError::Operator(
            "pf set-init requires one init-provider value".to_owned(),
        )),
        [provider] => Err(CoreError::Operator(format!(
            "invalid empty init-provider `{provider}`"
        ))),
        [_, extra, ..] => Err(CoreError::Operator(format!(
            "unexpected `pf set-init` operand `{extra}`"
        ))),
    }
}
