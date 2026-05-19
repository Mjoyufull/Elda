use std::sync::Arc;

use super::state::{
    RemoteSnapshotState, RemoteTrustState, load_remote_trust_state, store_remote_snapshot,
    store_remote_trust_state,
};
use super::sync_cache::{load_offline_remote, load_stale_remote};
use super::trust::{parse_signature_envelope, verify_remote_signature};
use super::*;
use crate::model::{InteremotePreview, TrustMode};

#[derive(Debug, Clone)]
pub enum RemoteSyncEvent {
    RemoteStart {
        name: String,
    },
    RemoteDone {
        name: String,
        package_count: usize,
        stale: bool,
        issue: Option<String>,
    },
}

pub type RemoteSyncProgress = Arc<dyn Fn(RemoteSyncEvent) + Send + Sync>;

#[derive(Clone)]
pub struct SyncOptions {
    pub offline: bool,
    pub allow_initial_tofu: bool,
    pub accept_rotated_keys: Vec<String>,
    pub target_remotes: Vec<String>,
    pub allowed_git_protocols: Vec<String>,
    pub progress: Option<RemoteSyncProgress>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            offline: false,
            allow_initial_tofu: false,
            accept_rotated_keys: Vec::new(),
            target_remotes: Vec::new(),
            allowed_git_protocols: vec!["https".to_owned(), "ssh".to_owned(), "file".to_owned()],
            progress: None,
        }
    }
}

pub fn sync_remotes(
    remotes_dir: &Path,
    snapshot_path: &Path,
    options: SyncOptions,
) -> Result<SyncReport, RepoError> {
    let previous_snapshot = load_snapshot(snapshot_path).ok();
    let all_remotes = list_remotes(remotes_dir)?;
    validate_target_remotes(&all_remotes, &options.target_remotes)?;
    let mut remotes = all_remotes
        .into_iter()
        .filter(|remote| remote.enabled)
        .filter(|remote| target_included(&remote.name, &options.target_remotes))
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

    let mut interemotes = Vec::new();
    for remote in remotes {
        if let Some(progress) = &options.progress {
            progress(RemoteSyncEvent::RemoteStart {
                name: remote.name.clone(),
            });
        }
        let result = sync_one_remote(&remote, snapshot_path, &options)?;
        if let Some(progress) = &options.progress {
            progress(RemoteSyncEvent::RemoteDone {
                name: remote.name.clone(),
                package_count: result.record.package_count,
                stale: result.record.stale,
                issue: result.record.issue.clone(),
            });
        }
        if let Some(preview) = result.interemote {
            interemotes.push(preview);
        }
        snapshot.packages.extend(result.packages);
        snapshot.remotes.push(result.record);
    }

    snapshot.packages.sort_by(super::select::candidate_order);
    let package_deltas =
        super::sync_delta::package_deltas(previous_snapshot.as_ref(), &snapshot.packages);
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
        return Err(RepoError::Trust(
            super::sync_failure::offline_failure_message(&snapshot.remotes),
        ));
    }
    if failed_remote_count > 0 && !has_usable_snapshot {
        return Err(RepoError::Trust(super::sync_failure::all_failed_message(
            &snapshot.remotes,
        )));
    }

    let snapshot_dir = snapshot_path.parent().ok_or_else(|| {
        RepoError::Parse(format!(
            "snapshot path `{}` has no parent directory",
            snapshot_path.display()
        ))
    })?;
    if has_usable_snapshot || failed_remote_count == 0 || !snapshot_path.exists() {
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
        package_deltas,
        interemotes,
    })
}

fn validate_target_remotes(
    remotes: &[RemoteDocument],
    targets: &[String],
) -> Result<(), RepoError> {
    if targets.is_empty() {
        return Ok(());
    }

    for target in targets {
        let Some(remote) = remotes.iter().find(|remote| remote.name == *target) else {
            return Err(RepoError::Parse(format!(
                "remote `{target}` is not registered"
            )));
        };
        if !remote.enabled {
            return Err(RepoError::Parse(format!(
                "remote `{target}` is disabled; enable it before targeted sync"
            )));
        }
    }
    Ok(())
}

fn target_included(remote_name: &str, targets: &[String]) -> bool {
    targets.is_empty() || targets.iter().any(|target| target == remote_name)
}

pub fn load_snapshot(snapshot_path: &Path) -> Result<SyncedIndexSnapshot, RepoError> {
    if !snapshot_path.exists() {
        return Err(RepoError::SnapshotMissing);
    }

    let content = fs::read_to_string(snapshot_path)?;
    serde_json::from_str(&content).map_err(RepoError::from)
}

#[derive(Debug)]
pub(super) struct RemoteSyncResult {
    pub(super) record: SyncedRemoteRecord,
    pub(super) packages: Vec<SyncedPackageRecord>,
    pub(super) interemote: Option<InteremotePreview>,
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
            let issue = super::sync_failure::contextual_remote_error(
                &remote.name,
                super::interemote::should_try_interemote(remote),
                &error.to_string(),
            );
            load_stale_remote(remote, snapshot_path, &trust_state, Some(issue))
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
    if super::interemote::should_try_interemote(remote) {
        return fetch_interemote(remote, snapshot_path, trust_state, now, options);
    }

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

    Ok(RemoteSyncResult {
        record,
        packages,
        interemote: None,
    })
}

fn fetch_interemote(
    remote: &RemoteDocument,
    snapshot_path: &Path,
    trust_state: &mut RemoteTrustState,
    now: u64,
    options: &SyncOptions,
) -> Result<RemoteSyncResult, RepoError> {
    let interemote =
        super::interemote::sync_interemote_with_preview(remote, &options.allowed_git_protocols)?;
    let packages = interemote.packages;
    let preview = interemote.preview;
    trust_state.last_sync_unix = Some(now);
    trust_state.last_error = None;
    store_remote_trust_state(snapshot_path, &remote.name, trust_state)?;

    let record = SyncedRemoteRecord {
        name: remote.name.clone(),
        index_url: remote.index_url.clone(),
        channel: remote.channel.clone(),
        priority: remote.priority,
        package_count: packages.len(),
        trust: remote.trust.clone(),
        verified: false,
        stale: false,
        source: "interemote".to_owned(),
        selected_key: None,
        last_sync_unix: trust_state.last_sync_unix,
        last_verified_unix: trust_state.last_verified_unix,
        issue: interemote_issue(&preview, packages.len()),
    };

    store_remote_snapshot(
        snapshot_path,
        &remote.name,
        &RemoteSnapshotState {
            remote: record.clone(),
            packages: packages.clone(),
        },
    )?;

    Ok(RemoteSyncResult {
        record,
        packages,
        interemote: Some(preview),
    })
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

fn interemote_issue(preview: &InteremotePreview, package_count: usize) -> Option<String> {
    let issue_count = preview.issues.len();
    if issue_count == 0 {
        return None;
    }

    if package_count == 0 {
        return Some(format!(
            "interemote parser rejected all included packages ({issue_count} issue(s))"
        ));
    }

    Some(format!(
        "interemote parser skipped {issue_count} package(s); synced {package_count}"
    ))
}

#[derive(Debug)]
struct VerificationResult {
    verified: bool,
    selected_key: Option<String>,
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
