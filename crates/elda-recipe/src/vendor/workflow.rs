use std::fs;
use std::path::Path;

use crate::error::RecipeError;
use crate::{SOURCE_LANE_BINARY, load_recipe};

use super::model::{
    ResolvedVendorSource, VendorExportReport, VendorImportReport, VendorLockEntry, VendorLockFile,
    VendorRecipeReport,
};
use super::render::{render_vendor_manifest_line, render_vendor_pkg_lua};
use super::source::{parse_vendor_manifest_line, resolve_vendor_source};

pub fn add_vendor_recipe(
    recipes_dir: &Path,
    package_name: &str,
    source: &str,
    binary: Option<&str>,
    asset: Option<&str>,
) -> Result<VendorRecipeReport, RecipeError> {
    fs::create_dir_all(recipes_dir)?;
    let resolved = resolve_vendor_source(source, binary, asset, package_name)?;
    write_vendor_recipe(recipes_dir, package_name, resolved)
}

pub fn import_vendor_source(
    recipes_dir: &Path,
    input_path: &Path,
) -> Result<VendorImportReport, RecipeError> {
    let content = fs::read_to_string(input_path)?;
    let format = input_format(input_path);

    let packages = if format == "lock-json" {
        let lock = serde_json::from_str::<VendorLockFile>(&content)?;
        lock.entries
            .into_iter()
            .map(|entry| add_vendor_recipe_from_lock(recipes_dir, entry))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        content
            .lines()
            .filter_map(trim_manifest_line)
            .map(parse_vendor_manifest_line)
            .map(|parsed| {
                let parsed = parsed?;
                add_vendor_recipe(
                    recipes_dir,
                    &parsed.package_name,
                    &parsed.source,
                    parsed.binary.as_deref(),
                    parsed.asset.as_deref(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(VendorImportReport {
        source_path: input_path.to_path_buf(),
        format: format.to_owned(),
        packages,
    })
}

pub fn export_vendor_source(
    recipes_dir: &Path,
    output_path: &Path,
    package_names: &[String],
) -> Result<VendorExportReport, RecipeError> {
    let entries = package_names
        .iter()
        .map(|package_name| lock_entry_from_recipe(recipes_dir, package_name))
        .collect::<Result<Vec<_>, _>>()?;

    let format = if input_format(output_path) == "lock-json" {
        let lock = VendorLockFile {
            version: 1,
            entries,
        };
        fs::write(output_path, serde_json::to_string_pretty(&lock)?)?;
        "lock-json"
    } else {
        let manifest = entries
            .iter()
            .map(render_vendor_manifest_line)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(output_path, format!("{manifest}\n"))?;
        "manifest"
    };

    Ok(VendorExportReport {
        output_path: output_path.to_path_buf(),
        format: format.to_owned(),
        packages: package_names.to_vec(),
    })
}

fn add_vendor_recipe_from_lock(
    recipes_dir: &Path,
    entry: VendorLockEntry,
) -> Result<VendorRecipeReport, RecipeError> {
    let resolved = match entry.source_kind.as_str() {
        "url_archive" => ResolvedVendorSource::UrlArchive {
            url: entry.url.ok_or_else(|| {
                RecipeError::InvalidInput("vendor lock entry is missing `url`".to_owned())
            })?,
            sha256: entry.sha256,
            binary: entry.binary,
            rename: entry.rename,
        },
        "github_release" => ResolvedVendorSource::GitHubRelease {
            repo: entry.repo.ok_or_else(|| {
                RecipeError::InvalidInput("vendor lock entry is missing `repo`".to_owned())
            })?,
            tag: entry.tag.ok_or_else(|| {
                RecipeError::InvalidInput("vendor lock entry is missing `tag`".to_owned())
            })?,
            asset: entry.asset.ok_or_else(|| {
                RecipeError::InvalidInput("vendor lock entry is missing `asset`".to_owned())
            })?,
            sha256: entry.sha256,
            binary: entry.binary,
            rename: entry.rename,
        },
        other => {
            return Err(RecipeError::InvalidInput(format!(
                "unsupported vendor lock source_kind `{other}`"
            )));
        }
    };
    write_vendor_recipe(recipes_dir, &entry.package_name, resolved)
}

fn lock_entry_from_recipe(
    recipes_dir: &Path,
    package_name: &str,
) -> Result<VendorLockEntry, RecipeError> {
    let recipe = load_recipe(recipes_dir, package_name)?;
    let source = recipe
        .package
        .source
        .lane_definition(SOURCE_LANE_BINARY)
        .or_else(|| {
            if recipe.package.source.kind == "url_archive"
                || recipe.package.source.kind == "github_release"
            {
                Some(crate::SourceLaneDefinition {
                    kind: recipe.package.source.kind.clone(),
                    fields: recipe.package.source.fields.clone(),
                    github_release_assets: recipe.package.source.github_release_assets.clone(),
                })
            } else {
                None
            }
        })
        .ok_or_else(|| {
            RecipeError::InvalidInput(format!(
                "recipe `{package_name}` does not contain an exportable binary vendor source"
            ))
        })?;

    let string_field = |key: &str| match source.fields.get(key) {
        Some(crate::ScalarValue::String(value)) => Some(value.clone()),
        _ => None,
    };

    Ok(VendorLockEntry {
        package_name: package_name.to_owned(),
        source_kind: source.kind.clone(),
        url: string_field("url"),
        repo: string_field("repo"),
        tag: string_field("tag"),
        asset: string_field("asset"),
        sha256: string_field("sha256").ok_or_else(|| {
            RecipeError::InvalidInput(format!(
                "recipe `{package_name}` binary source is missing `sha256`"
            ))
        })?,
        binary: string_field("binary"),
        rename: string_field("rename"),
    })
}

fn write_vendor_recipe(
    recipes_dir: &Path,
    package_name: &str,
    resolved: ResolvedVendorSource,
) -> Result<VendorRecipeReport, RecipeError> {
    let recipe_dir = recipes_dir.join(package_name);
    fs::create_dir_all(&recipe_dir)?;
    fs::write(
        recipe_dir.join("pkg.lua"),
        render_vendor_pkg_lua(package_name, &resolved),
    )?;

    Ok(VendorRecipeReport {
        package_name: package_name.to_owned(),
        recipe_dir,
        source_kind: resolved.source_kind().to_owned(),
        source_url: resolved.source_url(),
        sha256: resolved.sha256().to_owned(),
        asset: resolved.asset_name().map(ToOwned::to_owned),
        binary: resolved.binary_name().map(ToOwned::to_owned),
    })
}

fn input_format(path: &Path) -> &'static str {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        "lock-json"
    } else {
        "manifest"
    }
}

fn trim_manifest_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        None
    } else {
        Some(trimmed)
    }
}
