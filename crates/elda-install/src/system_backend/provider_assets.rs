use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

use elda_build::{ProviderAsset, ProviderTreeEntry, SystemPackageMetadata};
use elda_db::StateLayout;

use crate::InstallError;
use crate::fsops::{remove_existing_path, strip_leading_slash};

pub(crate) fn active_provider_families(
    layout: &StateLayout,
) -> Result<BTreeMap<String, String>, InstallError> {
    let mut families = BTreeMap::new();
    let init = super::load_applied_profile_init(layout)?;
    if !init.trim().is_empty() {
        families.insert("init".to_owned(), init);
    }

    Ok(families)
}

pub(crate) fn materialize_provider_assets_under_root(
    root: &Path,
    packages: &BTreeMap<String, SystemPackageMetadata>,
    active_families: &BTreeMap<String, String>,
) -> Result<(), InstallError> {
    for metadata in packages.values() {
        for asset in &metadata.provider_assets {
            write_provider_asset_store(root, asset)?;
        }
    }

    for asset in collect_active_provider_assets(packages, active_families)?.into_values() {
        write_provider_asset_target(root, asset)?;
    }

    Ok(())
}

pub(crate) fn provider_storage_paths(metadata: &SystemPackageMetadata) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for asset in &metadata.provider_assets {
        match asset.kind.as_str() {
            "file" => {
                paths.insert(asset.stored_path.clone());
            }
            "tree" => {
                paths.insert(asset.stored_path.clone());
                for entry in &asset.tree_entries {
                    paths.insert(join_logical_path(&asset.stored_path, &entry.relative_path));
                }
            }
            _ => {}
        }
    }

    paths
}

pub(crate) fn all_provider_target_paths(metadata: &SystemPackageMetadata) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for asset in &metadata.provider_assets {
        match asset.kind.as_str() {
            "file" => {
                paths.insert(asset.target.clone());
            }
            "tree" => {
                paths.insert(asset.target.clone());
                for entry in &asset.tree_entries {
                    paths.insert(join_logical_path(&asset.target, &entry.relative_path));
                }
            }
            _ => {}
        }
    }

    paths
}

pub(crate) fn active_provider_paths(
    metadata: &SystemPackageMetadata,
    active_families: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for asset in metadata.provider_assets.iter().filter(|asset| {
        active_families
            .get(&asset.family)
            .is_some_and(|provider| provider == &asset.provider)
    }) {
        match asset.kind.as_str() {
            "file" => {
                paths.insert(asset.target.clone());
            }
            "tree" => {
                paths.insert(asset.target.clone());
                for entry in &asset.tree_entries {
                    paths.insert(join_logical_path(&asset.target, &entry.relative_path));
                }
            }
            _ => {}
        }
    }

    paths
}

pub(crate) fn remove_provider_paths(
    layout: &StateLayout,
    paths: BTreeSet<String>,
) -> Result<(), InstallError> {
    let mut sorted = paths.into_iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        path_depth(right)
            .cmp(&path_depth(left))
            .then_with(|| right.cmp(left))
    });

    for path in sorted {
        let live = logical_path(layout.root_dir.as_path(), &path)?;
        if live.exists() || live.is_symlink() {
            remove_existing_path(&live)?;
        }
    }

    Ok(())
}

pub(crate) fn clear_provider_storage_root(layout: &StateLayout) -> Result<(), InstallError> {
    let root = logical_path(layout.root_dir.as_path(), "/usr/lib/elda/provider-assets")?;
    if root.exists() || root.is_symlink() {
        remove_existing_path(&root)?;
    }
    Ok(())
}

fn collect_active_provider_assets<'a>(
    packages: &'a BTreeMap<String, SystemPackageMetadata>,
    active_families: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, &'a ProviderAsset>, InstallError> {
    let mut assets = BTreeMap::<String, (&str, &ProviderAsset)>::new();
    for (package_name, metadata) in packages {
        for asset in &metadata.provider_assets {
            if active_families
                .get(&asset.family)
                .is_none_or(|provider| provider != &asset.provider)
            {
                continue;
            }
            if let Some((owner, _)) = assets.insert(asset.target.clone(), (package_name, asset)) {
                return Err(InstallError::PathConflict {
                    path: asset.target.clone(),
                    owner: owner.to_owned(),
                });
            }
        }
    }

    Ok(assets
        .into_iter()
        .map(|(target, (_, asset))| (target, asset))
        .collect())
}

fn write_provider_asset_store(root: &Path, asset: &ProviderAsset) -> Result<(), InstallError> {
    match asset.kind.as_str() {
        "file" => write_provider_file(root, &asset.stored_path, &asset.content, asset.mode),
        "tree" => write_provider_tree(root, &asset.stored_path, &asset.tree_entries),
        other => Err(InstallError::Unsupported(format!(
            "unsupported provider asset kind `{other}`"
        ))),
    }
}

fn write_provider_asset_target(root: &Path, asset: &ProviderAsset) -> Result<(), InstallError> {
    match asset.kind.as_str() {
        "file" => write_provider_file(root, &asset.target, &asset.content, asset.mode),
        "tree" => write_provider_tree(root, &asset.target, &asset.tree_entries),
        other => Err(InstallError::Unsupported(format!(
            "unsupported provider asset kind `{other}`"
        ))),
    }
}

fn write_provider_file(
    root: &Path,
    logical: &str,
    content: &[u8],
    mode: Option<u32>,
) -> Result<(), InstallError> {
    let target = logical_path(root, logical)?;
    if target.exists() || target.is_symlink() {
        return Err(InstallError::UnmanagedPathCollision(logical.to_owned()));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, content)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(mode.unwrap_or(0o644)))?;
    Ok(())
}

fn write_provider_tree(
    root: &Path,
    logical_root: &str,
    entries: &[ProviderTreeEntry],
) -> Result<(), InstallError> {
    let root_path = logical_path(root, logical_root)?;
    if root_path.exists() || root_path.is_symlink() {
        return Err(InstallError::UnmanagedPathCollision(
            logical_root.to_owned(),
        ));
    }
    fs::create_dir_all(&root_path)?;

    for entry in entries {
        let logical = join_logical_path(logical_root, &entry.relative_path);
        let target = logical_path(root, &logical)?;
        if target.exists() || target.is_symlink() {
            return Err(InstallError::UnmanagedPathCollision(logical));
        }
        match entry.entry_kind.as_str() {
            "dir" => {
                fs::create_dir_all(&target)?;
                fs::set_permissions(&target, fs::Permissions::from_mode(entry.mode))?;
            }
            "file" => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, &entry.content)?;
                fs::set_permissions(&target, fs::Permissions::from_mode(entry.mode))?;
            }
            "symlink" => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                symlink(
                    entry.link_target.as_deref().ok_or_else(|| {
                        InstallError::Unsupported(format!(
                            "provider asset `{logical}` is missing its symlink target"
                        ))
                    })?,
                    &target,
                )?;
            }
            other => {
                return Err(InstallError::Unsupported(format!(
                    "unsupported provider tree entry kind `{other}`"
                )));
            }
        }
    }

    Ok(())
}

fn logical_path(root: &Path, logical: &str) -> Result<PathBuf, InstallError> {
    Ok(root.join(strip_leading_slash(logical)?))
}

fn join_logical_path(root: &str, relative: &str) -> String {
    if relative.is_empty() {
        root.to_owned()
    } else {
        format!("{}/{}", root.trim_end_matches('/'), relative)
    }
}

fn path_depth(path: &str) -> usize {
    path.trim_matches('/').split('/').count()
}
