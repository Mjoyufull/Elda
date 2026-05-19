use std::fs;
use std::path::Path;

use serde_json::json;

use crate::error::RecipeError;

use super::detect::{discover_source_url, infer_recipe_name, is_git_like_target};
use super::legacy::{copy_dir_recursive, parse_pkgdeps, render_imported_build_lua};
use super::model::{ImportOptions, ImportReport, ImportResult};
use super::strategy::{
    detect_source_strategy_for_source, select_source_option_by_index, selected_source_option,
    source_dir_for_detection, source_options_with_priority,
};

pub fn add_recipe(
    recipes_dir: &Path,
    input: &str,
    recipe_kind: Option<&str>,
) -> Result<ImportResult, RecipeError> {
    add_recipe_with_options(recipes_dir, input, recipe_kind, &ImportOptions::default())
}

pub fn add_recipe_with_priority(
    recipes_dir: &Path,
    input: &str,
    recipe_kind: Option<&str>,
    strategy_priority: &[String],
    release_binary_format_priority: &[String],
) -> Result<ImportResult, RecipeError> {
    add_recipe_with_options(
        recipes_dir,
        input,
        recipe_kind,
        &ImportOptions {
            strategy_priority: strategy_priority.to_vec(),
            release_binary_format_priority: release_binary_format_priority.to_vec(),
            selected_source_option: None,
            git_ref: None,
            replace: false,
            exclude: Vec::new(),
        },
    )
}

pub fn add_recipe_with_options(
    recipes_dir: &Path,
    input: &str,
    recipe_kind: Option<&str>,
    options: &ImportOptions,
) -> Result<ImportResult, RecipeError> {
    let recipe_kind = normalize_recipe_kind(recipe_kind)?;
    fs::create_dir_all(recipes_dir)?;

    let input_path = Path::new(input);
    if input_path.exists() {
        return import_from_local_path(recipes_dir, input_path, recipe_kind, options)
            .map(ImportResult::Single);
    }
    if is_git_like_target(input) {
        // Try local path detection first (file:// URLs pointing to dirs)
        let local_source_dir = source_dir_for_detection(input);
        // If no local dir, shallow-clone into a temp dir for strategy probing
        let probe_dir = if local_source_dir.is_none() {
            shallow_clone_for_probe(input).ok()
        } else {
            None
        };
        let probe_checkout = probe_dir.as_ref().map(|d| d.path().join("src"));
        let effective_source_dir = local_source_dir.as_deref().or(probe_checkout.as_deref());

        if let Some(source_dir) = effective_source_dir
            && let Some(snapshot_kind) = super::snapshot::detect_snapshot_type(source_dir)
        {
            let result = super::workflow_snapshot::import_snapshot(
                recipes_dir,
                input,
                source_dir,
                snapshot_kind,
                options,
            )
            .map(ImportResult::Bulk);
            // probe_dir drops here
            return result;
        }

        let result = scaffold_from_source(
            recipes_dir,
            infer_recipe_name(input),
            Some(input),
            recipe_kind,
            effective_source_dir,
            options,
        )
        .map(ImportResult::Single);
        // probe_dir drops here, cleaning up the temp directory
        return result;
    }

    scaffold_from_source(
        recipes_dir,
        input.to_owned(),
        None,
        recipe_kind,
        None,
        options,
    )
    .map(ImportResult::Single)
}

fn import_from_local_path(
    recipes_dir: &Path,
    source_dir: &Path,
    recipe_kind: &str,
    options: &ImportOptions,
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
    report.source_options = source_options_with_priority(
        Some(source_dir),
        discover_source_url(source_dir).as_deref(),
        options,
    );
    apply_source_option_selection(&mut report.source_options, options.selected_source_option)?;
    report.selected_source_option = selected_source_option(&report.source_options);

    copy_recipe_inputs(
        &source_pkg_lua,
        &source_build_lua,
        &source_patches,
        &recipe_dir,
        options.replace,
        &mut report,
    )?;
    ensure_recipe_pkg_lua(
        &recipe_dir,
        source_dir,
        &legacy_pkgdeps,
        recipe_kind,
        options,
        &mut report,
    )?;
    import_legacy_files(
        source_dir,
        &recipe_dir,
        &source_pkgdeps,
        &source_bldit,
        legacy_pkgdeps,
        options.replace,
        &mut report,
    )?;

    Ok(report)
}

fn scaffold_from_source(
    recipes_dir: &Path,
    recipe_name: String,
    source_url: Option<&str>,
    recipe_kind: &str,
    source_dir: Option<&Path>,
    options: &ImportOptions,
) -> Result<ImportReport, RecipeError> {
    let recipe_dir = recipes_dir.join(&recipe_name);
    fs::create_dir_all(&recipe_dir)?;

    let pkg_lua_path = recipe_dir.join("pkg.lua");
    let mut source_options = source_options_with_priority(source_dir, source_url, options);
    apply_source_option_selection(&mut source_options, options.selected_source_option)?;
    let selected_source_option = selected_source_option(&source_options);
    let wrote_pkg_lua = options.replace || !pkg_lua_path.exists();
    if wrote_pkg_lua {
        let strategy_priority =
            selected_strategy_priority(&options.strategy_priority, &selected_source_option);
        let strategy = detect_source_strategy_for_source(
            source_dir,
            source_url,
            &strategy_priority,
            &options.release_binary_format_priority,
        );
        fs::write(
            &pkg_lua_path,
            super::workflow_render::render_generated_pkg_lua(
                &recipe_name,
                source_url,
                source_dir,
                &[],
                recipe_kind,
                &strategy,
                options,
            ),
        )?;
    }

    Ok(ImportReport {
        recipe_name,
        recipe_dir,
        source_options,
        selected_source_option,
        imported_pkg_lua: false,
        imported_build_lua: false,
        imported_patches: false,
        generated_pkg_lua: wrote_pkg_lua,
        generated_build_lua: false,
        imported_legacy_pkgdeps: false,
        imported_legacy_bldit: false,
        wrote_legacy_summary: false,
    })
}

fn apply_source_option_selection(
    source_options: &mut [super::model::SourceOptionReport],
    selected_index: Option<usize>,
) -> Result<(), RecipeError> {
    let Some(index) = selected_index else {
        return Ok(());
    };
    if !source_options.iter().any(|option| option.index == index) {
        return Err(RecipeError::InvalidInput(format!(
            "source option `{index}` is not available for this input"
        )));
    }
    select_source_option_by_index(source_options, index);
    Ok(())
}

fn selected_strategy_priority(
    priority: &[String],
    selected: &Option<super::model::SourceOptionReport>,
) -> Vec<String> {
    let Some(selected) = selected else {
        return priority.to_vec();
    };
    let mut ordered = vec![selected.strategy.clone()];
    ordered.extend(
        priority
            .iter()
            .filter(|strategy| strategy.as_str() != selected.strategy)
            .cloned(),
    );
    ordered
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
        source_options: Vec::new(),
        selected_source_option: None,
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
    replace: bool,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    if source_pkg_lua.is_file() {
        report.imported_pkg_lua =
            copy_file_if_allowed(source_pkg_lua, &recipe_dir.join("pkg.lua"), replace)?;
    }
    if source_build_lua.is_file() {
        report.imported_build_lua =
            copy_file_if_allowed(source_build_lua, &recipe_dir.join("build.lua"), replace)?;
    }
    if source_patches.is_dir() {
        report.imported_patches =
            copy_dir_if_allowed(source_patches, &recipe_dir.join("patches"), replace)?;
    }

    Ok(())
}

fn ensure_recipe_pkg_lua(
    recipe_dir: &Path,
    source_dir: &Path,
    legacy_pkgdeps: &Option<Vec<super::model::LegacyPkgdep>>,
    recipe_kind: &str,
    options: &ImportOptions,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    let pkg_lua_path = recipe_dir.join("pkg.lua");
    if report.imported_pkg_lua || (pkg_lua_path.exists() && !options.replace) {
        return Ok(());
    }

    let source_url = discover_source_url(source_dir);
    let strategy_priority =
        selected_strategy_priority(&options.strategy_priority, &report.selected_source_option);
    let strategy = detect_source_strategy_for_source(
        Some(source_dir),
        source_url.as_deref(),
        &strategy_priority,
        &options.release_binary_format_priority,
    );
    fs::write(
        pkg_lua_path,
        super::workflow_render::render_generated_pkg_lua(
            &report.recipe_name,
            source_url.as_deref(),
            Some(source_dir),
            legacy_pkgdeps.as_deref().unwrap_or(&[]),
            recipe_kind,
            &strategy,
            options,
        ),
    )?;
    report.generated_pkg_lua = true;

    Ok(())
}

fn normalize_recipe_kind(recipe_kind: Option<&str>) -> Result<&str, RecipeError> {
    let recipe_kind = recipe_kind.unwrap_or("normal");
    if recipe_kind == "normal" || recipe_kind == "meta" || recipe_kind == "profile" {
        return Ok(recipe_kind);
    }

    Err(RecipeError::InvalidInput(format!(
        "unsupported recipe kind `{recipe_kind}`; expected one of: normal, meta, profile"
    )))
}

fn import_legacy_files(
    source_dir: &Path,
    recipe_dir: &Path,
    source_pkgdeps: &Path,
    source_bldit: &Path,
    legacy_pkgdeps: Option<Vec<super::model::LegacyPkgdep>>,
    replace: bool,
    report: &mut ImportReport,
) -> Result<(), RecipeError> {
    if !source_pkgdeps.is_file() && !source_bldit.is_file() {
        return Ok(());
    }

    let legacy_dir = recipe_dir.join("legacy");
    fs::create_dir_all(&legacy_dir)?;

    if source_pkgdeps.is_file() {
        report.imported_legacy_pkgdeps =
            copy_file_if_allowed(source_pkgdeps, &legacy_dir.join("pkgdeps"), replace)?;
    }

    if source_bldit.is_file() {
        report.imported_legacy_bldit =
            copy_file_if_allowed(source_bldit, &legacy_dir.join("pkgit.bldit"), replace)?;
        if !report.imported_build_lua {
            report.generated_build_lua = write_file_if_allowed(
                &recipe_dir.join("build.lua"),
                &render_imported_build_lua(&report.recipe_name),
                replace,
            )?;
        }
    }

    report.wrote_legacy_summary = write_file_if_allowed(
        &legacy_dir.join("pkgit-import.json"),
        &serde_json::to_string_pretty(&json!({
            "source_path": source_dir,
            "source_url": discover_source_url(source_dir),
            "pkgdeps": legacy_pkgdeps,
            "has_bldit": source_bldit.is_file(),
        }))?,
        replace,
    )?;

    Ok(())
}

fn copy_file_if_allowed(
    source: &Path,
    destination: &Path,
    replace: bool,
) -> Result<bool, RecipeError> {
    if destination.exists() && !replace {
        return Ok(false);
    }

    fs::copy(source, destination)?;
    Ok(true)
}

fn copy_dir_if_allowed(
    source: &Path,
    destination: &Path,
    replace: bool,
) -> Result<bool, RecipeError> {
    if destination.exists() {
        if !replace {
            return Ok(false);
        }
        fs::remove_dir_all(destination)?;
    }

    copy_dir_recursive(source, destination)?;
    Ok(true)
}

fn write_file_if_allowed(path: &Path, contents: &str, replace: bool) -> Result<bool, RecipeError> {
    if path.exists() && !replace {
        return Ok(false);
    }

    fs::write(path, contents)?;
    Ok(true)
}

/// Shallow-clone a git URL into a temporary directory for strategy and
/// metadata detection. The clone is `--depth 1` to minimize bandwidth.
/// Returns the TempDir (caller must keep it alive until detection is done).
fn shallow_clone_for_probe(url: &str) -> Result<tempfile::TempDir, RecipeError> {
    let temp = tempfile::TempDir::new()
        .map_err(|e| RecipeError::InvalidInput(format!("failed to create temp dir: {e}")))?;
    let checkout_dir = temp.path().join("src");

    let output = std::process::Command::new("git")
        .args(["clone", "--depth", "1", "--single-branch", url])
        .arg(&checkout_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| RecipeError::InvalidInput(format!("git clone probe failed: {e}")))?;

    if !output.status.success() {
        return Err(RecipeError::InvalidInput(format!(
            "git clone probe failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(temp)
}
