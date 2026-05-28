use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::{BinarySourceVerification, BuildError};

pub(super) fn verify_downloaded_payload(
    downloaded_path: &Path,
    verification: &BinarySourceVerification,
) -> Result<(), BuildError> {
    let encoded_signature = verification.payload_signature.as_deref().ok_or_else(|| {
        BuildError::Invalid(format!(
            "secure remote `{}` is missing `payload_sig` for this package",
            verification.remote_name
        ))
    })?;
    if verification.trusted_public_keys.is_empty() {
        return Err(BuildError::Invalid(format!(
            "secure remote `{}` has no trusted public keys for payload verification",
            verification.remote_name
        )));
    }

    let payload = fs::read(downloaded_path)?;
    let signature = decode_signature(encoded_signature)?;

    for encoded_key in &verification.trusted_public_keys {
        let public_key = decode_public_key(encoded_key)?;
        if public_key.verify(&payload, &signature).is_ok() {
            return Ok(());
        }
    }

    Err(BuildError::Invalid(format!(
        "payload signature verification failed for secure remote `{}`",
        verification.remote_name
    )))
}

fn decode_signature(encoded: &str) -> Result<Signature, BuildError> {
    let bytes = STANDARD.decode(encoded).map_err(|error| {
        BuildError::Invalid(format!("invalid payload signature base64: {error}"))
    })?;
    let array = <[u8; 64]>::try_from(bytes.as_slice())
        .map_err(|_| BuildError::Invalid("payload signature must be 64 bytes".to_owned()))?;

    Ok(Signature::from_bytes(&array))
}

fn decode_public_key(encoded: &str) -> Result<VerifyingKey, BuildError> {
    let bytes = STANDARD.decode(encoded).map_err(|error| {
        BuildError::Invalid(format!("invalid trusted public key base64: {error}"))
    })?;
    let array = <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| BuildError::Invalid("trusted public key must be 32 bytes".to_owned()))?;

    VerifyingKey::from_bytes(&array)
        .map_err(|error| BuildError::Invalid(format!("invalid trusted public key: {error}")))
}
