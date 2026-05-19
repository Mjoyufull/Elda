use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::error::BuildError;

/// Verdict from a release-asset sidecar signature verification attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureVerdict {
    /// The sidecar signature was fetched and verified against a trusted key.
    Verified { key_index: usize },
    /// The recipe declared a signature field but no trust keys are configured.
    NoTrustKeys,
    /// The recipe did not declare a signature field.
    NoSignature,
}

/// Fetch a release sidecar and verify it against trusted public keys.
///
/// The sidecar URL is constructed by appending the `signature_filename`
/// to the asset URL's parent path. Supports:
/// - Raw Ed25519 base64 signatures (64 bytes decoded)
/// - Minisign format (untrusted comment + signature line)
///
/// Returns `Verified` on success, `NoTrustKeys` when keys are absent,
/// and `NoSignature` when no signature field is declared.
pub fn fetch_and_verify_release_sidecar(
    asset_url: &str,
    signature_filename: Option<&str>,
    payload_path: &Path,
    trusted_keys: &[String],
) -> Result<SignatureVerdict, BuildError> {
    let Some(sig_name) = signature_filename else {
        return Ok(SignatureVerdict::NoSignature);
    };

    if trusted_keys.is_empty() {
        return Ok(SignatureVerdict::NoTrustKeys);
    }

    let sidecar_url = derive_sidecar_url(asset_url, sig_name)?;
    let sidecar_bytes = fetch_sidecar(&sidecar_url)?;
    let signature = parse_sidecar_signature(&sidecar_bytes)?;
    let payload = std::fs::read(payload_path)?;

    for (index, encoded_key) in trusted_keys.iter().enumerate() {
        let public_key = decode_public_key(encoded_key)?;
        if public_key.verify(&payload, &signature).is_ok() {
            return Ok(SignatureVerdict::Verified { key_index: index });
        }
    }

    Err(BuildError::Invalid(format!(
        "release signature verification failed: none of {} trusted \
         key(s) match the sidecar at {sidecar_url}",
        trusted_keys.len()
    )))
}

/// Construct the sidecar URL from the asset URL and signature filename.
///
/// If the signature filename is an absolute URL (starts with http), use
/// it directly. Otherwise, join it against the asset URL's parent path.
fn derive_sidecar_url(asset_url: &str, signature_filename: &str) -> Result<String, BuildError> {
    if signature_filename.starts_with("http://") || signature_filename.starts_with("https://") {
        return Ok(signature_filename.to_owned());
    }

    let parent = asset_url
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .ok_or_else(|| {
            BuildError::Invalid(format!(
                "cannot derive sidecar URL from asset URL \
                 `{asset_url}`"
            ))
        })?;

    Ok(format!("{parent}/{signature_filename}"))
}

/// Fetch the sidecar file from the given URL.
fn fetch_sidecar(url: &str) -> Result<Vec<u8>, BuildError> {
    let response = ureq::get(url).call().map_err(|error| {
        BuildError::Fetch(format!("failed to fetch release sidecar at {url}: {error}"))
    })?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|error| {
            BuildError::Fetch(format!("failed to read release sidecar at {url}: {error}"))
        })?;

    Ok(bytes)
}

/// Parse a sidecar file as either a raw Ed25519 signature or a minisign
/// signature.
///
/// Minisign format:
/// ```text
/// untrusted comment: <comment>
/// <base64 signature>
/// trusted comment: <comment>
/// <base64 global signature>
/// ```
///
/// Raw format: base64-encoded 64-byte Ed25519 signature.
fn parse_sidecar_signature(sidecar_bytes: &[u8]) -> Result<Signature, BuildError> {
    let text = String::from_utf8_lossy(sidecar_bytes);
    let text = text.trim();

    // Try minisign format first
    if text.starts_with("untrusted comment:") {
        return parse_minisign_signature(text);
    }

    // Try raw base64 Ed25519
    decode_raw_signature(text)
}

/// Parse a minisign-format sidecar.
///
/// The signature is on the second line (after "untrusted comment:").
/// Minisign signatures include a 2-byte algorithm identifier and
/// 8-byte key ID prefix before the 64-byte Ed25519 signature.
fn parse_minisign_signature(text: &str) -> Result<Signature, BuildError> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        return Err(BuildError::Invalid(
            "minisign sidecar is too short".to_owned(),
        ));
    }

    let sig_line = lines[1].trim();
    let decoded = STANDARD.decode(sig_line).map_err(|error| {
        BuildError::Invalid(format!("minisign signature base64 is invalid: {error}"))
    })?;

    // Minisign layout: 2 bytes algorithm + 8 bytes key_id + 64 bytes sig
    if decoded.len() < 74 {
        return Err(BuildError::Invalid(format!(
            "minisign signature is {} bytes, expected at least 74",
            decoded.len()
        )));
    }

    let sig_bytes = &decoded[10..74];
    let array = <[u8; 64]>::try_from(sig_bytes).map_err(|_| {
        BuildError::Invalid("minisign Ed25519 signature extraction failed".to_owned())
    })?;

    Ok(Signature::from_bytes(&array))
}

/// Decode a raw base64-encoded 64-byte Ed25519 signature.
fn decode_raw_signature(text: &str) -> Result<Signature, BuildError> {
    let decoded = STANDARD.decode(text).map_err(|error| {
        BuildError::Invalid(format!("release sidecar base64 is invalid: {error}"))
    })?;

    let array = <[u8; 64]>::try_from(decoded.as_slice()).map_err(|_| {
        BuildError::Invalid(format!(
            "release sidecar signature is {} bytes, expected 64",
            decoded.len()
        ))
    })?;

    Ok(Signature::from_bytes(&array))
}

fn decode_public_key(encoded: &str) -> Result<VerifyingKey, BuildError> {
    let bytes = STANDARD.decode(encoded).map_err(|error| {
        BuildError::Invalid(format!("invalid release trust public key base64: {error}"))
    })?;
    let array = <[u8; 32]>::try_from(bytes.as_slice())
        .map_err(|_| BuildError::Invalid("release trust public key must be 32 bytes".to_owned()))?;

    VerifyingKey::from_bytes(&array)
        .map_err(|error| BuildError::Invalid(format!("invalid release trust public key: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_signature_returns_verdict() {
        let verdict = fetch_and_verify_release_sidecar(
            "https://example.com/release/tool-1.0.tar.gz",
            None,
            Path::new("/tmp/test"),
            &[],
        )
        .expect("missing signature should produce a verdict");
        assert_eq!(verdict, SignatureVerdict::NoSignature);
    }

    #[test]
    fn no_trust_keys_returns_verdict() {
        let verdict = fetch_and_verify_release_sidecar(
            "https://example.com/release/tool-1.0.tar.gz",
            Some("tool-1.0.tar.gz.minisig"),
            Path::new("/tmp/test"),
            &[],
        )
        .expect("missing trust keys should produce a verdict");
        assert_eq!(verdict, SignatureVerdict::NoTrustKeys);
    }

    #[test]
    fn sidecar_url_from_relative_name() {
        let url = derive_sidecar_url(
            "https://github.com/user/repo/releases/download/v1/tool.tar.gz",
            "tool.tar.gz.minisig",
        )
        .expect("relative sidecar URL should resolve");
        assert_eq!(
            url,
            "https://github.com/user/repo/releases/download/v1/tool.tar.gz.minisig"
        );
    }

    #[test]
    fn sidecar_url_from_absolute_url() {
        let url = derive_sidecar_url(
            "https://github.com/user/repo/releases/download/v1/tool.tar.gz",
            "https://example.com/sigs/tool.tar.gz.minisig",
        )
        .expect("absolute sidecar URL should resolve");
        assert_eq!(url, "https://example.com/sigs/tool.tar.gz.minisig");
    }

    #[test]
    fn minisign_signature_parsed() {
        // Construct a valid minisign-formatted sidecar
        // 2 bytes algo + 8 bytes key_id + 64 bytes signature
        let mut sig_bytes = vec![0x45, 0x64]; // algo "Ed"
        sig_bytes.extend_from_slice(&[0u8; 8]); // key_id
        sig_bytes.extend_from_slice(&[0x42u8; 64]); // signature
        let sig_b64 = STANDARD.encode(&sig_bytes);

        let sidecar = format!(
            "untrusted comment: test\n\
             {sig_b64}\n\
             trusted comment: test\n\
             AAAA"
        );

        let result = parse_sidecar_signature(sidecar.as_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn raw_base64_signature_parsed() {
        let sig_bytes = [0x42u8; 64];
        let sig_b64 = STANDARD.encode(sig_bytes);
        let result = parse_sidecar_signature(sig_b64.as_bytes());
        assert!(result.is_ok());
    }
}
