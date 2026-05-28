use crate::model::SyncedRemoteRecord;

pub(super) fn offline_failure_message(remotes: &[SyncedRemoteRecord]) -> String {
    let summary = remote_issue_summary(remotes);
    if summary.is_empty() {
        return "offline sync could not satisfy all enabled remotes from verified local snapshots"
            .to_owned();
    }

    format!(
        "offline sync could not satisfy all enabled remotes from verified local snapshots: {summary}"
    )
}

pub(super) fn all_failed_message(remotes: &[SyncedRemoteRecord]) -> String {
    let count = remotes.len();
    let summary = remote_issue_summary(remotes);
    if summary.is_empty() {
        return format!("sync produced no usable packages from {count} enabled remote(s)");
    }

    format!(
        "sync produced no usable packages from {count} enabled remote(s): {summary}. previous snapshot was left unchanged; inspect the named remote with `elda rmt info <name>` or `elda rmt preview <name>` for interemotes"
    )
}

pub(super) fn contextual_remote_error(remote_name: &str, interemote: bool, error: &str) -> String {
    let surface = if interemote {
        "interemote sync"
    } else {
        "index sync"
    };
    format!("{surface} failed for `{remote_name}`: {error}")
}

fn remote_issue_summary(remotes: &[SyncedRemoteRecord]) -> String {
    remotes
        .iter()
        .filter_map(|remote| {
            let issue = remote.issue.as_deref()?;
            Some(format!("{} [{}]: {}", remote.name, remote.source, issue))
        })
        .take(6)
        .collect::<Vec<_>>()
        .join("; ")
}
