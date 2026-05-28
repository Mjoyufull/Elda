use std::collections::{BTreeMap, BTreeSet};

use crate::model::{SyncPackageDelta, SyncedIndexSnapshot, SyncedPackageRecord};

pub(super) fn package_deltas(
    previous: Option<&SyncedIndexSnapshot>,
    current: &[SyncedPackageRecord],
) -> Vec<SyncPackageDelta> {
    let mut by_remote = BTreeMap::<String, (BTreeSet<String>, BTreeSet<String>)>::new();

    if let Some(previous) = previous {
        for package in &previous.packages {
            by_remote
                .entry(package.remote_name.clone())
                .or_default()
                .0
                .insert(package.pkgname.clone());
        }
    }
    for package in current {
        by_remote
            .entry(package.remote_name.clone())
            .or_default()
            .1
            .insert(package.pkgname.clone());
    }

    by_remote
        .into_iter()
        .map(|(remote_name, (previous, current))| {
            let added = current.difference(&previous).cloned().collect::<Vec<_>>();
            let removed = previous.difference(&current).cloned().collect::<Vec<_>>();

            SyncPackageDelta {
                remote_name,
                previous_count: previous.len(),
                current_count: current.len(),
                added_count: added.len(),
                removed_count: removed.len(),
                kept_count: current.intersection(&previous).count(),
                added_packages: added.into_iter().take(16).collect(),
                removed_packages: removed.into_iter().take(16).collect(),
            }
        })
        .collect()
}
