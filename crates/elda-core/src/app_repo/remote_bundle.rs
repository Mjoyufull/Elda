use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::app::AppContext;
use crate::app_confirm::confirm_mutation;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_repo::{DEFAULT_REMOTE_CHANNEL, RemoteDocument, TrustMode, load_remote, save_remote};

#[derive(Debug, Deserialize)]
struct ClientBundleFile {
    #[serde(default)]
    remotes: Vec<BundleRemote>,
    remote: Option<BundleRemote>,
}

#[derive(Debug, Deserialize)]
struct BundleRemote {
    name: String,
    index_url: String,
    #[serde(default)]
    signature_url: Option<String>,
    #[serde(default)]
    metadata_url: Option<String>,
    #[serde(default)]
    packages_url: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    trusted_keys: Vec<String>,
    #[serde(default)]
    trust: Option<String>,
}

impl AppContext {
    pub(crate) fn handle_remote_add_from_bundle(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let bundle_path = request
            .operands
            .first()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| {
                CoreError::Operator("`rmt add-from-bundle` requires a bundle file path".to_owned())
            })?;
        let remotes = parse_client_bundle(Path::new(bundle_path))?;
        if remotes.is_empty() {
            return Err(CoreError::Operator(
                "client bundle did not contain any remotes".to_owned(),
            ));
        }

        let replace = request.operands.iter().any(|value| value == "--replace");
        confirm_mutation(
            &request,
            &format!("Register {} remote(s) from bundle?", remotes.len()),
        )?;
        if request.dry_run {
            return Ok(CommandReport {
                area: "remote",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("would register {} remote(s) from bundle.", remotes.len()),
                details: Some(
                    serde_json::json!({ "remotes": remotes.iter().map(|remote| &remote.name).collect::<Vec<_>>() }),
                ),
            });
        }

        let remotes_dir = self.database.layout().remotes_dir.clone();
        let mut added = Vec::new();
        for remote in remotes {
            if load_remote(&remotes_dir, &remote.name)?.is_some() && !replace {
                return Err(CoreError::Operator(format!(
                    "remote `{}` already exists; pass --replace to overwrite",
                    remote.name
                )));
            }
            let document = RemoteDocument {
                name: remote.name.clone(),
                index_url: remote.index_url,
                signature_url: remote.signature_url,
                metadata_url: remote.metadata_url,
                packages_url: remote.packages_url,
                channel: remote
                    .channel
                    .unwrap_or_else(|| DEFAULT_REMOTE_CHANNEL.to_owned()),
                trust: parse_bundle_trust(remote.trust.as_deref())?,
                trusted_keys: remote.trusted_keys,
                priority: 100,
                enabled: true,
                allow_stale: false,
                exclude: Vec::new(),
            };
            save_remote(&remotes_dir, document)?;
            added.push(remote.name);
        }

        Ok(CommandReport {
            area: "remote",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("registered {} remote(s) from bundle.", added.len()),
            details: Some(serde_json::json!({ "remotes": added })),
        })
    }
}

fn parse_client_bundle(path: &Path) -> Result<Vec<BundleRemote>, CoreError> {
    let content = fs::read_to_string(path)?;
    if let Ok(value) = serde_json::from_str::<Value>(&content) {
        if let Some(remotes) = value.get("remotes").and_then(Value::as_array) {
            return remotes
                .iter()
                .map(|entry| serde_json::from_value(entry.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| CoreError::Operator(format!("invalid bundle remote: {error}")));
        }
        if value.get("remote").is_some() || value.get("remotes").is_some() {
            let bundle: ClientBundleFile = serde_json::from_value(value)?;
            return bundle_into_remotes(bundle);
        }
    }

    let bundle: ClientBundleFile = toml::from_str(&content).map_err(|error| {
        CoreError::Operator(format!(
            "invalid client bundle `{}`: {error}",
            path.display()
        ))
    })?;
    bundle_into_remotes(bundle)
}

fn bundle_into_remotes(bundle: ClientBundleFile) -> Result<Vec<BundleRemote>, CoreError> {
    let mut remotes = bundle.remotes;
    if let Some(single) = bundle.remote {
        remotes.push(single);
    }
    Ok(remotes)
}

fn parse_bundle_trust(value: Option<&str>) -> Result<TrustMode, CoreError> {
    match value.unwrap_or("tofu") {
        "tofu" => Ok(TrustMode::Tofu),
        "pinned" => Ok(TrustMode::Pinned),
        "insecure" => Ok(TrustMode::Insecure),
        other => Err(CoreError::Operator(format!(
            "invalid bundle trust mode `{other}`; expected tofu, pinned, or insecure"
        ))),
    }
}
