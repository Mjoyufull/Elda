use serde::Serialize;

use crate::app_profile::{PendingSystemChange, ProviderFamilies};
use elda_build::{ProviderAsset, SystemPackageMetadata};
use elda_recipe::{LuaValue, RecipeDocument};

#[derive(Debug, Clone, Serialize)]
struct ProviderAssetVisibility {
    provider_families: ProviderFamilies,
    pending_provider_handlers: Vec<PendingSystemChange>,
    declared_provider_assets: Vec<DeclaredProviderAssetVisibility>,
    declarative_assets: DeclarativeAssetVisibility,
    installed_system_assets: Option<InstalledSystemAssetVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct DeclaredProviderAssetVisibility {
    family: String,
    provider: String,
    kind: String,
    target: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct DeclarativeAssetVisibility {
    sysusers: Option<DeclarativeFamilyVisibility>,
    tmpfiles: Option<DeclarativeFamilyVisibility>,
    alternatives: Vec<AlternativeVisibility>,
    hooks: Vec<HookVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct DeclarativeFamilyVisibility {
    kind: String,
    entry_count: Option<usize>,
    file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AlternativeVisibility {
    name: String,
    link: String,
    path: String,
    priority: i64,
}

#[derive(Debug, Clone, Serialize)]
struct HookVisibility {
    phase: String,
    source_kind: String,
    file: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct InstalledSystemAssetVisibility {
    sysusers: Option<InstalledDeclarativeAssetVisibility>,
    tmpfiles: Option<InstalledDeclarativeAssetVisibility>,
    alternatives: Vec<AlternativeVisibility>,
    hooks: Vec<InstalledHookVisibility>,
    provider_assets: Vec<InstalledProviderAssetVisibility>,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledDeclarativeAssetVisibility {
    path: String,
    source: String,
    line_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledHookVisibility {
    phase: String,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
struct InstalledProviderAssetVisibility {
    family: String,
    provider: String,
    kind: String,
    target: String,
    stored_path: String,
    active: bool,
    tree_entry_count: usize,
}

pub(super) fn build_provider_asset_visibility(
    recipe: Option<&RecipeDocument>,
    installed_system_assets: Option<&SystemPackageMetadata>,
    provider_families: &ProviderFamilies,
    pending_handlers: &[PendingSystemChange],
) -> serde_json::Value {
    serde_json::to_value(ProviderAssetVisibility {
        provider_families: provider_families.clone(),
        pending_provider_handlers: pending_handlers
            .iter()
            .filter(|change| {
                matches!(
                    change.kind,
                    "profile-provider-reconciliation" | "init-provider-transition"
                )
            })
            .cloned()
            .collect(),
        declared_provider_assets: recipe
            .map(describe_declared_provider_assets)
            .unwrap_or_default(),
        declarative_assets: recipe.map(describe_recipe_assets).unwrap_or_default(),
        installed_system_assets: installed_system_assets
            .map(|assets| describe_installed_system_assets(assets, provider_families)),
    })
    .unwrap_or_else(|_| serde_json::json!({}))
}

fn describe_declared_provider_assets(
    recipe: &RecipeDocument,
) -> Vec<DeclaredProviderAssetVisibility> {
    let Some(LuaValue::Table(families)) = recipe.package.provider_assets.as_ref() else {
        return Vec::new();
    };

    let mut assets = Vec::new();
    for (family, providers) in families {
        let LuaValue::Table(providers) = providers else {
            continue;
        };
        for (provider, entries) in providers {
            let LuaValue::Array(entries) = entries else {
                continue;
            };
            for entry in entries {
                let LuaValue::Table(table) = entry else {
                    continue;
                };
                assets.push(DeclaredProviderAssetVisibility {
                    family: family.clone(),
                    provider: provider.clone(),
                    kind: table
                        .get("kind")
                        .and_then(as_non_empty_string)
                        .unwrap_or_default()
                        .to_owned(),
                    target: table
                        .get("target")
                        .and_then(as_non_empty_string)
                        .unwrap_or_default()
                        .to_owned(),
                });
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
    assets
}

fn describe_recipe_assets(recipe: &RecipeDocument) -> DeclarativeAssetVisibility {
    DeclarativeAssetVisibility {
        sysusers: summarize_declarative_family(recipe.package.sysusers.as_ref()),
        tmpfiles: summarize_declarative_family(recipe.package.tmpfiles.as_ref()),
        alternatives: summarize_alternatives(recipe.package.alternatives.as_ref()),
        hooks: summarize_hooks(recipe.package.hooks.as_ref()),
    }
}

fn summarize_declarative_family(value: Option<&LuaValue>) -> Option<DeclarativeFamilyVisibility> {
    match value {
        Some(LuaValue::Array(entries)) => Some(DeclarativeFamilyVisibility {
            kind: "inline".to_owned(),
            entry_count: Some(entries.len()),
            file: None,
        }),
        Some(LuaValue::Table(table)) => Some(DeclarativeFamilyVisibility {
            kind: "file".to_owned(),
            entry_count: None,
            file: table
                .get("file")
                .and_then(as_non_empty_string)
                .map(ToOwned::to_owned),
        }),
        _ => None,
    }
}

fn summarize_alternatives(value: Option<&LuaValue>) -> Vec<AlternativeVisibility> {
    let Some(LuaValue::Array(entries)) = value else {
        return Vec::new();
    };

    entries
        .iter()
        .filter_map(|entry| {
            let LuaValue::Table(table) = entry else {
                return None;
            };
            Some(AlternativeVisibility {
                name: table
                    .get("name")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                link: table
                    .get("link")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                path: table
                    .get("path")
                    .and_then(as_non_empty_string)
                    .unwrap_or_default()
                    .to_owned(),
                priority: match table.get("priority") {
                    Some(LuaValue::Integer(value)) => *value,
                    _ => 0,
                },
            })
        })
        .collect()
}

fn summarize_hooks(value: Option<&LuaValue>) -> Vec<HookVisibility> {
    let Some(LuaValue::Table(hooks)) = value else {
        return Vec::new();
    };
    let mut summarized = hooks
        .iter()
        .filter_map(|(phase, spec)| {
            let LuaValue::Table(table) = spec else {
                return None;
            };
            Some(HookVisibility {
                phase: phase.clone(),
                source_kind: if table.get("file").and_then(as_non_empty_string).is_some() {
                    "file".to_owned()
                } else {
                    "lua".to_owned()
                },
                file: table
                    .get("file")
                    .and_then(as_non_empty_string)
                    .map(ToOwned::to_owned),
            })
        })
        .collect::<Vec<_>>();
    summarized.sort_by(|left, right| left.phase.cmp(&right.phase));
    summarized
}

fn describe_installed_system_assets(
    assets: &SystemPackageMetadata,
    provider_families: &ProviderFamilies,
) -> InstalledSystemAssetVisibility {
    InstalledSystemAssetVisibility {
        sysusers: assets
            .sysusers
            .as_ref()
            .map(|asset| InstalledDeclarativeAssetVisibility {
                path: asset.path.clone(),
                source: asset.source.clone(),
                line_count: non_empty_line_count(&asset.content),
            }),
        tmpfiles: assets
            .tmpfiles
            .as_ref()
            .map(|asset| InstalledDeclarativeAssetVisibility {
                path: asset.path.clone(),
                source: asset.source.clone(),
                line_count: non_empty_line_count(&asset.content),
            }),
        alternatives: assets
            .alternatives
            .iter()
            .map(|alternative| AlternativeVisibility {
                name: alternative.name.clone(),
                link: alternative.link.clone(),
                path: alternative.path.clone(),
                priority: alternative.priority,
            })
            .collect(),
        hooks: assets
            .hooks
            .iter()
            .map(|hook| InstalledHookVisibility {
                phase: hook.phase.clone(),
                source: hook.source.clone(),
            })
            .collect(),
        provider_assets: describe_installed_provider_assets(
            &assets.provider_assets,
            provider_families,
        ),
    }
}

fn describe_installed_provider_assets(
    assets: &[ProviderAsset],
    provider_families: &ProviderFamilies,
) -> Vec<InstalledProviderAssetVisibility> {
    let mut summarized = assets
        .iter()
        .map(|asset| InstalledProviderAssetVisibility {
            family: asset.family.clone(),
            provider: asset.provider.clone(),
            kind: asset.kind.clone(),
            target: asset.target.clone(),
            stored_path: asset.stored_path.clone(),
            active: asset.family == "init" && asset.provider == provider_families.init,
            tree_entry_count: asset.tree_entries.len(),
        })
        .collect::<Vec<_>>();
    summarized.sort_by(|left, right| {
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
    summarized
}

fn non_empty_line_count(content: &str) -> usize {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn as_non_empty_string(value: &LuaValue) -> Option<&str> {
    match value {
        LuaValue::String(value) if !value.trim().is_empty() => Some(value.as_str()),
        _ => None,
    }
}
