use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::symlink;

use serde::{Deserialize, Serialize};

use elda_build::{BuiltPackage, SystemPackageMetadata};
use elda_db::{Database, InstallationMode, StateLayout};

use super::paths::{
    alternative_registry_path, package_metadata_dir, package_metadata_path, resolve_symlink_target,
    strip_leading_slash,
};
use super::provider_assets::{
    active_provider_families, active_provider_paths, all_provider_target_paths,
    clear_provider_storage_root, materialize_provider_assets_under_root, provider_storage_paths,
    remove_provider_paths,
};
use crate::InstallError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredPackageMetadata {
    package_name: String,
    system_metadata: SystemPackageMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct AlternativeRegistry {
    winners: BTreeMap<String, AlternativeWinner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AlternativeWinner {
    package_name: String,
    target: String,
}

pub(crate) fn reconcile_system_state_after_install(
    database: &Database,
    package: &BuiltPackage,
) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(());
    }

    write_package_metadata(
        database.layout(),
        &package.package_name,
        &package.system_metadata,
    )?;
    materialize_package_assets(
        database.layout(),
        &package.package_name,
        &package.system_metadata,
    )?;
    reconcile_provider_assets(database.layout())?;
    reconcile_alternatives(database.layout())
}

pub(crate) fn reconcile_system_state_after_remove(
    database: &Database,
    package_name: &str,
) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(());
    }

    let removed_provider_paths = load_installed_system_metadata(database.layout(), package_name)?
        .map(|metadata| {
            let mut paths = provider_storage_paths(&metadata);
            paths.extend(all_provider_target_paths(&metadata));
            paths
        })
        .unwrap_or_default();
    remove_package_metadata(database.layout(), package_name)?;
    remove_asset(
        database.layout(),
        &format!("/usr/lib/elda/sysusers.d/{package_name}.conf"),
    )?;
    remove_asset(
        database.layout(),
        &format!("/usr/lib/elda/tmpfiles.d/{package_name}.conf"),
    )?;
    remove_provider_paths(database.layout(), removed_provider_paths)?;
    reconcile_provider_assets(database.layout())?;
    reconcile_alternatives(database.layout())
}

pub(crate) fn active_system_paths(layout: &StateLayout) -> Result<BTreeSet<String>, InstallError> {
    if layout.mode != InstallationMode::System {
        return Ok(BTreeSet::new());
    }

    let mut paths = BTreeSet::new();
    let active_families = active_provider_families(layout)?;
    for record in load_all_package_metadata(layout)? {
        if let Some(asset) = record.system_metadata.sysusers.as_ref() {
            paths.insert(asset.path.clone());
        }
        if let Some(asset) = record.system_metadata.tmpfiles.as_ref() {
            paths.insert(asset.path.clone());
        }
        paths.extend(provider_storage_paths(&record.system_metadata));
        paths.extend(active_provider_paths(
            &record.system_metadata,
            &active_families,
        ));
    }
    paths.extend(load_alternative_registry(layout)?.winners.into_keys());

    Ok(paths)
}

pub fn load_installed_system_metadata(
    layout: &StateLayout,
    package_name: &str,
) -> Result<Option<SystemPackageMetadata>, InstallError> {
    let path = package_metadata_path(layout, package_name);
    if !path.exists() {
        return Ok(None);
    }

    let record = serde_json::from_slice::<StoredPackageMetadata>(&fs::read(path)?)?;
    Ok(Some(record.system_metadata))
}

fn write_package_metadata(
    layout: &StateLayout,
    package_name: &str,
    system_metadata: &SystemPackageMetadata,
) -> Result<(), InstallError> {
    let path = package_metadata_path(layout, package_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&StoredPackageMetadata {
            package_name: package_name.to_owned(),
            system_metadata: system_metadata.clone(),
        })?,
    )?;

    Ok(())
}

fn remove_package_metadata(layout: &StateLayout, package_name: &str) -> Result<(), InstallError> {
    let path = package_metadata_path(layout, package_name);
    if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn materialize_package_assets(
    layout: &StateLayout,
    package_name: &str,
    system_metadata: &SystemPackageMetadata,
) -> Result<(), InstallError> {
    write_asset(layout, system_metadata.sysusers.as_ref())?;
    write_asset(layout, system_metadata.tmpfiles.as_ref())?;

    if system_metadata.sysusers.is_none() {
        remove_asset(
            layout,
            &format!("/usr/lib/elda/sysusers.d/{package_name}.conf"),
        )?;
    }
    if system_metadata.tmpfiles.is_none() {
        remove_asset(
            layout,
            &format!("/usr/lib/elda/tmpfiles.d/{package_name}.conf"),
        )?;
    }

    Ok(())
}

fn write_asset(
    layout: &StateLayout,
    asset: Option<&elda_build::DeclarativeAsset>,
) -> Result<(), InstallError> {
    let Some(asset) = asset else {
        return Ok(());
    };
    let target = layout.root_dir.join(strip_leading_slash(&asset.path)?);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, &asset.content)?;

    Ok(())
}

fn remove_asset(layout: &StateLayout, asset_path: &str) -> Result<(), InstallError> {
    let target = layout.root_dir.join(strip_leading_slash(asset_path)?);
    if target.exists() || target.is_symlink() {
        fs::remove_file(target)?;
    }

    Ok(())
}

pub fn reconcile_provider_assets(layout: &StateLayout) -> Result<(), InstallError> {
    let packages = load_all_package_metadata(layout)?
        .into_iter()
        .map(|record| (record.package_name, record.system_metadata))
        .collect::<BTreeMap<_, _>>();
    let provider_targets = packages
        .values()
        .flat_map(all_provider_target_paths)
        .collect::<BTreeSet<_>>();

    remove_provider_paths(layout, provider_targets)?;
    clear_provider_storage_root(layout)?;
    materialize_provider_assets_under_root(
        layout.root_dir.as_path(),
        &packages,
        &active_provider_families(layout)?,
    )
}

fn reconcile_alternatives(layout: &StateLayout) -> Result<(), InstallError> {
    let records = load_all_package_metadata(layout)?;
    let mut winners = BTreeMap::<String, AlternativeWinner>::new();

    for record in &records {
        for alternative in &record.system_metadata.alternatives {
            let entry = winners.entry(alternative.link.clone());
            let candidate = AlternativeWinner {
                package_name: record.package_name.clone(),
                target: alternative.path.clone(),
            };
            match entry {
                std::collections::btree_map::Entry::Vacant(vacant) => {
                    vacant.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(mut occupied) => {
                    let current_record = records
                        .iter()
                        .find(|record| record.package_name == occupied.get().package_name)
                        .expect("current alternative owner should exist");
                    let current = current_record
                        .system_metadata
                        .alternatives
                        .iter()
                        .find(|value| value.link == alternative.link)
                        .expect("current alternative should exist");

                    if alternative.priority > current.priority
                        || (alternative.priority == current.priority
                            && (candidate.package_name.as_str(), candidate.target.as_str())
                                < (
                                    occupied.get().package_name.as_str(),
                                    occupied.get().target.as_str(),
                                ))
                    {
                        occupied.insert(candidate);
                    }
                }
            }
        }
    }

    let previous = load_alternative_registry(layout)?;
    for link in previous.winners.keys() {
        if !winners.contains_key(link) {
            remove_link_path(layout, link)?;
        }
    }

    for (link, winner) in &winners {
        write_link_path(layout, link, &winner.target)?;
    }
    save_alternative_registry(layout, &AlternativeRegistry { winners })
}

pub(crate) fn load_all_installed_system_metadata(
    layout: &StateLayout,
) -> Result<BTreeMap<String, SystemPackageMetadata>, InstallError> {
    Ok(load_all_package_metadata(layout)?
        .into_iter()
        .map(|record| (record.package_name, record.system_metadata))
        .collect())
}

fn load_all_package_metadata(
    layout: &StateLayout,
) -> Result<Vec<StoredPackageMetadata>, InstallError> {
    let dir = package_metadata_dir(layout);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    let mut records = Vec::new();
    for path in paths {
        records.push(serde_json::from_slice::<StoredPackageMetadata>(&fs::read(
            path,
        )?)?);
    }

    Ok(records)
}

fn load_alternative_registry(layout: &StateLayout) -> Result<AlternativeRegistry, InstallError> {
    let path = alternative_registry_path(layout);
    if !path.exists() {
        return Ok(AlternativeRegistry::default());
    }

    Ok(serde_json::from_slice::<AlternativeRegistry>(&fs::read(
        path,
    )?)?)
}

fn save_alternative_registry(
    layout: &StateLayout,
    registry: &AlternativeRegistry,
) -> Result<(), InstallError> {
    let path = alternative_registry_path(layout);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(registry)?)?;

    Ok(())
}

fn write_link_path(layout: &StateLayout, link: &str, target: &str) -> Result<(), InstallError> {
    let link_path = layout.root_dir.join(strip_leading_slash(link)?);
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if link_path.exists() || link_path.is_symlink() {
        fs::remove_file(&link_path)?;
    }

    symlink(resolve_symlink_target(layout, target), link_path)?;
    Ok(())
}

fn remove_link_path(layout: &StateLayout, link: &str) -> Result<(), InstallError> {
    let link_path = layout.root_dir.join(strip_leading_slash(link)?);
    if link_path.exists() || link_path.is_symlink() {
        fs::remove_file(link_path)?;
    }

    Ok(())
}
