use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app_ci::CiWorkspacePaths;
use crate::app_ci::workspace::{
    read_json, sign_bytes, signing_key, write_json, write_signature_envelope,
};
use crate::error::CoreError;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IndexEnvelope {
    packages: Vec<IndexRecord>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct IndexRecord {
    pkgname: String,
    channel: String,
    asset_url: String,
    sha256: String,
    size: u64,
    payload_sig: String,
    sbom_url: String,
    attestation_url: String,
    source_kind: String,
    source_ref: String,
    repo_commit: Option<String>,
    variant_id: Option<String>,
    pkg_lua: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FinalizeReport {
    pub(crate) index_path: std::path::PathBuf,
    pub(crate) index_zst_path: std::path::PathBuf,
    pub(crate) signature_path: std::path::PathBuf,
    pub(crate) rewritten: usize,
}

pub(crate) fn finalize_published_index(
    workspace: &CiWorkspacePaths,
    base_url: &str,
    channel_subpath: Option<&str>,
    signing_key_path: Option<&Path>,
) -> Result<FinalizeReport, CoreError> {
    if !workspace.index_path.is_file() {
        return Err(CoreError::Operator(format!(
            "publish index `{}` does not exist; run `elda publish run` first",
            workspace.index_path.display()
        )));
    }

    let key_path = signing_key_path.unwrap_or(&workspace.signing_key_path);
    let signing_key = signing_key(key_path)?;

    let mut envelope: IndexEnvelope = read_json(&workspace.index_path)?;
    let prefix = build_url_prefix(base_url, channel_subpath);
    let mut rewritten = 0usize;

    for record in &mut envelope.packages {
        if rewrite_url_field(&mut record.asset_url, &prefix, &workspace.artifacts_dir)? {
            rewritten += 1;
        }
        rewrite_url_field(&mut record.sbom_url, &prefix, &workspace.artifacts_dir)?;
        rewrite_url_field(
            &mut record.attestation_url,
            &prefix,
            &workspace.artifacts_dir,
        )?;
    }

    write_json(&workspace.index_path, &envelope)?;
    let index_bytes = fs::read(&workspace.index_path)?;
    let zst_path = workspace.index_zst_path();
    write_zst(&zst_path, &index_bytes)?;
    let signature = sign_bytes(&signing_key, &fs::read(&zst_path)?);
    write_signature_envelope(&signing_key, &workspace.signature_path, &signature)?;

    Ok(FinalizeReport {
        index_path: workspace.index_path.clone(),
        index_zst_path: zst_path,
        signature_path: workspace.signature_path.clone(),
        rewritten,
    })
}

fn build_url_prefix(base_url: &str, channel_subpath: Option<&str>) -> String {
    let base = base_url.trim_end_matches('/');
    match channel_subpath.filter(|value| !value.is_empty()) {
        Some(subpath) => format!("{base}/{subpath}"),
        None => base.to_owned(),
    }
}

fn rewrite_url_field(
    url: &mut String,
    prefix: &str,
    artifacts_dir: &Path,
) -> Result<bool, CoreError> {
    let Some(file_path) = url.strip_prefix("file://") else {
        return Ok(false);
    };
    let path = Path::new(file_path);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CoreError::Operator(format!("could not derive file name from `{url}`")))?;
    let relative = path
        .strip_prefix(artifacts_dir)
        .ok()
        .and_then(|rest| rest.to_str())
        .unwrap_or(file_name);
    *url = format!("{prefix}/artifacts/{relative}");
    Ok(true)
}

fn write_zst(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut encoder = zstd::Encoder::new(fs::File::create(path)?, 19)?;
    encoder.write_all(bytes)?;
    encoder.finish()?;
    Ok(())
}

use crate::app::AppContext;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::plan::parse_flag_value;

impl AppContext {
    pub(crate) fn handle_publish_finalize(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let channel =
            parse_flag_value(&request, "--channel").unwrap_or_else(|| "stable".to_owned());
        let base_url = parse_flag_value(&request, "--base-url")
            .or_else(|| {
                crate::host_config::load_host_profile(
                    &self.database.layout().root_dir,
                    parse_flag_value(&request, "--profile").as_deref(),
                )
                .ok()
                .and_then(|profile| profile.resolve_base_url())
            })
            .ok_or_else(|| {
                CoreError::Operator(
                "`publish finalize` requires `--base-url <https://...>` or host profile base_url"
                    .to_owned(),
            )
            })?;

        let profile = crate::host_config::load_host_profile(
            &self.database.layout().root_dir,
            parse_flag_value(&request, "--profile").as_deref(),
        )
        .ok();
        let subpath = profile
            .as_ref()
            .and_then(|value| value.channel_index_subpath(&channel));
        let workspace = CiWorkspacePaths::for_channel(self.database.layout(), &channel);
        let signing_key = profile.as_ref().and_then(|value| value.signing_key_path());
        let report = finalize_published_index(
            &workspace,
            &base_url,
            subpath.as_deref(),
            signing_key.as_deref(),
        )?;

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "finalized index for channel `{channel}` with base URL `{base_url}` ({} row(s) rewritten).",
                report.rewritten
            ),
            details: Some(serde_json::json!({
                "channel": channel,
                "base_url": base_url,
                "index_path": report.index_path,
                "index_zst_path": report.index_zst_path,
                "signature_path": report.signature_path,
                "rewritten": report.rewritten,
            })),
        })
    }
}

pub(crate) fn read_index_packages(path: &Path) -> Result<Vec<Value>, CoreError> {
    let envelope: IndexEnvelope = if path.extension().and_then(|ext| ext.to_str()) == Some("zst") {
        let bytes = fs::read(path)?;
        let decoded = zstd::decode_all(std::io::Cursor::new(bytes))?;
        serde_json::from_slice(&decoded)?
    } else {
        read_json(path)?
    };
    Ok(envelope
        .packages
        .into_iter()
        .map(|record| serde_json::to_value(record).unwrap_or(Value::Null))
        .collect())
}
