use std::collections::{BTreeMap, BTreeSet};

use elda_db::Database;

use crate::archive_state::{ArchivedStateDocument, available_state_ids, read_state_archive};
use crate::{InstallError, RollbackPlan};

pub fn rollback_plan(
    database: &Database,
    target_state: Option<&str>,
) -> Result<RollbackPlan, InstallError> {
    database.bootstrap()?;
    let current_state = database.state_snapshot()?.active_state;
    let target = resolve_target_archive(database, current_state.as_deref(), target_state)?;
    build_rollback_plan(database, current_state, &target)
}

pub(super) fn build_rollback_plan(
    database: &Database,
    current_state: Option<String>,
    target: &ArchivedStateDocument,
) -> Result<RollbackPlan, InstallError> {
    let current_versions = database
        .list_installed_packages()?
        .into_iter()
        .map(|package| (package.pkgname, package.version))
        .collect::<BTreeMap<_, _>>();
    let target_versions = target
        .packages
        .iter()
        .map(|package| {
            (
                package.pkgname.clone(),
                format!("{}:{}-{}", package.epoch, package.pkgver, package.pkgrel),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let removed_packages = current_versions
        .keys()
        .filter(|name| {
            target_versions
                .get(*name)
                .is_none_or(|target_version| current_versions.get(*name) != Some(target_version))
        })
        .cloned()
        .collect::<Vec<_>>();
    let restored_packages = target_versions
        .keys()
        .filter(|name| current_versions.get(*name) != target_versions.get(*name))
        .cloned()
        .collect::<Vec<_>>();
    let untouched_packages = target_versions
        .keys()
        .filter(|name| current_versions.get(*name) == target_versions.get(*name))
        .cloned()
        .collect::<Vec<_>>();

    Ok(RollbackPlan {
        from_state: current_state,
        to_state: target.state_id.clone(),
        removed_packages,
        restored_packages,
        untouched_packages,
    })
}

pub(super) fn resolve_target_archive(
    database: &Database,
    current_state: Option<&str>,
    requested_state: Option<&str>,
) -> Result<ArchivedStateDocument, InstallError> {
    let state_id = if let Some(requested_state) = requested_state {
        requested_state.to_owned()
    } else {
        infer_previous_state_id(database, current_state)?
    };

    read_state_archive(database.layout(), &state_id)
}

fn infer_previous_state_id(
    database: &Database,
    current_state: Option<&str>,
) -> Result<String, InstallError> {
    let archive_ids = available_state_ids(database.layout())?;
    let current_state = current_state.ok_or_else(|| {
        InstallError::StateArchive(
            "cannot infer a rollback target because no current archived state is active".to_owned(),
        )
    })?;
    let current_index = archive_ids
        .iter()
        .position(|state_id| state_id == current_state)
        .ok_or_else(|| {
            InstallError::StateArchive(format!(
                "current state `{current_state}` does not exist in the archived state set"
            ))
        })?;
    if current_index == 0 {
        return Err(InstallError::StateArchive(
            "no previous archived state exists for rollback".to_owned(),
        ));
    }

    let current_archive = read_state_archive(database.layout(), current_state)?;
    let current_packages = current_archive
        .packages
        .iter()
        .map(|package| package.pkgname.clone())
        .collect::<BTreeSet<_>>();
    let mut best_match = None::<(usize, usize, String)>;

    for (index, archive_id) in archive_ids[..current_index].iter().enumerate() {
        let archive = read_state_archive(database.layout(), archive_id)?;
        let candidate_packages = archive
            .packages
            .iter()
            .map(|package| package.pkgname.clone())
            .collect::<BTreeSet<_>>();
        let overlap = if current_packages.is_empty() {
            0
        } else {
            candidate_packages.intersection(&current_packages).count()
        };
        if best_match
            .as_ref()
            .is_none_or(|best| overlap > best.0 || (overlap == best.0 && index > best.1))
        {
            best_match = Some((overlap, index, archive_id.clone()));
        }
    }

    Ok(best_match
        .map(|(_, _, archive_id)| archive_id)
        .unwrap_or_else(|| archive_ids[current_index - 1].clone()))
}
