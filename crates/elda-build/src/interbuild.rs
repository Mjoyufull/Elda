mod aur;
mod gentoo;
pub(crate) mod gpkg;
mod local;
mod metadata;
pub(crate) mod nix;
mod shell;
mod source;
mod xbps;

use std::fs;
use std::path::{Path, PathBuf};

use elda_recipe::RecipeDocument;

use crate::error::BuildError;
use crate::git::{SourceCheckout, checkout_source};
use crate::process::emit_build_line;

pub use local::{local_interbuild_root, snapshot_rev};

pub use metadata::{
    ArchSourceReport, AurReport, GentooReport, InterbuildReport, LockfileReport,
    PhaseCommandReport, XbpsReport,
};
pub use nix::NixMetaReport;

pub fn prepare_interbuild_source(
    recipe: &RecipeDocument,
    work_root: &std::path::Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<InterbuildCheckout, BuildError> {
    emit_build_line(
        &line_hook,
        format!(
            "[Interbuild] preparing {} source",
            recipe.package.source.kind
        ),
    );
    let checkout = checkout_source(
        &recipe.package.source,
        work_root,
        offline,
        allowed_git_protocols,
        line_hook.clone(),
    )?;
    finish_interbuild_checkout(
        recipe,
        checkout,
        work_root,
        offline,
        allowed_git_protocols,
        line_hook,
    )
}

pub fn prepare_local_interbuild_source(
    recipe: &RecipeDocument,
    local_root: &Path,
    work_root: &Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<InterbuildCheckout, BuildError> {
    emit_build_line(&line_hook, "[Interbuild] using local recipe metadata");
    let checkout = SourceCheckout {
        source_dir: local_root.to_path_buf(),
        repo_commit: snapshot_rev(recipe),
        repo_commit_unix: None,
    };
    finish_interbuild_checkout(
        recipe,
        checkout,
        work_root,
        offline,
        allowed_git_protocols,
        line_hook,
    )
}

fn finish_interbuild_checkout(
    recipe: &RecipeDocument,
    checkout: SourceCheckout,
    work_root: &Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<InterbuildCheckout, BuildError> {
    match recipe.package.source.kind.as_str() {
        "nix_flake" => {
            let source_dir = checkout.source_dir.clone();
            let report = nix::validate_flake(recipe, &source_dir)?;
            Ok(InterbuildCheckout {
                source_dir,
                checkout,
                report,
            })
        }
        "gentoo_overlay" => {
            let mut validation = gentoo::validate_ebuild(recipe, &checkout.source_dir)?;

            // GPKG binary fast-path: if a binhost is configured and a
            // USE-compatible binary package exists, short-circuit the
            // source build and extract the pre-compiled payload.
            if let Some(gentoo_report) = &mut validation.report.gentoo {
                let binhost = gentoo::binhost_from_recipe(recipe);
                let required_use = gentoo_report.iuse.clone();
                if let Some(gpkg_result) = gpkg::try_gpkg_fast_path(
                    binhost.as_deref(),
                    &gentoo::category_from_recipe(recipe),
                    &gentoo_report.package,
                    &required_use,
                    &validation.package_dir,
                )? {
                    gentoo_report.gpkg_used = gpkg_result.used_binary;
                    gentoo_report.gpkg_use = gpkg_result.gpkg_use;
                }
            }

            let source_dir = source::materialize_or_use_checkout(
                "gentoo_overlay",
                &validation.package_dir,
                validation.upstream_source.as_ref(),
                work_root,
                offline,
                allowed_git_protocols,
                line_hook.clone(),
            )?;

            Ok(InterbuildCheckout {
                source_dir,
                checkout,
                report: validation.report,
            })
        }
        "aur_pkgbuild" => {
            let source_dir = checkout.source_dir.clone();
            let report = aur::validate_pkgbuild(recipe, &source_dir)?;
            Ok(InterbuildCheckout {
                source_dir,
                checkout,
                report,
            })
        }
        "xbps_template" => {
            let validation = xbps::validate_template(recipe, &checkout.source_dir)?;
            emit_build_line(&line_hook, "[Interbuild] resolving xbps upstream source");
            let source_dir = source::materialize_or_use_checkout(
                "xbps_template",
                &checkout.source_dir,
                validation.upstream_source.as_ref(),
                work_root,
                offline,
                allowed_git_protocols,
                line_hook,
            )?;
            Ok(InterbuildCheckout {
                source_dir,
                checkout,
                report: validation.report,
            })
        }
        other => Err(BuildError::Unsupported(format!(
            "interbuild source kind `{other}` is not implemented"
        ))),
    }
}

/// Run interbuild validation against a pre-existing checkout directory.
/// Used by auto-promotion when a git checkout is found to contain
/// foreign build definitions.
pub(crate) fn validate_interbuild_in_checkout(
    recipe: &RecipeDocument,
    kind: &str,
    source_dir: &Path,
) -> Result<InterbuildReport, BuildError> {
    match kind {
        "nix_flake" => nix::validate_flake(recipe, source_dir),
        "aur_pkgbuild" => aur::validate_pkgbuild(recipe, source_dir),
        "xbps_template" => {
            xbps::validate_template(recipe, source_dir).map(|validation| validation.report)
        }
        // Gentoo overlays have a different directory structure (category/package/)
        // and are unlikely to be the root of a git checkout. Skip auto-promotion
        // for overlays — they require explicit source.kind = gentoo_overlay.
        _ => Err(BuildError::Unsupported(format!(
            "auto-promotion for interbuild kind `{kind}` is not supported"
        ))),
    }
}

/// Scan a checked-out directory for foreign build definitions.
/// Returns the interbuild source kind if one is detected, in priority
/// order: nix_flake > aur_pkgbuild > xbps_template.
///
/// Gentoo overlays are excluded from auto-detection because their
/// directory structure (category/package/*.ebuild) is distinctive
/// enough that the recipe strategy detection in elda-recipe already
/// handles them correctly.
pub(crate) fn detect_interbuild_kind(source_dir: &Path) -> Option<&'static str> {
    if source_dir.join("flake.nix").is_file() {
        return Some("nix_flake");
    }
    if source_dir.join("PKGBUILD").is_file() {
        return Some("aur_pkgbuild");
    }
    if source_dir.join("template").is_file()
        && looks_like_xbps_template(&source_dir.join("template"))
    {
        return Some("xbps_template");
    }
    None
}

/// Simple heuristic to distinguish an actual XBPS-SRC template from
/// a coincidentally named "template" file. Real templates always have
/// a `pkgname=` assignment.
fn looks_like_xbps_template(path: &Path) -> bool {
    fs::read_to_string(path).ok().is_some_and(|contents| {
        contents.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("pkgname=") || trimmed.starts_with("pkgname =")
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterbuildCheckout {
    pub checkout: SourceCheckout,
    pub source_dir: PathBuf,
    pub report: InterbuildReport,
}
