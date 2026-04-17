use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::RepoError;
use crate::model::{SyncedPackageRecord, SyncedRemoteRecord, TrustedPublicKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct RemoteTrustState {
    #[serde(default)]
    pub(super) trusted_public_keys: Vec<TrustedPublicKey>,
    #[serde(default)]
    pub(super) trusted_fingerprints: Vec<String>,
    pub(super) last_sync_unix: Option<u64>,
    pub(super) last_verified_unix: Option<u64>,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct RemoteSnapshotState {
    pub(super) remote: SyncedRemoteRecord,
    pub(super) packages: Vec<SyncedPackageRecord>,
}

pub(super) fn load_remote_trust_state(
    snapshot_path: &Path,
    remote_name: &str,
) -> Result<RemoteTrustState, RepoError> {
    let path = trust_state_path(snapshot_path, remote_name);
    if !path.exists() {
        return Ok(RemoteTrustState::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

pub(super) fn store_remote_trust_state(
    snapshot_path: &Path,
    remote_name: &str,
    state: &RemoteTrustState,
) -> Result<(), RepoError> {
    let path = trust_state_path(snapshot_path, remote_name);
    ensure_repo_state_dir(&path)?;
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

pub(super) fn load_cached_remote_snapshot(
    snapshot_path: &Path,
    remote_name: &str,
) -> Result<Option<RemoteSnapshotState>, RepoError> {
    let path = remote_snapshot_path(snapshot_path, remote_name);
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&content)?))
}

pub(super) fn store_remote_snapshot(
    snapshot_path: &Path,
    remote_name: &str,
    snapshot: &RemoteSnapshotState,
) -> Result<(), RepoError> {
    let path = remote_snapshot_path(snapshot_path, remote_name);
    ensure_repo_state_dir(&path)?;
    fs::write(path, serde_json::to_vec_pretty(snapshot)?)?;
    Ok(())
}

impl RemoteTrustState {
    pub(super) fn has_trusted_fingerprint(&self, fingerprint: &str) -> bool {
        self.trusted_fingerprints
            .iter()
            .any(|trusted| trusted == fingerprint)
            || self
                .trusted_public_keys
                .iter()
                .any(|trusted| trusted.fingerprint == fingerprint)
    }

    pub(super) fn remember_verified_key(
        &mut self,
        key_id: &str,
        fingerprint: &str,
        public_key: &str,
    ) {
        if !self.has_trusted_fingerprint(fingerprint) {
            self.trusted_fingerprints.push(fingerprint.to_owned());
        }

        if let Some(existing) = self
            .trusted_public_keys
            .iter_mut()
            .find(|trusted| trusted.fingerprint == fingerprint)
        {
            existing.key_id = key_id.to_owned();
            existing.public_key = public_key.to_owned();
            return;
        }

        self.trusted_public_keys.push(TrustedPublicKey {
            key_id: key_id.to_owned(),
            fingerprint: fingerprint.to_owned(),
            public_key: public_key.to_owned(),
        });
    }
}

fn trust_state_path(snapshot_path: &Path, remote_name: &str) -> PathBuf {
    repo_state_dir(snapshot_path).join(format!("{remote_name}.trust.json"))
}

fn remote_snapshot_path(snapshot_path: &Path, remote_name: &str) -> PathBuf {
    repo_state_dir(snapshot_path).join(format!("{remote_name}.snapshot.json"))
}

fn repo_state_dir(snapshot_path: &Path) -> PathBuf {
    snapshot_path
        .parent()
        .map(|parent| parent.join("repo-state"))
        .unwrap_or_else(|| PathBuf::from("repo-state"))
}

fn ensure_repo_state_dir(path: &Path) -> Result<(), RepoError> {
    let Some(parent) = path.parent() else {
        return Err(RepoError::Parse(format!(
            "repo state path `{}` has no parent directory",
            path.display()
        )));
    };
    fs::create_dir_all(parent)?;
    Ok(())
}
