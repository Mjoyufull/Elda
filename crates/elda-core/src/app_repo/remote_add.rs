use std::fs;

use crate::CommandRequest;
use crate::app::AppContext;
use crate::error::CoreError;
use elda_repo::{DEFAULT_REMOTE_CHANNEL, RemoteDocument, TrustMode};

impl AppContext {
    pub(super) fn parse_remote_add_request(
        &self,
        request: &CommandRequest,
        input: &str,
    ) -> Result<RemoteDocument, CoreError> {
        let (name, index_url) = parse_named_remote_input(input)?;
        let mut trust = TrustMode::Tofu;
        let mut trusted_keys = Vec::new();
        let mut trusted_key_file = None::<String>;
        let mut signature_url = None::<String>;
        let mut metadata_url = None::<String>;
        let mut packages_url = None::<String>;
        let mut channel = DEFAULT_REMOTE_CHANNEL.to_owned();
        let mut allow_stale = false;
        let mut priority = 100_u32;
        let mut operands = request.operands.iter().skip(1);

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--priority" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--priority` requires an unsigned integer".to_owned())
                    })?;
                    priority = value.parse::<u32>().map_err(|_| {
                        CoreError::Operator(format!(
                            "invalid remote priority `{value}`; expected an unsigned integer"
                        ))
                    })?;
                }
                "--trust" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator(
                            "`--trust` requires `tofu`, `pinned`, or `insecure`".to_owned(),
                        )
                    })?;
                    trust = parse_trust_mode(value)?;
                }
                "--trusted-key" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator(
                            "`--trusted-key` requires one key id or fingerprint".to_owned(),
                        )
                    })?;
                    trusted_keys.push(value.clone());
                }
                "--trusted-key-file" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator(
                            "`--trusted-key-file` requires a filesystem path".to_owned(),
                        )
                    })?;
                    trusted_key_file = Some(value.clone());
                }
                "--signature-url" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--signature-url` requires a URL".to_owned())
                    })?;
                    signature_url = Some(value.clone());
                }
                "--metadata-url" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--metadata-url` requires a URL".to_owned())
                    })?;
                    metadata_url = Some(value.clone());
                }
                "--packages-url" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--packages-url` requires a URL".to_owned())
                    })?;
                    packages_url = Some(value.clone());
                }
                "--channel" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--channel` requires a channel name".to_owned())
                    })?;
                    let parsed = value.trim();
                    if parsed.is_empty() {
                        return Err(CoreError::Operator(
                            "`--channel` requires a non-empty channel name".to_owned(),
                        ));
                    }
                    channel = parsed.to_owned();
                }
                "--allow-stale" => allow_stale = true,
                other => {
                    return Err(CoreError::Operator(format!(
                        "unrecognized `rmt add` operand `{other}`"
                    )));
                }
            }
        }

        if let Some(path) = trusted_key_file {
            trusted_keys.extend(read_trusted_keys_file(&path)?);
        }
        if trust == TrustMode::Pinned && trusted_keys.is_empty() {
            return Err(CoreError::Operator(
                "pinned remotes require at least one `--trusted-key` or `--trusted-key-file` entry"
                    .to_owned(),
            ));
        }
        if trust == TrustMode::Insecure && !trusted_keys.is_empty() {
            return Err(CoreError::Operator(
                "insecure remotes must not carry pinned trusted keys".to_owned(),
            ));
        }

        Ok(RemoteDocument {
            name,
            index_url,
            channel,
            packages_url,
            metadata_url,
            signature_url,
            enabled: true,
            trust,
            trusted_keys,
            allow_stale,
            priority,
        })
    }
}

fn parse_named_remote_input(input: &str) -> Result<(String, String), CoreError> {
    if let Some((name, url)) = input.split_once('=') {
        let name = sanitize_remote_name(name);
        if name.is_empty() {
            return Err(CoreError::Operator(
                "remote name must not be empty before `=`".to_owned(),
            ));
        }
        if url.trim().is_empty() {
            return Err(CoreError::Operator(
                "remote url must not be empty after `=`".to_owned(),
            ));
        }
        return Ok((name, url.trim().to_owned()));
    }

    if !looks_like_remote_url(input) {
        return Err(CoreError::Operator(
            "rmt add requires `<name>=<url>` or a bare URL".to_owned(),
        ));
    }

    Ok((derive_remote_name(input), input.trim().to_owned()))
}

fn parse_trust_mode(value: &str) -> Result<TrustMode, CoreError> {
    match value {
        "tofu" => Ok(TrustMode::Tofu),
        "pinned" => Ok(TrustMode::Pinned),
        "insecure" => Ok(TrustMode::Insecure),
        other => Err(CoreError::Operator(format!(
            "unsupported trust mode `{other}`; expected `tofu`, `pinned`, or `insecure`"
        ))),
    }
}

fn read_trusted_keys_file(path: &str) -> Result<Vec<String>, CoreError> {
    let content = fs::read_to_string(path)?;
    let keys = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    if keys.is_empty() {
        return Err(CoreError::Operator(format!(
            "trusted key file `{path}` did not contain any usable keys"
        )));
    }

    Ok(keys)
}

fn looks_like_remote_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://")
}

fn derive_remote_name(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let tail = trimmed.rsplit('/').next().unwrap_or("remote");
    let tail = tail
        .trim_end_matches(".json")
        .trim_end_matches(".toml")
        .trim_end_matches(".idx");
    let sanitized = sanitize_remote_name(tail);

    if sanitized.is_empty() {
        "remote".to_owned()
    } else {
        sanitized
    }
}

fn sanitize_remote_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}
