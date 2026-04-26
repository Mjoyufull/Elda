use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::CoreError;

use super::model::CiLockDocument;
use super::publish_plan::PlannedCiPackage;
use super::workspace::{CiWorkspacePaths, current_unix_timestamp};

#[derive(Debug, Clone)]
pub(crate) struct GeneratedArtifactSidecars {
    pub(crate) signature_path: PathBuf,
    pub(crate) sbom_path: PathBuf,
    pub(crate) attestation_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct PublishedArtifactContext<'a> {
    pub(crate) signing_key: &'a SigningKey,
    pub(crate) trusted_key_fingerprint: &'a str,
    pub(crate) payload_path: &'a Path,
    pub(crate) manifest_path: &'a Path,
    pub(crate) payload_signature: &'a str,
    pub(crate) recipe_path: &'a Path,
    pub(crate) planned: &'a PlannedCiPackage,
    pub(crate) built: &'a elda_build::BuiltPackage,
    pub(crate) repo_commit: Option<&'a str>,
}

pub(crate) fn lock_document_path(workspace: &CiWorkspacePaths) -> PathBuf {
    workspace.locks_dir.join("lock-v1.json.zst")
}

pub(crate) fn write_lock_document_zstd(
    workspace: &CiWorkspacePaths,
    document: &CiLockDocument,
) -> Result<PathBuf, CoreError> {
    let path = lock_document_path(workspace);
    let parent = path.parent().ok_or_else(|| {
        CoreError::Operator(format!("path `{}` has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;

    let bytes = serde_json::to_vec_pretty(document)?;
    let mut encoder = zstd::Encoder::new(fs::File::create(&path)?, 19)?;
    encoder.write_all(&bytes)?;
    encoder.finish()?;

    Ok(path)
}

pub(crate) fn write_artifact_sidecars(
    context: PublishedArtifactContext<'_>,
) -> Result<GeneratedArtifactSidecars, CoreError> {
    let base_name = artifact_base_name(context.built);
    let artifacts_dir = context.payload_path.parent().ok_or_else(|| {
        CoreError::Operator(format!(
            "published payload `{}` has no parent directory",
            context.payload_path.display()
        ))
    })?;
    let signature_path = artifacts_dir.join(format!("{base_name}.minisig"));
    let sbom_path = artifacts_dir.join(format!("{base_name}.spdx.json"));
    let attestation_path = artifacts_dir.join(format!("{base_name}.attestation.json"));

    let sbom = build_sbom_document(&context);
    let sbom_bytes = serde_json::to_vec_pretty(&sbom)?;
    fs::write(&sbom_path, &sbom_bytes)?;
    let sbom_sha256 = sha256_bytes(&sbom_bytes);

    let attestation = build_attestation_document(&context, &sbom_sha256)?;
    let attestation_bytes = serde_json::to_vec_pretty(&attestation)?;
    fs::write(&attestation_path, &attestation_bytes)?;

    super::workspace::write_signature_envelope(
        context.signing_key,
        &signature_path,
        context.payload_signature,
    )?;

    Ok(GeneratedArtifactSidecars {
        signature_path,
        sbom_path,
        attestation_path,
    })
}

fn artifact_base_name(built: &elda_build::BuiltPackage) -> String {
    format!(
        "{}-{}-{}-{}",
        built.package_name, built.pkgver, built.pkgrel, built.arch
    )
}

#[derive(Debug, Serialize)]
struct SbomDocument {
    schema: &'static str,
    spdx_version: &'static str,
    data_license: &'static str,
    generated_at: u64,
    package: SbomPackage,
    files: Vec<SbomFile>,
    relationships: SbomRelationships,
}

#[derive(Debug, Serialize)]
struct SbomPackage {
    name: String,
    version: String,
    arch: String,
    variant_id: String,
    source_kind: String,
    source_ref: Option<String>,
    repo_commit: Option<String>,
    recipe_path: String,
    payload_sha256: String,
    manifest_hash: String,
}

#[derive(Debug, Serialize)]
struct SbomFile {
    path: String,
    kind: String,
    size: u64,
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct SbomRelationships {
    runtime_depends: Vec<String>,
    makedepends: Vec<String>,
    checkdepends: Vec<String>,
}

fn build_sbom_document(context: &PublishedArtifactContext<'_>) -> SbomDocument {
    SbomDocument {
        schema: "elda-spdx-lite-v1",
        spdx_version: "SPDX-2.3",
        data_license: "CC0-1.0",
        generated_at: current_unix_timestamp(),
        package: SbomPackage {
            name: context.built.package_name.clone(),
            version: format!(
                "{}:{}-{}",
                context.built.epoch, context.built.pkgver, context.built.pkgrel
            ),
            arch: context.built.arch.clone(),
            variant_id: context.built.variant_id.clone(),
            source_kind: context.built.source_kind.clone(),
            source_ref: context.built.source_ref.clone(),
            repo_commit: context.repo_commit.map(ToOwned::to_owned),
            recipe_path: context.recipe_path.display().to_string(),
            payload_sha256: context.built.payload_sha256.clone(),
            manifest_hash: context.built.manifest_hash.clone(),
        },
        files: context
            .built
            .manifest
            .entries
            .iter()
            .map(|entry| SbomFile {
                path: entry.path.clone(),
                kind: format!("{:?}", entry.kind).to_ascii_lowercase(),
                size: entry.size,
                sha256: entry.sha256.clone(),
            })
            .collect(),
        relationships: SbomRelationships {
            runtime_depends: context.planned.runtime_depends.clone(),
            makedepends: context.planned.makedepends.clone(),
            checkdepends: context.planned.checkdepends.clone(),
        },
    }
}

#[derive(Debug, Serialize)]
struct AttestationDocument {
    schema: &'static str,
    generated_at: u64,
    signer: AttestationSigner,
    subject: AttestationSubject,
    provenance: AttestationProvenance,
}

#[derive(Debug, Serialize)]
struct AttestationSigner {
    key_fingerprint: String,
    signature: String,
}

#[derive(Debug, Serialize)]
struct AttestationSubject {
    package_name: String,
    version: String,
    arch: String,
    variant_id: String,
    payload_path: String,
    payload_sha256: String,
    manifest_path: String,
    manifest_hash: String,
    sbom_path: String,
    sbom_sha256: String,
    payload_signature: String,
}

#[derive(Debug, Serialize)]
struct AttestationProvenance {
    build_system: &'static str,
    source_kind: String,
    source_ref: Option<String>,
    repo_commit: Option<String>,
    recipe_path: String,
    runtime_depends: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AttestationUnsigned {
    generated_at: u64,
    subject: AttestationSubject,
    provenance: AttestationProvenance,
}

fn build_attestation_document(
    context: &PublishedArtifactContext<'_>,
    sbom_sha256: &str,
) -> Result<AttestationDocument, CoreError> {
    let sbom_path = context
        .payload_path
        .parent()
        .ok_or_else(|| {
            CoreError::Operator(format!(
                "published payload `{}` has no parent directory",
                context.payload_path.display()
            ))
        })?
        .join(format!("{}.spdx.json", artifact_base_name(context.built)));
    let unsigned = AttestationUnsigned {
        generated_at: current_unix_timestamp(),
        subject: AttestationSubject {
            package_name: context.built.package_name.clone(),
            version: format!(
                "{}:{}-{}",
                context.built.epoch, context.built.pkgver, context.built.pkgrel
            ),
            arch: context.built.arch.clone(),
            variant_id: context.built.variant_id.clone(),
            payload_path: context.payload_path.display().to_string(),
            payload_sha256: context.built.payload_sha256.clone(),
            manifest_path: context.manifest_path.display().to_string(),
            manifest_hash: context.built.manifest_hash.clone(),
            sbom_path: sbom_path.display().to_string(),
            sbom_sha256: sbom_sha256.to_owned(),
            payload_signature: context.payload_signature.to_owned(),
        },
        provenance: AttestationProvenance {
            build_system: "elda-local-ci",
            source_kind: context.built.source_kind.clone(),
            source_ref: context.built.source_ref.clone(),
            repo_commit: context.repo_commit.map(ToOwned::to_owned),
            recipe_path: context.recipe_path.display().to_string(),
            runtime_depends: context.planned.runtime_depends.clone(),
        },
    };
    let unsigned_bytes = serde_json::to_vec(&unsigned)?;
    let signature = context.signing_key.sign(&unsigned_bytes);

    Ok(AttestationDocument {
        schema: "elda-attestation-v1",
        generated_at: unsigned.generated_at,
        signer: AttestationSigner {
            key_fingerprint: context.trusted_key_fingerprint.to_owned(),
            signature: base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
        },
        subject: unsigned.subject,
        provenance: unsigned.provenance,
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
