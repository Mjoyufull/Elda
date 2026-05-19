use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::RepoError;
use crate::model::{RemoteDocument, TrustMode, TrustedPublicKey};

use super::state::RemoteTrustState;

#[cfg(test)]
mod tests;

#[derive(Debug, Deserialize)]
pub(super) struct SignatureEnvelope {
    pub(super) key_id: Option<String>,
    pub(super) public_key: String,
    pub(super) signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerifiedKey {
    pub(super) selected_key: String,
}

#[derive(Debug, Deserialize)]
struct RemoteMetadataDocument {
    #[serde(default, alias = "trusted_public_keys")]
    trusted_keys: Vec<MetadataTrustedKey>,
    #[serde(default)]
    revoked_fingerprints: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataTrustedKey {
    key_id: Option<String>,
    public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VerifiedSignature {
    selected_key: String,
    fingerprint: String,
    encoded_public_key: String,
}

pub(super) fn parse_signature_envelope(content: &str) -> Result<SignatureEnvelope, RepoError> {
    parse_document(content)
}

pub(super) fn verify_remote_signature(
    remote: &RemoteDocument,
    trust_state: &mut RemoteTrustState,
    index_content: &str,
    signature: SignatureEnvelope,
    options: &super::sync::SyncOptions,
) -> Result<VerifiedKey, RepoError> {
    if remote.trust == TrustMode::Insecure {
        return Err(RepoError::Trust(format!(
            "remote `{}` is insecure and does not use signed verification",
            remote.name
        )));
    }

    let verified = verify_signed_content(index_content, signature)?;

    match remote.trust {
        TrustMode::Pinned => {
            verify_pinned_remote(remote, &verified.selected_key, &verified.fingerprint)?
        }
        TrustMode::Tofu => {
            if tofu_rotation_possible(trust_state)
                && verify_tofu_remote(remote, trust_state, &verified.fingerprint, options).is_err()
            {
                verify_tofu_rotation(remote, trust_state, &verified.fingerprint, options)?;
            } else {
                verify_tofu_remote(remote, trust_state, &verified.fingerprint, options)?;
            }
        }
        TrustMode::Insecure => unreachable!(),
    }
    trust_state.remember_verified_key(
        &verified.selected_key,
        &verified.fingerprint,
        &verified.encoded_public_key,
    );

    Ok(VerifiedKey {
        selected_key: verified.selected_key,
    })
}

fn decode_public_key(encoded: &str) -> Result<VerifyingKey, RepoError> {
    let bytes = decode_base64(encoded, "public key")?;
    let array = <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| RepoError::Trust("ed25519 public key must be 32 bytes".to_owned()))?;
    VerifyingKey::from_bytes(&array)
        .map_err(|error| RepoError::Trust(format!("invalid ed25519 public key: {error}")))
}

fn decode_signature(encoded: &str) -> Result<Signature, RepoError> {
    let bytes = decode_base64(encoded, "signature")?;
    let array = <[u8; 64]>::try_from(bytes.as_slice())
        .map_err(|_| RepoError::Trust("ed25519 signature must be 64 bytes".to_owned()))?;
    Ok(Signature::from_bytes(&array))
}

fn decode_base64(encoded: &str, label: &str) -> Result<Vec<u8>, RepoError> {
    STANDARD
        .decode(encoded)
        .map_err(|error| RepoError::Trust(format!("invalid base64 {label}: {error}")))
}

fn verify_signed_content(
    content: &str,
    signature: SignatureEnvelope,
) -> Result<VerifiedSignature, RepoError> {
    let public_key = decode_public_key(&signature.public_key)?;
    let detached_signature = decode_signature(&signature.signature)?;
    public_key
        .verify(content.as_bytes(), &detached_signature)
        .map_err(|error| RepoError::Trust(format!("invalid detached signature: {error}")))?;

    let fingerprint = fingerprint_for_key(public_key.as_bytes());
    let selected_key = signature
        .key_id
        .unwrap_or_else(|| format!("sha256:{fingerprint}"));

    Ok(VerifiedSignature {
        selected_key,
        fingerprint,
        encoded_public_key: STANDARD.encode(public_key.as_bytes()),
    })
}

fn fingerprint_for_key(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn verify_pinned_remote(
    remote: &RemoteDocument,
    key_id: &str,
    fingerprint: &str,
) -> Result<(), RepoError> {
    if remote.trusted_keys.is_empty() {
        return Err(RepoError::Trust(format!(
            "pinned remote `{}` is missing configured `trusted_keys`",
            remote.name
        )));
    }
    if remote
        .trusted_keys
        .iter()
        .any(|trusted| trusted == key_id || trusted == fingerprint)
    {
        return Ok(());
    }

    Err(RepoError::Trust(format!(
        "remote `{}` signature key `{key_id}` / sha256:{fingerprint} is not in the pinned trust set",
        remote.name
    )))
}

fn verify_tofu_remote(
    remote: &RemoteDocument,
    trust_state: &mut RemoteTrustState,
    fingerprint: &str,
    options: &super::sync::SyncOptions,
) -> Result<(), RepoError> {
    if trust_state.trusted_fingerprints.is_empty() && trust_state.trusted_public_keys.is_empty() {
        if options.allow_initial_tofu {
            return Ok(());
        }

        return Err(RepoError::Trust(format!(
            "remote `{}` requires explicit trust bootstrap before unattended sync; use a pinned key or run an interactive sync first",
            remote.name
        )));
    }
    if trust_state.has_trusted_fingerprint(fingerprint) {
        return Ok(());
    }

    let expected = if trust_state.trusted_fingerprints.is_empty() {
        trust_state
            .trusted_public_keys
            .iter()
            .map(|trusted| trusted.fingerprint.clone())
            .collect::<Vec<_>>()
    } else {
        trust_state.trusted_fingerprints.clone()
    };

    Err(RepoError::Trust(format!(
        "remote signature key changed without a trusted rotation path; expected one of `{}`, got `sha256:{fingerprint}`",
        expected.join("`, `")
    )))
}

fn verify_tofu_rotation(
    remote: &RemoteDocument,
    trust_state: &mut RemoteTrustState,
    next_fingerprint: &str,
    options: &super::sync::SyncOptions,
) -> Result<(), RepoError> {
    let metadata_url = remote.metadata_url.as_ref().ok_or_else(|| {
        RepoError::Trust(format!(
            "remote signature key changed without a trusted rotation path; remote `{}` does not define `metadata_url`",
            remote.name
        ))
    })?;
    let metadata_content = super::fetch::read_location_text(metadata_url)?;
    let metadata_signature_content =
        super::fetch::read_location_text(&format!("{metadata_url}.sig"))?;
    let metadata_signature = parse_signature_envelope(&metadata_signature_content)?;
    let metadata_verified = verify_signed_content(&metadata_content, metadata_signature)?;

    if !trust_state.has_trusted_fingerprint(&metadata_verified.fingerprint) {
        return Err(RepoError::Trust(format!(
            "remote rotation metadata for `{}` is not signed by an already trusted key",
            remote.name
        )));
    }

    let metadata = parse_remote_metadata_document(&metadata_content)?;
    let rotated_keys = trusted_public_keys_from_metadata(&metadata)?;
    let revoked_fingerprints = metadata
        .revoked_fingerprints
        .iter()
        .map(|fingerprint| normalize_fingerprint_reference(fingerprint))
        .collect::<Vec<_>>();
    let trusted_public_keys = rotated_keys
        .into_iter()
        .filter(|trusted| !revoked_fingerprints.contains(&trusted.fingerprint))
        .collect::<Vec<_>>();

    if !trusted_public_keys
        .iter()
        .any(|trusted| trusted.fingerprint == next_fingerprint)
    {
        return Err(RepoError::Trust(format!(
            "remote rotation metadata for `{}` does not authorize the new signing key `sha256:{next_fingerprint}`",
            remote.name
        )));
    }

    if !rotation_is_accepted(options, &remote.name) {
        return Err(RepoError::Trust(format!(
            "remote `{}` presented rotated signing key `sha256:{next_fingerprint}` authorized by signed metadata but still requires operator confirmation; rerun with `--accept-rotated-key {}`",
            remote.name, remote.name
        )));
    }

    trust_state.trusted_fingerprints = trusted_public_keys
        .iter()
        .map(|trusted| trusted.fingerprint.clone())
        .collect();
    trust_state.trusted_public_keys = trusted_public_keys;

    Ok(())
}

fn rotation_is_accepted(options: &super::sync::SyncOptions, remote_name: &str) -> bool {
    options
        .accept_rotated_keys
        .iter()
        .any(|accepted| accepted == remote_name)
}

fn tofu_rotation_possible(trust_state: &RemoteTrustState) -> bool {
    !trust_state.trusted_fingerprints.is_empty() || !trust_state.trusted_public_keys.is_empty()
}

fn parse_remote_metadata_document(content: &str) -> Result<RemoteMetadataDocument, RepoError> {
    parse_document(content)
}

fn trusted_public_keys_from_metadata(
    metadata: &RemoteMetadataDocument,
) -> Result<Vec<TrustedPublicKey>, RepoError> {
    metadata
        .trusted_keys
        .iter()
        .map(|trusted| {
            let public_key = decode_public_key(&trusted.public_key)?;
            let fingerprint = fingerprint_for_key(public_key.as_bytes());
            Ok(TrustedPublicKey {
                key_id: trusted
                    .key_id
                    .clone()
                    .unwrap_or_else(|| format!("sha256:{fingerprint}")),
                fingerprint,
                public_key: STANDARD.encode(public_key.as_bytes()),
            })
        })
        .collect()
}

fn normalize_fingerprint_reference(value: &str) -> String {
    value
        .trim()
        .strip_prefix("sha256:")
        .unwrap_or(value.trim())
        .to_owned()
}

fn parse_document<T>(content: &str) -> Result<T, RepoError>
where
    T: for<'de> Deserialize<'de>,
{
    let trimmed = content.trim_start();
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && let Ok(document) = serde_json::from_str(trimmed)
    {
        return Ok(document);
    }

    toml::from_str(trimmed).map_err(RepoError::from)
}
