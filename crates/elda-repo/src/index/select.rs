use super::*;
use crate::model::{RemotePayloadTrust, RemoteTrustInspection, TrustMode};

pub fn resolve_package(
    snapshot_path: &Path,
    package_name: &str,
) -> Result<Option<SyncedPackageRecord>, RepoError> {
    let snapshot = super::sync::load_snapshot(snapshot_path)?;
    let candidates = snapshot
        .packages
        .into_iter()
        .filter(|package| package.pkgname == package_name)
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Ok(None);
    }

    select_candidate(&candidates).map(Some)
}

pub fn load_remote_payload_trust(
    snapshot_path: &Path,
    remote_name: &str,
) -> Result<RemotePayloadTrust, RepoError> {
    let snapshot = super::sync::load_snapshot(snapshot_path)?;
    let remote = snapshot
        .remotes
        .into_iter()
        .find(|record| record.name == remote_name)
        .ok_or_else(|| {
            RepoError::Parse(format!(
                "remote payload trust lookup could not find remote `{remote_name}` in the synced snapshot"
            ))
        })?;
    let trust_state = super::state::load_remote_trust_state(snapshot_path, remote_name)?;

    if remote.trust != TrustMode::Insecure {
        if !remote.verified {
            return Err(RepoError::Trust(format!(
                "remote `{remote_name}` is not verified and cannot supply trusted payload keys"
            )));
        }
        if trust_state.trusted_public_keys.is_empty() {
            return Err(RepoError::Trust(format!(
                "remote `{remote_name}` has no persisted trusted public keys for payload verification; run `elda sync` again"
            )));
        }
    }

    Ok(RemotePayloadTrust {
        remote_name: remote.name,
        trust: remote.trust,
        verified: remote.verified,
        trusted_public_keys: trust_state.trusted_public_keys,
    })
}

pub fn inspect_remote_trust(
    snapshot_path: &Path,
    remote: &RemoteDocument,
) -> Result<RemoteTrustInspection, RepoError> {
    let trust_state = super::state::load_remote_trust_state(snapshot_path, &remote.name)?;
    let snapshot_record = match super::sync::load_snapshot(snapshot_path) {
        Ok(snapshot) => snapshot
            .remotes
            .into_iter()
            .find(|record| record.name == remote.name),
        Err(RepoError::SnapshotMissing) => None,
        Err(error) => return Err(error),
    };

    Ok(RemoteTrustInspection {
        remote_name: remote.name.clone(),
        trust: remote.trust.clone(),
        enabled: remote.enabled,
        channel: remote.channel.clone(),
        allow_stale: remote.allow_stale,
        configured_trusted_keys: remote.trusted_keys.clone(),
        persisted_trusted_public_keys: trust_state.trusted_public_keys.clone(),
        persisted_trusted_fingerprints: trust_state.trusted_fingerprints.clone(),
        metadata_url: remote.metadata_url.clone(),
        signature_url: remote.signature_url.clone(),
        snapshot_present: snapshot_record.is_some(),
        snapshot_verified: snapshot_record.as_ref().map(|record| record.verified),
        snapshot_stale: snapshot_record.as_ref().map(|record| record.stale),
        selected_key: snapshot_record
            .as_ref()
            .and_then(|record| record.selected_key.clone()),
        last_sync_unix: trust_state.last_sync_unix.or_else(|| {
            snapshot_record
                .as_ref()
                .and_then(|record| record.last_sync_unix)
        }),
        last_verified_unix: trust_state.last_verified_unix.or_else(|| {
            snapshot_record
                .as_ref()
                .and_then(|record| record.last_verified_unix)
        }),
        last_error: trust_state.last_error.clone(),
        rotation_policy: rotation_policy(remote),
        payload_verification: payload_verification(remote, snapshot_record.as_ref(), &trust_state),
        pending_rotation: trust_state.last_error.as_deref().is_some_and(|error| {
            error.contains("rotation")
                || error.contains("key changed")
                || error.contains("accept-rotated-key")
        }),
        rotation_accept_required: remote.trust == TrustMode::Tofu
            && remote.metadata_url.is_some()
            && trust_state
                .last_error
                .as_deref()
                .is_some_and(|error| error.contains("accept-rotated-key")),
    })
}

pub fn search_packages(
    snapshot_path: &Path,
    query: &str,
    regex_search: bool,
) -> Result<Vec<SyncedPackageRecord>, RepoError> {
    let snapshot = super::sync::load_snapshot(snapshot_path)?;
    let winners = winners_by_name(snapshot.packages)?;
    let query_lower = query.to_ascii_lowercase();
    let regex = if regex_search {
        Some(Regex::new(query)?)
    } else {
        None
    };

    let mut matches = winners
        .into_values()
        .filter(|package| matches_query(package, &query_lower, regex.as_ref()))
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        if regex_search {
            return left
                .pkgname
                .cmp(&right.pkgname)
                .then_with(|| candidate_order(left, right));
        }

        query_rank(&left.pkgname, &query_lower)
            .cmp(&query_rank(&right.pkgname, &query_lower))
            .then_with(|| left.pkgname.cmp(&right.pkgname))
            .then_with(|| candidate_order(left, right))
    });

    Ok(matches)
}

fn rotation_policy(remote: &RemoteDocument) -> String {
    match &remote.trust {
        TrustMode::Insecure => "unsigned insecure remote; key rotation is disabled".to_owned(),
        TrustMode::Pinned => {
            if remote.trusted_keys.is_empty() {
                "pinned remote is blocked until trusted keys are configured".to_owned()
            } else {
                "pinned key set; rotations require explicit remote document update".to_owned()
            }
        }
        TrustMode::Tofu => {
            if remote.metadata_url.is_some() {
                "TOFU with signed metadata rotation; rotated keys require --accept-rotated-key"
                    .to_owned()
            } else {
                "TOFU first-use trust; key changes are blocked without metadata_url".to_owned()
            }
        }
    }
}

fn payload_verification(
    remote: &RemoteDocument,
    snapshot_record: Option<&SyncedRemoteRecord>,
    trust_state: &super::state::RemoteTrustState,
) -> String {
    if remote.trust == TrustMode::Insecure {
        return "payload signatures are not enforced for insecure remotes".to_owned();
    }
    if snapshot_record.is_none() {
        return "no synced snapshot yet; run elda sync before payload trust is usable".to_owned();
    }
    if !snapshot_record.is_some_and(|record| record.verified) {
        return "last snapshot is unverified; signed payload installs are blocked".to_owned();
    }
    if trust_state.trusted_public_keys.is_empty() {
        return "snapshot is verified but payload key material is missing; run elda sync again"
            .to_owned();
    }

    format!(
        "signed payload verification enabled with {} persisted remote key(s)",
        trust_state.trusted_public_keys.len()
    )
}

pub(super) fn candidate_order(left: &SyncedPackageRecord, right: &SyncedPackageRecord) -> Ordering {
    left.remote_priority
        .cmp(&right.remote_priority)
        .then_with(|| {
            let left_version = parse_version(left).unwrap_or(PackageVersion {
                epoch: left.epoch,
                pkgver: left.pkgver.clone(),
                pkgrel: left.pkgrel,
            });
            let right_version = parse_version(right).unwrap_or(PackageVersion {
                epoch: right.epoch,
                pkgver: right.pkgver.clone(),
                pkgrel: right.pkgrel,
            });
            right_version.cmp(&left_version)
        })
        .then_with(|| left.remote_name.cmp(&right.remote_name))
}

fn winners_by_name(
    packages: Vec<SyncedPackageRecord>,
) -> Result<BTreeMap<String, SyncedPackageRecord>, RepoError> {
    let mut grouped = BTreeMap::<String, Vec<SyncedPackageRecord>>::new();
    for package in packages {
        grouped
            .entry(package.pkgname.clone())
            .or_default()
            .push(package);
    }

    grouped
        .into_iter()
        .map(|(pkgname, records)| select_candidate(&records).map(|record| (pkgname, record)))
        .collect()
}

fn select_candidate(candidates: &[SyncedPackageRecord]) -> Result<SyncedPackageRecord, RepoError> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(candidate_order);

    let winner = sorted
        .first()
        .cloned()
        .ok_or_else(|| RepoError::Parse("package selection received no candidates".to_owned()))?;
    let winner_version = parse_version(&winner)?;

    for candidate in sorted.iter().skip(1) {
        if candidate.remote_priority != winner.remote_priority {
            break;
        }

        let candidate_version = parse_version(candidate)?;
        if candidate_version != winner_version {
            break;
        }

        if !same_provenance(&winner, candidate) {
            return Err(RepoError::Parse(format!(
                "package `{}` is ambiguous between remotes `{}` and `{}`",
                winner.pkgname, winner.remote_name, candidate.remote_name
            )));
        }
    }

    Ok(winner)
}

fn parse_version(package: &SyncedPackageRecord) -> Result<PackageVersion, RepoError> {
    PackageVersion::from_str(&package.version_string())
        .map_err(|error| RepoError::Parse(error.to_string()))
}

fn same_provenance(left: &SyncedPackageRecord, right: &SyncedPackageRecord) -> bool {
    left.pkg_lua == right.pkg_lua
        && left.source_kind == right.source_kind
        && left.source_ref == right.source_ref
        && left.asset_url == right.asset_url
        && left.sha256 == right.sha256
        && left.release_tag == right.release_tag
        && left.repo_commit == right.repo_commit
}

fn matches_query(package: &SyncedPackageRecord, query_lower: &str, regex: Option<&Regex>) -> bool {
    if let Some(regex) = regex {
        return regex.is_match(&package.pkgname);
    }

    package.pkgname.to_ascii_lowercase().contains(query_lower)
}

fn query_rank(pkgname: &str, query_lower: &str) -> u8 {
    let pkgname_lower = pkgname.to_ascii_lowercase();
    if pkgname_lower == query_lower {
        0
    } else if pkgname_lower.starts_with(query_lower) {
        1
    } else {
        2
    }
}
