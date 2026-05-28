use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use elda_recipe::{LuaValue, RecipeDocument};

use crate::BuildError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAsset {
    pub family: String,
    pub provider: String,
    pub kind: String,
    pub target: String,
    pub stored_path: String,
    pub mode: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tree_entries: Vec<ProviderTreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTreeEntry {
    pub relative_path: String,
    pub entry_kind: String,
    pub mode: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_target: Option<String>,
}

pub fn collect_provider_assets(recipe: &RecipeDocument) -> Result<Vec<ProviderAsset>, BuildError> {
    let Some(LuaValue::Table(families)) = recipe.package.provider_assets.as_ref() else {
        return Ok(Vec::new());
    };

    let mut assets = Vec::new();
    for (family, providers) in families {
        let LuaValue::Table(providers) = providers else {
            return Err(BuildError::Invalid(format!(
                "provider_assets.{family} for `{}` is invalid",
                recipe.package.name
            )));
        };
        for (provider, entries) in providers {
            let LuaValue::Array(entries) = entries else {
                return Err(BuildError::Invalid(format!(
                    "provider_assets.{family}.{provider} for `{}` is invalid",
                    recipe.package.name
                )));
            };
            let store_base = format!(
                "/usr/lib/elda/provider-assets/{family}/{provider}/{}",
                recipe.package.name
            );
            for (index, entry) in entries.iter().enumerate() {
                assets.push(parse_provider_asset(
                    recipe,
                    family,
                    provider,
                    index,
                    &store_base,
                    entry,
                )?);
            }
        }
    }

    assets.sort_by(|left, right| {
        (
            left.family.as_str(),
            left.provider.as_str(),
            left.target.as_str(),
        )
            .cmp(&(
                right.family.as_str(),
                right.provider.as_str(),
                right.target.as_str(),
            ))
    });
    Ok(assets)
}

fn parse_provider_asset(
    recipe: &RecipeDocument,
    family: &str,
    provider: &str,
    index: usize,
    store_base: &str,
    value: &LuaValue,
) -> Result<ProviderAsset, BuildError> {
    let LuaValue::Table(table) = value else {
        return Err(BuildError::Invalid(format!(
            "provider_assets.{family}.{provider} entry `{index}` for `{}` must be a table",
            recipe.package.name
        )));
    };

    let kind = required_string(table, "kind")?;
    let target = required_string(table, "target")?;

    match kind.as_str() {
        "file" => parse_file_asset(recipe, family, provider, index, store_base, table, target),
        "tree" => parse_tree_asset(recipe, family, provider, index, store_base, table, target),
        _ => Err(BuildError::Invalid(format!(
            "provider_assets.{family}.{provider} entry `{index}` for `{}` uses unsupported kind `{kind}`",
            recipe.package.name
        ))),
    }
}

fn parse_file_asset(
    recipe: &RecipeDocument,
    family: &str,
    provider: &str,
    index: usize,
    store_base: &str,
    table: &BTreeMap<String, LuaValue>,
    target: String,
) -> Result<ProviderAsset, BuildError> {
    let stored_path = format!("{store_base}/{index}.asset");
    let explicit_mode = parse_mode(table.get("mode"))?;
    let base_dir = recipe.path.parent().ok_or_else(|| {
        BuildError::Invalid(format!(
            "recipe path for `{}` has no parent directory",
            recipe.package.name
        ))
    })?;

    let file = table.get("file").and_then(as_non_empty_string);
    let text = table.get("text").and_then(as_non_empty_string);
    let (content, mode) = match (file, text) {
        (Some(relative), None) => {
            let path = base_dir.join(relative);
            let bytes = fs::read(&path).map_err(|error| {
                BuildError::Invalid(format!(
                    "provider_assets.{family}.{provider} file `{}` for `{}` could not be read: {error}",
                    path.display(),
                    recipe.package.name
                ))
            })?;
            let inherited_mode = fs::metadata(&path)
                .map(|metadata| metadata.permissions().mode())
                .unwrap_or(0o644);
            (bytes, explicit_mode.or(Some(inherited_mode)))
        }
        (None, Some(text)) => (text.as_bytes().to_vec(), explicit_mode.or(Some(0o644))),
        _ => {
            return Err(BuildError::Invalid(format!(
                "provider_assets.{family}.{provider} file entries for `{}` must define exactly one of `file` or `text`",
                recipe.package.name
            )));
        }
    };

    Ok(ProviderAsset {
        family: family.to_owned(),
        provider: provider.to_owned(),
        kind: "file".to_owned(),
        target,
        stored_path,
        mode,
        content,
        tree_entries: Vec::new(),
    })
}

fn parse_tree_asset(
    recipe: &RecipeDocument,
    family: &str,
    provider: &str,
    index: usize,
    store_base: &str,
    table: &BTreeMap<String, LuaValue>,
    target: String,
) -> Result<ProviderAsset, BuildError> {
    let base_dir = recipe.path.parent().ok_or_else(|| {
        BuildError::Invalid(format!(
            "recipe path for `{}` has no parent directory",
            recipe.package.name
        ))
    })?;
    let relative_dir = required_string(table, "dir")?;
    let source_root = base_dir.join(&relative_dir);
    if !source_root.is_dir() {
        return Err(BuildError::Invalid(format!(
            "provider_assets.{family}.{provider} tree directory `{}` for `{}` does not exist",
            source_root.display(),
            recipe.package.name
        )));
    }

    let mut entries = Vec::new();
    collect_tree_entries(&source_root, &source_root, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    Ok(ProviderAsset {
        family: family.to_owned(),
        provider: provider.to_owned(),
        kind: "tree".to_owned(),
        target,
        stored_path: format!("{store_base}/{index}"),
        mode: None,
        content: Vec::new(),
        tree_entries: entries,
    })
}

fn collect_tree_entries(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ProviderTreeEntry>,
) -> Result<(), BuildError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| BuildError::Invalid(error.to_string()))?
            .display()
            .to_string();
        if metadata.file_type().is_symlink() {
            entries.push(ProviderTreeEntry {
                relative_path: relative,
                entry_kind: "symlink".to_owned(),
                mode: metadata.permissions().mode(),
                content: Vec::new(),
                link_target: Some(fs::read_link(&path)?.display().to_string()),
            });
            continue;
        }
        if metadata.is_dir() {
            entries.push(ProviderTreeEntry {
                relative_path: relative.clone(),
                entry_kind: "dir".to_owned(),
                mode: metadata.permissions().mode(),
                content: Vec::new(),
                link_target: None,
            });
            collect_tree_entries(root, &path, entries)?;
            continue;
        }
        entries.push(ProviderTreeEntry {
            relative_path: relative,
            entry_kind: "file".to_owned(),
            mode: metadata.permissions().mode(),
            content: fs::read(&path)?,
            link_target: None,
        });
    }

    Ok(())
}

fn required_string(table: &BTreeMap<String, LuaValue>, key: &str) -> Result<String, BuildError> {
    match table.get(key) {
        Some(LuaValue::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(BuildError::Invalid(format!(
            "missing non-empty `{key}` field"
        ))),
    }
}

fn parse_mode(value: Option<&LuaValue>) -> Result<Option<u32>, BuildError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(mode) = as_non_empty_string(value) else {
        return Err(BuildError::Invalid(
            "provider asset `mode` must be a non-empty string".to_owned(),
        ));
    };
    u32::from_str_radix(mode, 8).map(Some).map_err(|error| {
        BuildError::Invalid(format!(
            "provider asset mode `{mode}` is not a valid octal permission string: {error}"
        ))
    })
}

fn as_non_empty_string(value: &LuaValue) -> Option<&str> {
    match value {
        LuaValue::String(value) if !value.trim().is_empty() => Some(value.as_str()),
        _ => None,
    }
}
