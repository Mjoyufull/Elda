use std::collections::BTreeMap;
use std::fs;

use serde::Serialize;

use crate::app::{AppContext, ParsedInstallRequest, ResolvedInstallTarget};
use crate::config::InstallPreference;
use crate::error::CoreError;
use elda_recipe::{SOURCE_LANE_SOURCE, load_recipe};

use super::artifacts::{
    PublishedArtifactContext, write_artifact_sidecars, write_lock_document_zstd,
};
use super::model::{CiLockDocument, CiLockPackage, PublishedPackageRecord};
use super::publish_plan::PlannedCiWork;
use super::workspace::{
    CiWorkspacePaths, commit_packages_repo, current_unix_timestamp, sign_bytes, signing_key,
    sync_recipe_into_packages_repo, write_json, write_signature_envelope,
};

#[derive(Debug, Clone)]
pub(crate) struct PublishedWorkspace {
    pub(crate) packages: Vec<PublishedPackageRecord>,
    pub(crate) repo_commit: Option<String>,
    pub(crate) trusted_key_fingerprint: String,
}

pub(crate) fn publish_workspace(
    app: &AppContext,
    workspace: &CiWorkspacePaths,
    plan: &PlannedCiWork,
    submission_id: &str,
    channel: &str,
) -> Result<PublishedWorkspace, CoreError> {
    workspace.ensure_exists()?;

    for package in &plan.packages {
        sync_recipe_into_packages_repo(
            workspace,
            &app.database.layout().recipes_dir,
            &package.package_name,
        )?;
    }
    let repo_commit =
        commit_packages_repo(workspace, &format!("sync ci submission {submission_id}"))?;
    let signing_key = signing_key(&workspace.signing_key_path)?;
    let trusted_key_fingerprint = super::workspace::fingerprint_for_key(&signing_key);

    let mut published = Vec::new();
    for package in &plan.packages {
        let resolved = resolve_publish_target(app, &package.package_name)?;
        let built = app.build_resolved_target(&resolved, false, false, None)?;
        let payload_name = built
            .package
            .payload_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| {
                CoreError::Operator(format!(
                    "built payload for `{}` has no file name",
                    package.package_name
                ))
            })?;
        let manifest_name = built
            .package
            .manifest_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| {
                CoreError::Operator(format!(
                    "built manifest for `{}` has no file name",
                    package.package_name
                ))
            })?;
        let published_payload = workspace.artifacts_dir.join(payload_name);
        let published_manifest = workspace.artifacts_dir.join(manifest_name);
        fs::copy(&built.package.payload_path, &published_payload)?;
        fs::copy(&built.package.manifest_path, &published_manifest)?;
        let payload_signature = sign_bytes(&signing_key, &fs::read(&published_payload)?);
        let sidecars = write_artifact_sidecars(PublishedArtifactContext {
            signing_key: &signing_key,
            trusted_key_fingerprint: &trusted_key_fingerprint,
            payload_path: &published_payload,
            manifest_path: &published_manifest,
            payload_signature: &payload_signature,
            recipe_path: &package.recipe_path,
            planned: package,
            built: &built.package,
            repo_commit: repo_commit.as_deref(),
        })?;

        published.push(PublishedPackageRecord {
            pkgname: built.package.package_name.clone(),
            epoch: built.package.epoch,
            pkgver: built.package.pkgver.clone(),
            pkgrel: built.package.pkgrel,
            arch: built.package.arch.clone(),
            variant_id: (!built.package.variant_id.is_empty())
                .then_some(built.package.variant_id.clone()),
            payload_path: published_payload,
            manifest_path: published_manifest,
            payload_sha256: built.package.payload_sha256.clone(),
            manifest_hash: built.package.manifest_hash.clone(),
            payload_signature,
            signature_path: sidecars.signature_path,
            sbom_path: sidecars.sbom_path,
            attestation_path: sidecars.attestation_path,
            repo_commit: repo_commit.clone(),
            published_at: current_unix_timestamp(),
        });
    }

    published.sort_by(|left, right| left.pkgname.cmp(&right.pkgname));
    let lock_document = build_lock_document(plan, &published, repo_commit.clone());
    write_lock_document_zstd(workspace, &lock_document)?;
    write_index_document(
        app,
        workspace,
        &published,
        submission_id,
        repo_commit.clone(),
        channel,
    )?;
    let index_signature = sign_bytes(&signing_key, &fs::read(&workspace.index_path)?);
    write_signature_envelope(&signing_key, &workspace.signature_path, &index_signature)?;

    Ok(PublishedWorkspace {
        packages: published,
        repo_commit,
        trusted_key_fingerprint,
    })
}

pub(crate) fn resolve_publish_target(
    app: &AppContext,
    package_name: &str,
) -> Result<ResolvedInstallTarget, CoreError> {
    let recipe = load_recipe(&app.database.layout().recipes_dir, package_name)?;
    let hard_lane = if recipe
        .package
        .source
        .lane_definition(SOURCE_LANE_SOURCE)
        .is_some()
    {
        Some(InstallPreference::Source)
    } else {
        None
    };
    let request = ParsedInstallRequest {
        targets: vec![package_name.to_owned()],
        hard_lane,
        preferred_lane: Some(InstallPreference::Binary),
        source_option: None,
        source_strategy: None,
        git_ref: None,
        git_source_refs: Default::default(),
        git_ref_overrides: Default::default(),
        cli_flag_overrides: BTreeMap::new(),
        replace: false,
        exclude: Vec::new(),
        provider_choices: BTreeMap::new(),
    };

    app.select_install_lane(
        package_name,
        recipe,
        &request,
        Some(
            app.database
                .layout()
                .recipes_dir
                .join(package_name)
                .display()
                .to_string(),
        ),
    )
}

fn build_lock_document(
    plan: &PlannedCiWork,
    published: &[PublishedPackageRecord],
    repo_commit: Option<String>,
) -> CiLockDocument {
    let mut artifact_by_name = BTreeMap::new();
    for package in published {
        artifact_by_name.insert(package.pkgname.as_str(), package);
    }

    CiLockDocument {
        format_version: 1,
        generated_at: current_unix_timestamp(),
        packages: plan
            .packages
            .iter()
            .map(|package| {
                let artifact = artifact_by_name.get(package.package_name.as_str());
                CiLockPackage {
                    pkgname: package.package_name.clone(),
                    epoch: artifact.map_or(0, |value| value.epoch),
                    pkgver: artifact
                        .map(|value| value.pkgver.clone())
                        .unwrap_or_else(|| "0.0.0".to_owned()),
                    pkgrel: artifact.map_or(0, |value| value.pkgrel),
                    arch: artifact
                        .map(|value| value.arch.clone())
                        .unwrap_or_else(|| "unknown".to_owned()),
                    source_ref: Some(package.recipe_path.display().to_string()),
                    runtime_depends: package.runtime_depends.clone(),
                    makedepends: package.makedepends.clone(),
                    checkdepends: package.checkdepends.clone(),
                    provides: Vec::new(),
                    conflicts: Vec::new(),
                    build_profile: "core".to_owned(),
                    ci_policy: if plan.requested_targets.contains(&package.package_name) {
                        "requested".to_owned()
                    } else {
                        "closure".to_owned()
                    },
                    layer: package.layer,
                    artifact_name: artifact.and_then(|value| {
                        value
                            .payload_path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    }),
                    artifact_sha256: artifact.map(|value| value.payload_sha256.clone()),
                    repo_commit: repo_commit.clone(),
                }
            })
            .collect(),
    }
}

fn write_index_document(
    app: &AppContext,
    workspace: &CiWorkspacePaths,
    published: &[PublishedPackageRecord],
    submission_id: &str,
    repo_commit: Option<String>,
    channel: &str,
) -> Result<(), CoreError> {
    #[derive(Debug, Serialize)]
    struct IndexEnvelope {
        packages: Vec<IndexRecord>,
    }

    #[derive(Debug, Serialize)]
    struct IndexRecord {
        pkgname: String,
        channel: String,
        asset_url: String,
        sha256: String,
        size: u64,
        payload_sig: String,
        sbom_url: String,
        attestation_url: String,
        source_kind: &'static str,
        source_ref: String,
        repo_commit: Option<String>,
        variant_id: Option<String>,
        pkg_lua: String,
    }

    let packages = published
        .iter()
        .map(|package| {
            let pkg_lua_path = app
                .database
                .layout()
                .recipes_dir
                .join(&package.pkgname)
                .join("pkg.lua");
            Ok(IndexRecord {
                pkgname: package.pkgname.clone(),
                channel: channel.to_owned(),
                asset_url: format!("file://{}", package.payload_path.display()),
                sha256: package.payload_sha256.clone(),
                size: fs::metadata(&package.payload_path)?.len(),
                payload_sig: package.payload_signature.clone(),
                sbom_url: format!("file://{}", package.sbom_path.display()),
                attestation_url: format!("file://{}", package.attestation_path.display()),
                source_kind: "repo_binary",
                source_ref: format!("ci:{submission_id}"),
                repo_commit: repo_commit.clone(),
                variant_id: package.variant_id.clone(),
                pkg_lua: fs::read_to_string(pkg_lua_path)?,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;

    write_json(&workspace.index_path, &IndexEnvelope { packages })
}
