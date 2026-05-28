use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;

pub(super) fn render_sync(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let sync = details.get("sync")?;
    let mut lines = Vec::new();
    push_kv(
        &mut lines,
        "snapshot",
        sync.get("snapshot_path").and_then(Value::as_str),
    );
    if let Some(offline) = sync.get("offline").and_then(Value::as_bool) {
        lines.push(format!("offline: {offline}"));
    }
    if let Some(n) = json_u64(sync, &["remote_count"]) {
        lines.push(format!("remotes: {n}"));
    }
    if let Some(n) = json_u64(sync, &["package_count"]) {
        lines.push(format!("packages indexed: {n}"));
    }
    if let Some(n) = json_u64(sync, &["verified_remote_count"]) {
        lines.push(format!("verified remotes: {n}"));
    }
    if let Some(n) = json_u64(sync, &["stale_remote_count"]) {
        lines.push(format!("stale remotes: {n}"));
    }
    if let Some(n) = json_u64(sync, &["failed_remote_count"]) {
        lines.push(format!("failed remotes: {n}"));
    }
    if let Some(remotes) = sync.get("remotes").and_then(Value::as_array) {
        for r in remotes.iter().take(16) {
            let name = json_string(r, &["name"]).unwrap_or("?");
            let pkgs = json_u64(r, &["package_count"]).unwrap_or(0);
            let verified = r.get("verified").and_then(Value::as_bool).unwrap_or(false);
            let stale = r.get("stale").and_then(Value::as_bool).unwrap_or(false);
            let issue = json_string(r, &["issue"]).unwrap_or("");
            let mut s = format!("  {name}: {pkgs} packages, verified={verified}, stale={stale}");
            if !issue.is_empty() {
                s.push_str(&format!(" ({issue})"));
            }
            lines.push(s);
        }
        if remotes.len() > 16 {
            lines.push(format!("  … {} more remotes", remotes.len() - 16));
        }
    }
    if let Some(deltas) = sync.get("package_deltas").and_then(Value::as_array) {
        for delta in deltas.iter().take(12) {
            let added = json_u64(delta, &["added_count"]).unwrap_or(0);
            let removed = json_u64(delta, &["removed_count"]).unwrap_or(0);
            if added == 0 && removed == 0 {
                continue;
            }
            let name = json_string(delta, &["remote_name"]).unwrap_or("?");
            let previous = json_u64(delta, &["previous_count"]).unwrap_or(0);
            let current = json_u64(delta, &["current_count"]).unwrap_or(0);
            let kept = json_u64(delta, &["kept_count"]).unwrap_or(0);
            lines.push(format!(
                "  delta {name}: +{added} -{removed}, kept {kept}, {previous} -> {current} packages"
            ));
            if let Some(removed_packages) = delta.get("removed_packages").and_then(Value::as_array)
            {
                let names = removed_packages
                    .iter()
                    .filter_map(Value::as_str)
                    .take(8)
                    .collect::<Vec<_>>();
                if !names.is_empty() {
                    lines.push(format!("    stale removed: {}", names.join(", ")));
                }
            }
            if let Some(added_packages) = delta.get("added_packages").and_then(Value::as_array) {
                let names = added_packages
                    .iter()
                    .filter_map(Value::as_str)
                    .take(8)
                    .collect::<Vec<_>>();
                if !names.is_empty() {
                    lines.push(format!("    added: {}", names.join(", ")));
                }
            }
        }
    }
    if let Some(interemotes) = sync.get("interemotes").and_then(Value::as_array) {
        for remote in interemotes.iter().take(8) {
            let name = json_string(remote, &["remote_name"]).unwrap_or("?");
            let kind = json_string(remote, &["kind"]).unwrap_or("unknown");
            let parser = json_string(remote, &["parser"]).unwrap_or("unknown parser");
            let included = json_u64(remote, &["included_count"]).unwrap_or(0);
            let discovered = json_u64(remote, &["discovered_count"]).unwrap_or(0);
            let excluded = json_u64(remote, &["excluded_count"]).unwrap_or(0);
            let parseable = json_u64(remote, &["parseable_count"]).unwrap_or(0);
            let commit = json_string(remote, &["commit"]).unwrap_or("unrecorded");
            lines.push(format!(
                "  interemote {name}: {kind}, {included}/{discovered} included, {excluded} excluded, {parseable} parseable, commit {commit}"
            ));
            lines.push(format!("    parser: {parser}"));
            if let Some(excludes) = remote.get("matched_excludes").and_then(Value::as_array) {
                let names = excludes
                    .iter()
                    .filter_map(Value::as_str)
                    .take(8)
                    .collect::<Vec<_>>();
                if !names.is_empty() {
                    lines.push(format!("    excluded: {}", names.join(", ")));
                }
            }
            if let Some(issues) = remote.get("issues").and_then(Value::as_array)
                && !issues.is_empty()
            {
                lines.push(format!("    parser issues: {}", issues.len()));
                for issue in issues.iter().take(3) {
                    let package = json_string(issue, &["name"]).unwrap_or("?");
                    let reason = json_string(issue, &["issue"]).unwrap_or("unknown parser issue");
                    lines.push(format!("      {package}: {reason}"));
                }
                if issues.len() > 3 {
                    lines.push(format!("      … {} more parser issue(s)", issues.len() - 3));
                }
            }
        }
        if interemotes.len() > 8 {
            lines.push(format!("  … {} more interemotes", interemotes.len() - 8));
        }
    }
    Some(human_framed_report(
        report,
        "Sync",
        &[("Result".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_cache(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(cache) = details.get("cache") {
        let mut lines = Vec::new();
        push_kv(&mut lines, "name", json_string(cache, &["name"]));
        push_kv(&mut lines, "base URL", json_string(cache, &["base_url"]));
        if let Some(p) = json_u64(cache, &["priority"]) {
            lines.push(format!("priority: {p}"));
        }
        if let Some(en) = cache.get("enabled").and_then(Value::as_bool) {
            lines.push(format!("enabled: {en}"));
        }
        sections.push(("Cache".to_owned(), lines));
    }

    if let Some(caches) = details.get("caches").and_then(Value::as_array) {
        let mut lines = vec![format!("configured: {}", caches.len())];
        for c in caches.iter().take(20) {
            let name = json_string(c, &["name"]).unwrap_or("?");
            let prio = json_u64(c, &["priority"]).unwrap_or(0);
            lines.push(format!("  {name} (priority {prio})"));
        }
        if caches.len() > 20 {
            lines.push(format!("  … {} more", caches.len() - 20));
        }
        sections.push(("Caches".to_owned(), lines));
    }

    if let Some(policy) = details.get("policy") {
        let mut lines = Vec::new();
        if let Some(d) = json_u64(policy, &["payload_retention_days"]) {
            lines.push(format!("payload retention: {d} days"));
        }
        if let Some(d) = json_u64(policy, &["source_retention_days"]) {
            lines.push(format!("source retention: {d} days"));
        }
        if let Some(b) = json_u64(policy, &["usage_bytes"]) {
            lines.push(format!("usage: {b} bytes"));
        }
        if let Some(b) = json_u64(policy, &["effective_trigger_bytes"]) {
            lines.push(format!("effective cleanup trigger: {b} bytes"));
        }
        if let Some(n) = policy.get("needs_cleanup").and_then(Value::as_bool) {
            lines.push(format!("needs cleanup: {n}"));
        }
        sections.push(("Disk policy".to_owned(), lines));
    }

    if sections.is_empty() {
        return None;
    }

    Some(human_framed_report(report, "Cache", &sections, None))
}

pub(super) fn render_daemon(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();
    if let Some(present) = details.get("snapshot_present").and_then(Value::as_bool) {
        lines.push(format!("snapshot present: {present}"));
    }
    push_kv(
        &mut lines,
        "snapshot path",
        details.get("snapshot_path").and_then(Value::as_str),
    );
    if details.get("snapshot").is_some_and(|s| !s.is_null()) {
        lines.push("snapshot loaded: yes".to_owned());
    }
    Some(human_framed_report(
        report,
        "Daemon",
        &[("Snapshot".to_owned(), lines)],
        None,
    ))
}
