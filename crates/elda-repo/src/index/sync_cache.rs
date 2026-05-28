use std::path::Path;

use crate::error::RepoError;
use crate::model::{RemoteDocument, SyncedRemoteRecord};

use super::state::{RemoteTrustState, load_cached_remote_snapshot};
use super::sync::RemoteSyncResult;

pub(super) fn load_offline_remote(
    remote: &RemoteDocument,
    snapshot_path: &Path,
    trust_state: &RemoteTrustState,
) -> Result<RemoteSyncResult, RepoError> {
    let Some(snapshot) = load_cached_remote_snapshot(snapshot_path, &remote.name)? else {
        return Err(RepoError::Trust(format!(
            "offline sync cannot use remote `{}` because no verified snapshot is cached locally",
            remote.name
        )));
    };
    if !snapshot.remote.verified {
        return Err(RepoError::Trust(format!(
            "offline sync cannot use remote `{}` because the cached snapshot is not verified",
            remote.name
        )));
    }
    ensure_cached_snapshot_matches_remote(remote, &snapshot.remote)?;

    Ok(RemoteSyncResult {
        record: SyncedRemoteRecord {
            name: remote.name.clone(),
            index_url: remote.index_url.clone(),
            channel: remote.channel.clone(),
            priority: remote.priority,
            package_count: snapshot.packages.len(),
            trust: remote.trust.clone(),
            verified: true,
            stale: true,
            source: "offline-cache".to_owned(),
            selected_key: snapshot.remote.selected_key,
            last_sync_unix: trust_state
                .last_sync_unix
                .or(snapshot.remote.last_sync_unix),
            last_verified_unix: trust_state
                .last_verified_unix
                .or(snapshot.remote.last_verified_unix),
            issue: None,
        },
        packages: snapshot.packages,
        interemote: None,
    })
}

pub(super) fn load_stale_remote(
    remote: &RemoteDocument,
    snapshot_path: &Path,
    trust_state: &RemoteTrustState,
    issue: Option<String>,
) -> Result<RemoteSyncResult, RepoError> {
    if remote.allow_stale
        && let Some(snapshot) = load_cached_remote_snapshot(snapshot_path, &remote.name)?
        && snapshot.remote.verified
        && cached_snapshot_matches_remote(remote, &snapshot.remote)
    {
        return Ok(RemoteSyncResult {
            record: SyncedRemoteRecord {
                name: remote.name.clone(),
                index_url: remote.index_url.clone(),
                channel: remote.channel.clone(),
                priority: remote.priority,
                package_count: snapshot.packages.len(),
                trust: remote.trust.clone(),
                verified: true,
                stale: true,
                source: "stale-cache".to_owned(),
                selected_key: snapshot.remote.selected_key,
                last_sync_unix: trust_state
                    .last_sync_unix
                    .or(snapshot.remote.last_sync_unix),
                last_verified_unix: trust_state
                    .last_verified_unix
                    .or(snapshot.remote.last_verified_unix),
                issue,
            },
            packages: snapshot.packages,
            interemote: None,
        });
    }

    Ok(RemoteSyncResult {
        record: SyncedRemoteRecord {
            name: remote.name.clone(),
            index_url: remote.index_url.clone(),
            channel: remote.channel.clone(),
            priority: remote.priority,
            package_count: 0,
            trust: remote.trust.clone(),
            verified: false,
            stale: false,
            source: if trust_state.last_verified_unix.is_some() {
                "failed"
            } else {
                "unverified"
            }
            .to_owned(),
            selected_key: None,
            last_sync_unix: trust_state.last_sync_unix,
            last_verified_unix: trust_state.last_verified_unix,
            issue,
        },
        packages: Vec::new(),
        interemote: None,
    })
}

fn ensure_cached_snapshot_matches_remote(
    remote: &RemoteDocument,
    cached: &SyncedRemoteRecord,
) -> Result<(), RepoError> {
    if cached_snapshot_matches_remote(remote, cached) {
        return Ok(());
    }

    Err(RepoError::Trust(format!(
        "cached snapshot for remote `{}` is for channel `{}` but the configured channel is `{}`",
        remote.name, cached.channel, remote.channel
    )))
}

fn cached_snapshot_matches_remote(remote: &RemoteDocument, cached: &SyncedRemoteRecord) -> bool {
    cached.channel == remote.channel
}
