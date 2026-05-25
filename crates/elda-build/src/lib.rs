#![forbid(unsafe_code)]

mod ad_hoc;
mod archive;
mod binary_fetch;
mod cache_meta;
mod cargo_build;
mod cmake_build;
mod error;
mod git;
mod go_build;
mod interbuild;
mod make_build;
mod manifest;
mod meson_build;
mod nimble_build;
mod object_analysis;
mod payload_verify;
mod process;
mod python_build;
mod release_trust;
mod system_metadata;
mod zig_build;

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tempfile::TempDir;

use elda_recipe::{
    DependencyBody, DependencyEntry, PackageDefinition, RecipeDocument, load_recipe,
};
use elda_types::CrateBoundary;

pub use cache_meta::{
    CacheEntryKind, CacheEntryMetadata, cache_metadata_path, load_cache_metadata,
    record_cache_access,
};
pub use error::BuildError;
pub use git::ensure_git_protocol_allowed;
pub use interbuild::{
    ArchSourceReport, AurReport, GentooReport, InterbuildReport, LockfileReport, NixMetaReport,
    XbpsReport,
};
pub use manifest::{ManifestEntry, ManifestEntryKind, PackageManifest};
pub use object_analysis::{ObjectMetadata, SharedLibraryProvide, SharedLibraryRequirement};
pub use system_metadata::{
    AlternativeAsset, DeclarativeAsset, LifecycleHookAsset, ProviderAsset, ProviderTreeEntry,
    SystemPackageMetadata,
};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-build",
    "Build orchestration, staging roots, and payload assembly.",
);

#[derive(Clone)]
pub struct BuildRequest<'a> {
    pub recipe: &'a RecipeDocument,
    pub cache_src_dir: &'a Path,
    pub cache_pkg_dir: &'a Path,
    pub tmp_dir: &'a Path,
    pub offline: bool,
    pub binary_caches: Vec<BinaryCache>,
    pub remote_name: Option<String>,
    pub binary_source_verification: Option<BinarySourceVerification>,
    pub release_trusted_keys: Vec<String>,
    pub allowed_git_protocols: Vec<String>,
    pub persisted_source_kind: String,
    pub persisted_source_ref: Option<String>,
    pub variant_id: String,
    pub ad_hoc_git: bool,
    /// When true, builders that support it attach child stdout/stderr to the terminal (human installs).
    pub stream_child_output: bool,
    /// Optional hook for bounded build-tool status lines (ProgressSink inner build frame).
    pub build_line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryCache {
    pub name: String,
    pub base_url: String,
    pub priority: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BinarySourceVerification {
    pub remote_name: String,
    pub payload_signature: Option<String>,
    pub trusted_public_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuiltPackage {
    pub package_name: String,
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
    pub arch: String,
    pub package_kind: String,
    pub variant_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub remote_name: Option<String>,
    pub repo_commit: Option<String>,
    pub dependencies: Vec<PackageDependency>,
    pub conffiles: Vec<String>,
    pub system_metadata: SystemPackageMetadata,
    pub object_metadata: ObjectMetadata,
    pub payload_path: PathBuf,
    pub payload_sha256: String,
    pub manifest_path: PathBuf,
    pub manifest_hash: String,
    pub manifest: PackageManifest,
    pub interbuild: Option<InterbuildReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageDependency {
    pub dependency_name: String,
    pub dependency_kind: String,
    pub raw_expr: String,
    pub is_weak: bool,
    pub provider_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub recipe: RecipeDocument,
    pub persisted_source_kind: String,
    pub persisted_source_ref: Option<String>,
}

pub fn resolve_local_target(
    recipes_dir: &Path,
    target: &str,
    persisted_source_kind: String,
    persisted_source_ref: Option<String>,
) -> Result<ResolvedTarget, BuildError> {
    Ok(ResolvedTarget {
        recipe: load_recipe(recipes_dir, target)
            .map_err(|error| BuildError::Invalid(error.to_string()))?,
        persisted_source_kind,
        persisted_source_ref,
    })
}

pub fn build_recipe(request: BuildRequest<'_>) -> Result<BuiltPackage, BuildError> {
    fs::create_dir_all(request.cache_pkg_dir)?;
    fs::create_dir_all(request.cache_src_dir)?;
    fs::create_dir_all(request.tmp_dir)?;

    let build_root = TempDir::new_in(request.tmp_dir)?;
    let stage_root = build_root.path().join("stage");
    fs::create_dir_all(&stage_root)?;
    let (repo_commit, repo_commit_unix, interbuild) =
        match request.recipe.package.source.kind.as_str() {
            "git" => {
                let checkout = git::checkout_git_source(
                    request.recipe,
                    build_root.path(),
                    request.offline,
                    &request.allowed_git_protocols,
                    request.build_line_hook.clone(),
                )?;

                // Auto-detect foreign build definitions in the checkout.
                // If a flake.nix, PKGBUILD, or XBPS template is present,
                // run the corresponding interbuild parser to extract
                // metadata. On validation failure, fall back to the
                // generic git source build.
                let interbuild_report =
                    if let Some(kind) = interbuild::detect_interbuild_kind(&checkout.source_dir) {
                        interbuild::validate_interbuild_in_checkout(
                            request.recipe,
                            kind,
                            &checkout.source_dir,
                        )
                        .ok()
                    } else {
                        None
                    };

                build_source_tree(
                    request.recipe,
                    &checkout.source_dir,
                    &stage_root,
                    request.stream_child_output,
                    request.build_line_hook.clone(),
                )?;

                (
                    checkout.repo_commit,
                    checkout.repo_commit_unix,
                    interbuild_report,
                )
            }
            "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template" => {
                let checkout =
                    if let Some(local_root) = interbuild::local_interbuild_root(request.recipe) {
                        interbuild::prepare_local_interbuild_source(
                            request.recipe,
                            &local_root,
                            build_root.path(),
                            request.offline,
                            &request.allowed_git_protocols,
                            request.build_line_hook.clone(),
                        )?
                    } else {
                        interbuild::prepare_interbuild_source(
                            request.recipe,
                            build_root.path(),
                            request.offline,
                            &request.allowed_git_protocols,
                            request.build_line_hook.clone(),
                        )?
                    };
                build_source_tree(
                    request.recipe,
                    &checkout.source_dir,
                    &stage_root,
                    request.stream_child_output,
                    request.build_line_hook.clone(),
                )?;

                (
                    checkout.checkout.repo_commit,
                    checkout.checkout.repo_commit_unix,
                    Some(checkout.report),
                )
            }
            "url_archive" | "github_release" | "release_asset" | "appimage" => {
                archive::stage_binary_source(
                    request.recipe,
                    request.cache_src_dir,
                    &stage_root,
                    request.offline,
                    &request.binary_caches,
                    request.binary_source_verification.as_ref(),
                    &request.release_trusted_keys,
                )?;
                (None, None, None)
            }
            other => {
                return Err(BuildError::Unsupported(format!(
                    "source.kind `{other}` is not implemented by the current build slice"
                )));
            }
        };

    let manifest = manifest::collect_manifest(&stage_root)?;
    let (manifest_hash, manifest_bytes) = manifest::manifest_hash(&manifest)?;
    let system_metadata = system_metadata::collect_system_metadata(request.recipe)?;
    let object_metadata = object_analysis::analyze_stage_objects(&stage_root, &manifest)?;
    let arch = request
        .recipe
        .package
        .arch
        .first()
        .cloned()
        .ok_or_else(|| BuildError::Invalid("recipe is missing a canonical arch".to_owned()))?;
    let pkgver = ad_hoc::resolved_pkgver(&request, repo_commit.as_deref(), repo_commit_unix)?;
    let pkgrel = ad_hoc::resolved_pkgrel(&request);
    let base_name = format!(
        "{}-{}-{}-{}",
        request.recipe.package.name, pkgver, pkgrel, arch
    );
    let payload_path = request
        .cache_pkg_dir
        .join(format!("{base_name}.pkg.tar.zst"));
    let manifest_path = request.cache_pkg_dir.join(format!("{base_name}.manifest"));
    manifest::archive_stage(&stage_root, &payload_path)?;
    fs::write(&manifest_path, manifest_bytes)?;
    cache_meta::record_cache_access(&payload_path, CacheEntryKind::PackagePayload)?;
    cache_meta::record_cache_access(&manifest_path, CacheEntryKind::PackageManifest)?;
    let payload_sha256 = manifest::sha256_file(&payload_path)?;

    Ok(BuiltPackage {
        package_name: request.recipe.package.name.clone(),
        epoch: request.recipe.package.epoch,
        pkgver,
        pkgrel,
        arch,
        package_kind: request.recipe.package.kind.clone(),
        variant_id: request.variant_id,
        source_kind: request.persisted_source_kind,
        source_ref: request.persisted_source_ref,
        remote_name: request.remote_name,
        repo_commit,
        dependencies: collect_package_dependencies(&request.recipe.package),
        conffiles: request.recipe.package.conffiles.clone(),
        system_metadata,
        object_metadata,
        payload_path,
        payload_sha256,
        manifest_path,
        manifest_hash,
        manifest,
        interbuild,
    })
}

fn build_source_tree(
    recipe: &RecipeDocument,
    source_dir: &Path,
    stage_root: &Path,
    stream_child_output: bool,
    build_line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<(), BuildError> {
    if matches!(recipe.package.kind.as_str(), "meta" | "profile") {
        return Ok(());
    }

    let build = match &recipe.package.build {
        Some(build) => Some(build.clone()),
        None => detect_build_definition(&recipe.package, source_dir)?,
    }
    .ok_or_else(|| {
        BuildError::Unsupported(format!(
            "no supported declarative build path was found for `{}`",
            recipe.package.name
        ))
    })?;

    match build.system.as_str() {
        "cargo" => cargo_build::build_with_cargo(
            &build,
            source_dir,
            stage_root,
            stream_child_output,
            build_line_hook,
        ),
        "cmake" => cmake_build::build_with_cmake(
            &build,
            source_dir,
            stage_root,
            stream_child_output,
            build_line_hook,
        ),
        "meson" => meson_build::build_with_meson(
            &build,
            source_dir,
            stage_root,
            stream_child_output,
            build_line_hook,
        ),
        "make" => make_build::build_with_make(
            &build,
            source_dir,
            stage_root,
            stream_child_output,
            build_line_hook,
        ),
        "go" => go_build::build_with_go(&build, &recipe.package, source_dir, stage_root),
        "zig" => zig_build::build_with_zig(&build, source_dir, stage_root),
        "python" => python_build::build_with_python(&build, source_dir, stage_root),
        "nim" | "nimble" => {
            nimble_build::build_with_nimble(&build, &recipe.package, source_dir, stage_root)
        }
        other => Err(BuildError::Unsupported(format!(
            "build.system `{other}` is not implemented by the current execution slice"
        ))),
    }
}

fn detect_build_definition(
    package: &elda_recipe::PackageDefinition,
    source_dir: &Path,
) -> Result<Option<elda_recipe::BuildDefinition>, BuildError> {
    let detectors = [
        cargo_build::detect_cargo_build(package, source_dir)?,
        cmake_build::detect_cmake_build(package, source_dir)?,
        go_build::detect_go_build(package, source_dir)?,
        meson_build::detect_meson_build(package, source_dir)?,
        zig_build::detect_zig_build(package, source_dir)?,
        python_build::detect_python_build(package, source_dir)?,
        nimble_build::detect_nimble_build(package, source_dir)?,
        make_build::detect_make_build(package, source_dir)?,
    ];

    Ok(detectors.into_iter().flatten().next())
}

fn collect_package_dependencies(package: &PackageDefinition) -> Vec<PackageDependency> {
    let mut dependencies = Vec::new();
    push_dependency_family(&mut dependencies, "depends", false, &package.depends);
    push_dependency_family(&mut dependencies, "recommends", true, &package.recommends);
    push_dependency_family(&mut dependencies, "suggests", true, &package.suggests);
    push_dependency_family(&mut dependencies, "supplements", true, &package.supplements);
    push_dependency_family(&mut dependencies, "enhances", true, &package.enhances);
    dependencies
}

fn push_dependency_family(
    dependencies: &mut Vec<PackageDependency>,
    dependency_kind: &str,
    is_weak: bool,
    entries: &[DependencyEntry],
) {
    for entry in entries {
        match &entry.body {
            DependencyBody::Constraint(value) => dependencies.push(PackageDependency {
                dependency_name: dependency_name_from_constraint(value),
                dependency_kind: dependency_kind.to_owned(),
                raw_expr: render_dependency_expr(value, entry),
                is_weak,
                provider_group: None,
            }),
            DependencyBody::AnyOf(providers) => {
                let provider_group = providers.join(" | ");
                for provider in providers {
                    dependencies.push(PackageDependency {
                        dependency_name: dependency_name_from_constraint(provider),
                        dependency_kind: dependency_kind.to_owned(),
                        raw_expr: render_dependency_expr(&format!("any({provider_group})"), entry),
                        is_weak,
                        provider_group: Some(provider_group.clone()),
                    });
                }
            }
        }
    }
}

fn render_dependency_expr(body_expr: &str, entry: &DependencyEntry) -> String {
    match entry.when.as_ref() {
        Some(predicate) => format!("{body_expr} when [{}]", predicate.raw),
        None => body_expr.to_owned(),
    }
}

fn dependency_name_from_constraint(constraint: &str) -> String {
    constraint.find(['<', '>', '=', '!']).map_or_else(
        || constraint.trim().to_owned(),
        |index| constraint[..index].trim().to_owned(),
    )
}
