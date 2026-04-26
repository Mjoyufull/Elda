use super::state::{
    RemoteSnapshotState, RemoteTrustState, load_cached_remote_snapshot, load_remote_trust_state,
    store_remote_snapshot, store_remote_trust_state,
};
use super::trust::{parse_signature_envelope, verify_remote_signature};
use super::*;
use crate::model::TrustMode;

#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    pub offline: bool,
    pub allow_initial_tofu: bool,
    pub accept_rotated_keys: Vec<String>,
}

pub fn sync_remotes(
    remotes_dir: &Path,
    snapshot_path: &Path,
    options: SyncOptions,
) -> Result<SyncReport, RepoError> {
    let mut remotes = list_remotes(remotes_dir)?
        .into_iter()
        .filter(|remote| remote.enabled)
        .collect::<Vec<_>>();
    remotes.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.name.cmp(&right.name))
    });

    let mut snapshot = SyncedIndexSnapshot {
        schema_version: 3,
        generated_at: current_unix_timestamp(),
        offline: options.offline,
        remotes: Vec::new(),
        packages: Vec::new(),
    };

    for remote in remotes {
        let result = sync_one_remote(&remote, snapshot_path, &options)?;
        snapshot.packages.extend(result.packages);
        snapshot.remotes.push(result.record);
    }

    snapshot.packages.sort_by(super::select::candidate_order);
    let failed_remote_count = snapshot
        .remotes
        .iter()
        .filter(|remote| remote.issue.is_some() && remote.package_count == 0)
        .count();
    let verified_remote_count = snapshot
        .remotes
        .iter()
        .filter(|remote| remote.verified)
        .count();
    let stale_remote_count = snapshot
        .remotes
        .iter()
        .filter(|remote| remote.stale)
        .count();
    let has_usable_snapshot = snapshot
        .remotes
        .iter()
        .any(|remote| remote.package_count > 0);

    if options.offline && failed_remote_count > 0 {
        let issue = snapshot
            .remotes
            .iter()
            .find_map(|remote| remote.issue.as_deref())
            .unwrap_or(
                "offline sync could not satisfy all enabled remotes from verified local snapshots",
            );
        return Err(RepoError::Trust(format!(
            "offline sync could not satisfy all enabled remotes from verified local snapshots: {issue}"
        )));
    }
    if failed_remote_count > 0 && !has_usable_snapshot {
        let issue = snapshot
            .remotes
            .iter()
            .find_map(|remote| remote.issue.as_deref())
            .unwrap_or("sync could not produce a usable snapshot for any enabled remote");

        return Err(RepoError::Trust(issue.to_owned()));
    }

    let snapshot_dir = snapshot_path.parent().ok_or_else(|| {
        RepoError::Parse(format!(
            "snapshot path `{}` has no parent directory",
            snapshot_path.display()
        ))
    })?;
    if has_usable_snapshot || !snapshot_path.exists() {
        fs::create_dir_all(snapshot_dir)?;
        fs::write(snapshot_path, serde_json::to_vec_pretty(&snapshot)?)?;
    }

    Ok(SyncReport {
        snapshot_path: snapshot_path.to_path_buf(),
        offline: options.offline,
        remote_count: snapshot.remotes.len(),
        package_count: snapshot.packages.len(),
        verified_remote_count,
        stale_remote_count,
        failed_remote_count,
        remotes: snapshot.remotes,
    })
}

pub fn load_snapshot(snapshot_path: &Path) -> Result<SyncedIndexSnapshot, RepoError> {
    if !snapshot_path.exists() {
        return Err(RepoError::SnapshotMissing);
    }

    let content = fs::read_to_string(snapshot_path)?;
    serde_json::from_str(&content).map_err(RepoError::from)
}

#[derive(Debug)]
struct RemoteSyncResult {
    record: SyncedRemoteRecord,
    packages: Vec<SyncedPackageRecord>,
}

fn sync_one_remote(
    remote: &RemoteDocument,
    snapshot_path: &Path,
    options: &SyncOptions,
) -> Result<RemoteSyncResult, RepoError> {
    let mut trust_state = load_remote_trust_state(snapshot_path, &remote.name)?;
    let now = current_unix_timestamp();

    let result = if options.offline {
        load_offline_remote(remote, snapshot_path, &trust_state)
    } else {
        fetch_remote(remote, snapshot_path, &mut trust_state, now, options)
    };

    if let Err(error) = &result {
        trust_state.last_sync_unix = Some(now);
        trust_state.last_error = Some(error.to_string());
        store_remote_trust_state(snapshot_path, &remote.name, &trust_state)?;
    }

    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            load_stale_remote(remote, snapshot_path, &trust_state, Some(error.to_string()))
        }
    }
}

fn fetch_remote(
    remote: &RemoteDocument,
    snapshot_path: &Path,
    trust_state: &mut RemoteTrustState,
    now: u64,
    options: &SyncOptions,
) -> Result<RemoteSyncResult, RepoError> {
    let content = super::fetch::read_location_text(&remote.index_url)?;
    let packages = super::document::parse_remote_packages_from_content(remote, &content)?;
    let verification = verify_live_remote(remote, trust_state, &content, options)?;

    trust_state.last_sync_unix = Some(now);
    trust_state.last_error = None;
    if verification.verified {
        trust_state.last_verified_unix = Some(now);
    }
    store_remote_trust_state(snapshot_path, &remote.name, trust_state)?;

    let record = SyncedRemoteRecord {
        name: remote.name.clone(),
        index_url: remote.index_url.clone(),
        channel: remote.channel.clone(),
        priority: remote.priority,
        package_count: packages.len(),
        trust: remote.trust.clone(),
        verified: verification.verified,
        stale: false,
        source: "fresh".to_owned(),
        selected_key: verification.selected_key,
        last_sync_unix: trust_state.last_sync_unix,
        last_verified_unix: trust_state.last_verified_unix,
        issue: None,
    };

    if record.verified {
        store_remote_snapshot(
            snapshot_path,
            &remote.name,
            &RemoteSnapshotState {
                remote: record.clone(),
                packages: packages.clone(),
            },
        )?;
    }

    Ok(RemoteSyncResult { record, packages })
}

fn verify_live_remote(
    remote: &RemoteDocument,
    trust_state: &mut RemoteTrustState,
    content: &str,
    options: &SyncOptions,
) -> Result<VerificationResult, RepoError> {
    if remote.trust == TrustMode::Insecure {
        return Ok(VerificationResult {
            verified: false,
            selected_key: None,
        });
    }

    let signature_url = remote
        .signature_url
        .clone()
        .unwrap_or_else(|| format!("{}.sig", remote.index_url));
    let signature_text = super::fetch::read_location_text(&signature_url)?;
    let signature = parse_signature_envelope(&signature_text)?;
    let verified_key = verify_remote_signature(remote, trust_state, content, signature, options)?;

    Ok(VerificationResult {
        verified: true,
        selected_key: Some(verified_key.selected_key),
    })
}

fn load_offline_remote(
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
    })
}

fn load_stale_remote(
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
    })
}

#[derive(Debug)]
struct VerificationResult {
    verified: bool,
    selected_key: Option<String>,
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

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
