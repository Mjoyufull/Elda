mod provider_assets;

use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};

use elda_recipe::{LuaValue, RecipeDocument};

use crate::BuildError;
pub use provider_assets::{ProviderAsset, ProviderTreeEntry, collect_provider_assets};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SystemPackageMetadata {
    pub sysusers: Option<DeclarativeAsset>,
    pub tmpfiles: Option<DeclarativeAsset>,
    pub alternatives: Vec<AlternativeAsset>,
    pub hooks: Vec<LifecycleHookAsset>,
    pub provider_assets: Vec<ProviderAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclarativeAsset {
    pub path: String,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlternativeAsset {
    pub name: String,
    pub link: String,
    pub path: String,
    pub priority: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleHookAsset {
    pub phase: String,
    pub source: String,
}

pub fn collect_system_metadata(
    recipe: &RecipeDocument,
) -> Result<SystemPackageMetadata, BuildError> {
    Ok(SystemPackageMetadata {
        sysusers: collect_declarative_asset(recipe, "sysusers", recipe.package.sysusers.as_ref())?,
        tmpfiles: collect_declarative_asset(recipe, "tmpfiles", recipe.package.tmpfiles.as_ref())?,
        alternatives: collect_alternatives(recipe.package.alternatives.as_ref())?,
        hooks: collect_hooks(recipe.package.hooks.as_ref())?,
        provider_assets: collect_provider_assets(recipe)?,
    })
}

fn collect_declarative_asset(
    recipe: &RecipeDocument,
    family: &str,
    value: Option<&LuaValue>,
) -> Result<Option<DeclarativeAsset>, BuildError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let path = format!("/usr/lib/elda/{family}.d/{}.conf", recipe.package.name);
    match value {
        LuaValue::Array(entries) => Ok(Some(DeclarativeAsset {
            path,
            content: render_inline_entries(entries),
            source: "inline".to_owned(),
        })),
        LuaValue::Table(table) => {
            let Some(LuaValue::String(relative_path)) = table.get("file") else {
                return Err(BuildError::Invalid(format!(
                    "{family} file-backed metadata for `{}` is invalid",
                    recipe.package.name
                )));
            };
            let base_dir = recipe.path.parent().ok_or_else(|| {
                BuildError::Invalid(format!(
                    "recipe path for `{}` has no parent directory",
                    recipe.package.name
                ))
            })?;
            let resolved = base_dir.join(relative_path);
            let content = fs::read_to_string(&resolved).map_err(|error| {
                BuildError::Invalid(format!(
                    "{family} metadata file `{}` for `{}` could not be read: {error}",
                    resolved.display(),
                    recipe.package.name
                ))
            })?;

            Ok(Some(DeclarativeAsset {
                path,
                content,
                source: format!("file:{relative_path}"),
            }))
        }
        _ => Err(BuildError::Invalid(format!(
            "{family} metadata for `{}` is invalid",
            recipe.package.name
        ))),
    }
}

fn collect_alternatives(value: Option<&LuaValue>) -> Result<Vec<AlternativeAsset>, BuildError> {
    let Some(LuaValue::Array(entries)) = value else {
        return Ok(Vec::new());
    };

    entries.iter().map(parse_alternative).collect()
}

fn parse_alternative(value: &LuaValue) -> Result<AlternativeAsset, BuildError> {
    let LuaValue::Table(table) = value else {
        return Err(BuildError::Invalid(
            "alternatives entries must be tables".to_owned(),
        ));
    };

    Ok(AlternativeAsset {
        name: required_string(table, "name")?,
        link: required_string(table, "link")?,
        path: required_string(table, "path")?,
        priority: required_integer(table, "priority")?,
    })
}

fn collect_hooks(value: Option<&LuaValue>) -> Result<Vec<LifecycleHookAsset>, BuildError> {
    let Some(LuaValue::Table(hooks)) = value else {
        return Ok(Vec::new());
    };

    let mut collected = Vec::new();
    for (phase, value) in hooks {
        let LuaValue::Table(spec) = value else {
            return Err(BuildError::Invalid(format!(
                "hook `{phase}` must use a table"
            )));
        };
        let source = if matches!(spec.get("file"), Some(LuaValue::String(path)) if !path.trim().is_empty())
        {
            "file"
        } else if matches!(spec.get("lua"), Some(LuaValue::String(chunk)) if !chunk.trim().is_empty())
        {
            "lua"
        } else {
            return Err(BuildError::Invalid(format!(
                "hook `{phase}` must define either `file` or `lua`"
            )));
        };
        collected.push(LifecycleHookAsset {
            phase: phase.clone(),
            source: source.to_owned(),
        });
    }
    collected.sort_by(|left, right| left.phase.cmp(&right.phase));

    Ok(collected)
}

fn render_inline_entries(entries: &[LuaValue]) -> String {
    let mut lines = Vec::with_capacity(entries.len());
    for entry in entries {
        lines.push(render_value(entry));
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn render_value(value: &LuaValue) -> String {
    match value {
        LuaValue::String(value) => value.clone(),
        LuaValue::Integer(value) => value.to_string(),
        LuaValue::Boolean(value) => value.to_string(),
        LuaValue::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(render_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        LuaValue::Table(table) => table
            .iter()
            .map(|(key, value)| format!("{key}={}", render_value(value)))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn required_string(table: &BTreeMap<String, LuaValue>, key: &str) -> Result<String, BuildError> {
    match table.get(key) {
        Some(LuaValue::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(BuildError::Invalid(format!(
            "missing non-empty `{key}` field"
        ))),
    }
}

fn required_integer(table: &BTreeMap<String, LuaValue>, key: &str) -> Result<i64, BuildError> {
    match table.get(key) {
        Some(LuaValue::Integer(value)) => Ok(*value),
        _ => Err(BuildError::Invalid(format!(
            "missing integer `{key}` field"
        ))),
    }
}
