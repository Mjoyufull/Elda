use std::fs;

use serde_json::json;

use crate::app::AppContext;
use crate::app_ci::CiWorkspacePaths;
use crate::app_ci::workspace::{sign_bytes, signing_key, write_signature_envelope};
use crate::app_publish::plan::parse_flag_value;
use crate::error::CoreError;
use crate::host_config::load_host_profile;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_publish_sign(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let channel =
            parse_flag_value(&request, "--channel").unwrap_or_else(|| "stable".to_owned());
        let workspace = CiWorkspacePaths::for_channel(self.database.layout(), &channel);
        let index_path = if workspace.index_zst_path().is_file() {
            workspace.index_zst_path()
        } else if workspace.index_path.is_file() {
            workspace.index_path.clone()
        } else {
            return Err(CoreError::Operator(format!(
                "no index to sign for channel `{channel}`"
            )));
        };

        let key_path = parse_flag_value(&request, "--key")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                load_host_profile(
                    &self.database.layout().root_dir,
                    parse_flag_value(&request, "--profile").as_deref(),
                )
                .ok()
                .and_then(|profile| profile.signing_key_path())
            })
            .unwrap_or(workspace.signing_key_path.clone());

        let signing_key = signing_key(&key_path)?;
        let bytes = fs::read(&index_path)?;
        let signature = sign_bytes(&signing_key, &bytes);
        write_signature_envelope(&signing_key, &workspace.signature_path, &signature)?;

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "re-signed `{}` for channel `{channel}`.",
                index_path.display()
            ),
            details: Some(json!({
                "channel": channel,
                "index_path": index_path,
                "signature_path": workspace.signature_path,
                "signing_key": key_path,
            })),
        })
    }
}
