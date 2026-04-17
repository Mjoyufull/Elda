use std::fs;
use std::path::Path;

use serde_json::json;

use crate::error::RecipeError;

use super::detect::{discover_source_url, infer_recipe_name, is_git_like_target};
use super::legacy::{copy_dir_recursive, parse_pkgdeps, render_imported_build_lua};
use super::model::ImportReport;
use super::render::render_pkg_lua;

pub fn add_recipe(recipes_dir: &Path, input: &str) -> Result<ImportReport, RecipeError> {
    fs::create_dir_all(recipes_dir)?;

    let input_path = Path::new(input);
    if input_path.exists() {
        return import_from_local_path(recipes_dir, input_path);
    }
    if is_git_like_target(input) {
        return scaffold_from_source(recipes_dir, infer_recipe_name(input), Some(input));
    }

    scaffold_from_source(recipes_dir, input.to_owned(), None)
}

fn import_from_local_path(
    recipes_dir: &Path,
    source_dir: &Path,
) -> Result<ImportReport, RecipeError> {
    let recipe_name = source_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            RecipeError::InvalidInput("could not derive recipe name from source path".to_owned())
        })?;
    let recipe_dir = recipes_dir.join(&recipe_name);
    fs::create_dir_all(&recipe_dir)?;

    let source_pkg_lua = source_dir.join("pkg.lua");
    let source_build_lua = source_dir.join("build.lua");
    let source_patches = source_dir.join("patches");
    let source_pkgdeps = source_dir.join("pkgdeps");
    let source_bldit = source_dir.join("bldit");
    let legacy_pkgdeps = load_legacy_pkgdeps(&source_pkgdeps)?;
    let mut report = empty_report(recipe_name, recipe_dir.clone());

    copy_recipe_inputs(
        &source_pkg_lua,
        &source_build_lua,
        &source_patches,
        &recipe_dir,
        &mut report,
    )?;
    ensure_recipe_pkg_lua(&recipe_dir, source_dir, &legacy_pkgdeps, &mut report)?;
    import_legacy_files(
        source_dir,
        &recipe_dir,
        &source_pkgdeps,
        &source_bldit,
        legacy_pkgdeps,
        &mut report,
    )?;

    Ok(report)
}

fn scaffold_from_source(
    recipes_dir: &Path,
    recipe_name: String,
    source_url: Option<&str>,
) -> Result<ImportReport, RecipeError> {
    let recipe_dir = recipes_dir.join(&recipe_name);
    fs::create_dir_all(&recipe_dir)?;

    let pkg_lua_path = recipe_dir.join("pkg.lua");
    if !pkg_lua_path.exists() {
        fs::write(&pkg_lua_path, render_pkg_lua(&recipe_name, source_url, &[]))?;
    }

    Ok(ImportReport {
        recipe_name,
        recipe_dir,
        imported_pkg_lua: false,
        imported_build_lua: false,
        imported_patches: false,
        generated_pkg_lua: true,
        generated_build_lua: false,
        imported_legacy_pkgdeps: false,
        imported_legacy_bldit: false,
        wrote_legacy_summary: false,
    })
}

fn load_legacy_pkgdeps(
    pkgdeps_path: &Path,
) -> Result<Option<Vec<super::model::LegacyPkgdep>>, RecipeError> {
    if pkgdeps_path.is_file() {
        return Ok(Some(parse_pkgdeps(pkgdeps_path)?));
    }

    Ok(None)
}

fn empty_report(recipe_name: String, recipe_dir: std::path::PathBuf) -> ImportReport {
    ImportReport {
        recipe_name,
        recipe_dir,
        imported_pkg_lua: false,
        imported_build_lua: false,
        imported_patches: false,
        generated_pkg_lua: false,
        generated_build_lua: false,
        imported_legacy_pkgdeps: false,
        imported_legacy_bldit: false,
        wrote_legacy_summary: false,
    }
}

fn copy_recipe_inputs(
    source_pkg_lua: &Path,
    source_build_lua: &Path,
    source_patches: &Path,
    recipe_dir: &Path,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    if source_pkg_lua.is_file() {
        fs::copy(source_pkg_lua, recipe_dir.join("pkg.lua"))?;
        report.imported_pkg_lua = true;
    }
    if source_build_lua.is_file() {
        fs::copy(source_build_lua, recipe_dir.join("build.lua"))?;
        report.imported_build_lua = true;
    }
    if source_patches.is_dir() {
        copy_dir_recursive(source_patches, &recipe_dir.join("patches"))?;
        report.imported_patches = true;
    }

    Ok(())
}

fn ensure_recipe_pkg_lua(
    recipe_dir: &Path,
    source_dir: &Path,
    legacy_pkgdeps: &Option<Vec<super::model::LegacyPkgdep>>,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    if report.imported_pkg_lua {
        return Ok(());
    }

    let source_url = discover_source_url(source_dir);
    fs::write(
        recipe_dir.join("pkg.lua"),
        render_pkg_lua(
            &report.recipe_name,
            source_url.as_deref(),
            legacy_pkgdeps.as_deref().unwrap_or(&[]),
        ),
    )?;
    report.generated_pkg_lua = true;

    Ok(())
}

fn import_legacy_files(
    source_dir: &Path,
    recipe_dir: &Path,
    source_pkgdeps: &Path,
    source_bldit: &Path,
    legacy_pkgdeps: Option<Vec<super::model::LegacyPkgdep>>,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    if !source_pkgdeps.is_file() && !source_bldit.is_file() {
        return Ok(());
    }

    let legacy_dir = recipe_dir.join("legacy");
    fs::create_dir_all(&legacy_dir)?;

    if source_pkgdeps.is_file() {
        fs::copy(source_pkgdeps, legacy_dir.join("pkgdeps"))?;
        report.imported_legacy_pkgdeps = true;
    }

    if source_bldit.is_file() {
        fs::copy(source_bldit, legacy_dir.join("pkgit.bldit"))?;
        report.imported_legacy_bldit = true;
        if !report.imported_build_lua {
            fs::write(
                recipe_dir.join("build.lua"),
                render_imported_build_lua(&report.recipe_name),
            )?;
            report.generated_build_lua = true;
        }
    }

    fs::write(
        legacy_dir.join("pkgit-import.json"),
        serde_json::to_string_pretty(&json!({
            "source_path": source_dir,
            "source_url": discover_source_url(source_dir),
            "pkgdeps": legacy_pkgdeps,
            "has_bldit": source_bldit.is_file(),
        }))?,
    )?;
    report.wrote_legacy_summary = true;

    Ok(())
}
