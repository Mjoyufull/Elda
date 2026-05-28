use std::collections::BTreeSet;

use serde::Serialize;

use elda_db::Database;
use elda_types::PackageVersion;

use crate::archive_state::{ArchivedPackage, available_state_ids, read_state_archive};
use crate::cached_archive::{archive_package_paths, built_package_from_archive};
use crate::install_tx::install_upgraded_package;
use crate::remove_tx::remove_package_for_upgrade;
use crate::{InstallError, InstallReport, MutationPolicy};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DowngradeCandidate {
    pub state_id: String,
    pub package_name: String,
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
    pub arch: String,
    pub source_kind: String,
    pub remote_name: Option<String>,
    pub repo_commit: Option<String>,
}

impl DowngradeCandidate {
    #[must_use]
    pub fn version(&self) -> PackageVersion {
        PackageVersion {
            epoch: self.epoch,
            pkgver: self.pkgver.clone(),
            pkgrel: self.pkgrel,
        }
    }
}

pub fn list_downgrade_candidates(
    database: &Database,
    package_name: &str,
) -> Result<Vec<DowngradeCandidate>, InstallError> {
    let mut candidates = Vec::new();

    for state_id in available_state_ids(database.layout())? {
        let archive = read_state_archive(database.layout(), &state_id)?;
        for archived in archive
            .packages
            .into_iter()
            .filter(|archived| archived.pkgname == package_name)
        {
            let (payload_path, manifest_path) = archive_package_paths(database.layout(), &archived);
            if !payload_path.exists() || !manifest_path.exists() {
                continue;
            }

            candidates.push(candidate_from_archived(state_id.clone(), archived));
        }
    }

    candidates.sort_by(|left, right| {
        right
            .version()
            .cmp(&left.version())
            .then_with(|| right.state_id.cmp(&left.state_id))
    });

    let mut seen = BTreeSet::new();
    candidates.retain(|candidate| {
        seen.insert(format!(
            "{}:{}-{}:{}",
            candidate.epoch, candidate.pkgver, candidate.pkgrel, candidate.arch
        ))
    });

    Ok(candidates)
}

pub fn downgrade_to_candidate(
    database: &Database,
    candidate: &DowngradeCandidate,
    install_reason: &str,
    pinned_version: Option<String>,
    held: bool,
    hold_source: Option<String>,
    policy: &MutationPolicy,
) -> Result<InstallReport, InstallError> {
    let archived = archived_package_for_candidate(database, candidate)?;
    let package = built_package_from_archive(database, &archived)?;

    if database
        .installed_package(&candidate.package_name)?
        .is_some()
    {
        remove_package_for_upgrade(database, &candidate.package_name, policy)?;
    }

    install_upgraded_package(
        database,
        &package,
        install_reason,
        pinned_version,
        held,
        hold_source,
        policy,
    )
}

fn archived_package_for_candidate(
    database: &Database,
    candidate: &DowngradeCandidate,
) -> Result<ArchivedPackage, InstallError> {
    let archive = read_state_archive(database.layout(), &candidate.state_id)?;
    archive
        .packages
        .into_iter()
        .find(|archived| {
            archived.pkgname == candidate.package_name
                && archived.epoch == candidate.epoch
                && archived.pkgver == candidate.pkgver
                && archived.pkgrel == candidate.pkgrel
                && archived.arch == candidate.arch
        })
        .ok_or_else(|| {
            InstallError::StateArchive(format!(
                "archived downgrade candidate `{}` {}:{}-{} is missing from state `{}`",
                candidate.package_name,
                candidate.epoch,
                candidate.pkgver,
                candidate.pkgrel,
                candidate.state_id
            ))
        })
}

fn candidate_from_archived(state_id: String, archived: ArchivedPackage) -> DowngradeCandidate {
    DowngradeCandidate {
        state_id,
        package_name: archived.pkgname,
        epoch: archived.epoch,
        pkgver: archived.pkgver,
        pkgrel: archived.pkgrel,
        arch: archived.arch,
        source_kind: archived.source_kind,
        remote_name: archived.remote_name,
        repo_commit: archived.repo_commit,
    }
}
