use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;
use super::repo_net_remote_trust::{remote_trust_summary_lines, render_remote_trust_report};

pub(super) fn render_remote(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if let Some(remotes) = details.get("remotes").and_then(Value::as_array) {
        return Some(render_remote_list_report(report, remotes));
    }
    let remote = details.get("remote")?;

    if let Some(preview) = details.get("preview") {
        return Some(render_remote_preview_report(
            report, details, remote, preview,
        ));
    }
    if details
        .get("trust_command")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && let Some(trust_report) = details.get("trust_report")
    {
        return Some(render_remote_trust_report(
            report,
            details,
            remote,
            trust_report,
        ));
    }

    let mut sections = Vec::new();
    let title = if details
        .get("removed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "Removed"
    } else if details
        .get("info")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "Remote"
    } else {
        "Registered"
    };
    sections.push((title.to_owned(), remote_identity_lines(remote, details)));

    if let Some(snapshot) = details.get("snapshot") {
        sections.push(("Sync state".to_owned(), remote_snapshot_lines(snapshot)));
    }

    if let Some(trust_report) = details.get("trust_report") {
        sections.push(("Trust".to_owned(), remote_trust_summary_lines(trust_report)));
    }

    let indexed = string_array(details.get("indexed_packages"));
    if !indexed.is_empty() {
        sections.push((
            "Catalog".to_owned(),
            counted_name_lines("indexed packages", &indexed),
        ));
    }

    if let Some(installed) = details.get("installed_packages").and_then(Value::as_array) {
        let mut lines = vec![format!("installed from remote: {}", installed.len())];
        for package in installed.iter().take(12) {
            let name = json_string(package, &["pkgname"]).unwrap_or("?");
            let version = json_string(package, &["version"]).unwrap_or("?");
            let reason = json_string(package, &["install_reason"]).unwrap_or("?");
            lines.push(format!("  {name} {version} ({reason})"));
        }
        if installed.len() > 12 {
            lines.push(format!("  ... {} more", installed.len() - 12));
        }
        sections.push(("Install impact".to_owned(), lines));
    }

    Some(human_framed_report(report, "Remote", &sections, None))
}

fn render_remote_list_report(report: &CommandReport, remotes: &[Value]) -> String {
    let mut lines = vec![format!("configured remotes: {}", remotes.len())];
    for remote in remotes.iter().take(24) {
        let name = json_string(remote, &["name"]).unwrap_or("?");
        let url = json_string(remote, &["index_url"]).unwrap_or("?");
        let channel = json_string(remote, &["channel"]).unwrap_or("stable");
        let priority = json_u64(remote, &["priority"]).unwrap_or(100);
        let enabled = remote
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let trust = json_string(remote, &["trust"]).unwrap_or("tofu");
        let kind = if url.ends_with(".toml") || url.ends_with(".json") || url.ends_with(".idx") {
            "index"
        } else {
            "interemote"
        };
        lines.push(format!(
            "  {name}: {kind}, enabled={enabled}, channel={channel}, trust={trust}, priority={priority}"
        ));
        lines.push(format!("    {url}"));
    }
    if remotes.len() > 24 {
        lines.push(format!("  ... {} more", remotes.len() - 24));
    }
    human_framed_report(report, "Remotes", &[("Configured".to_owned(), lines)], None)
}

fn render_remote_preview_report(
    report: &CommandReport,
    details: &Value,
    remote: &Value,
    preview: &Value,
) -> String {
    let mut sections = Vec::new();
    sections.push(("Remote".to_owned(), remote_identity_lines(remote, details)));

    let mut catalog = Vec::new();
    if let Some(discovered) = json_u64(preview, &["discovered_count"]) {
        catalog.push(format!("discovered: {discovered}"));
    }
    if let Some(included) = json_u64(preview, &["included_count"]) {
        catalog.push(format!("included: {included}"));
    }
    if let Some(excluded) = json_u64(preview, &["excluded_count"]) {
        catalog.push(format!("excluded by policy: {excluded}"));
    }
    if let Some(parseable) = json_u64(preview, &["parseable_count"]) {
        catalog.push(format!("parseable in preview: {parseable}"));
    }
    if let Some(commit) = json_string(preview, &["commit"])
        && !commit.is_empty()
    {
        catalog.push(format!("commit: {commit}"));
    }
    sections.push(("Catalog".to_owned(), catalog));

    let mut parser = Vec::new();
    push_kv(&mut parser, "kind", json_string(preview, &["kind"]));
    push_kv(&mut parser, "parser", json_string(preview, &["parser"]));
    push_kv(
        &mut parser,
        "source kind",
        json_string(preview, &["source_kind"]),
    );
    let fields = string_array(preview.get("metadata_fields"));
    if !fields.is_empty() {
        parser.push(format!("metadata fields: {}", fields.join(", ")));
    }
    sections.push(("Translation".to_owned(), parser));

    let excludes = string_array(preview.get("configured_excludes"));
    if !excludes.is_empty() {
        let matched = string_array(preview.get("matched_excludes"));
        let mut lines = vec![format!("configured: {}", excludes.join(", "))];
        if !matched.is_empty() {
            lines.push(format!("matched: {}", matched.join(", ")));
        }
        sections.push(("Policy".to_owned(), lines));
    }

    if let Some(packages) = preview.get("packages").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for package in packages {
            let name = json_string(package, &["name"]).unwrap_or("?");
            let version = json_string(package, &["version"]).unwrap_or("?");
            let path = json_string(package, &["package_path"]).unwrap_or("?");
            let mut line = format!("  {name} {version} ({path})");
            if let Some(issue) = json_string(package, &["issue"])
                && !issue.is_empty()
            {
                line.push_str(&format!(" - issue: {issue}"));
            }
            lines.push(line);
        }
        sections.push(("Preview".to_owned(), lines));
    }

    human_framed_report(report, "Interemote Preview", &sections, None)
}

pub(super) fn remote_identity_lines(remote: &Value, details: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "name", json_string(remote, &["name"]));
    push_kv(&mut lines, "index URL", json_string(remote, &["index_url"]));
    push_kv(&mut lines, "kind", json_string(details, &["kind"]));
    if json_string(details, &["kind"]).is_none()
        && let Some(url) = json_string(remote, &["index_url"])
    {
        let kind = if url.ends_with(".toml") || url.ends_with(".json") || url.ends_with(".idx") {
            "index"
        } else {
            "interemote"
        };
        lines.push(format!("kind: {kind}"));
    }
    push_kv(&mut lines, "channel", json_string(remote, &["channel"]));
    push_kv(
        &mut lines,
        "packages URL",
        json_string(remote, &["packages_url"]),
    );
    push_kv(
        &mut lines,
        "metadata URL",
        json_string(remote, &["metadata_url"]),
    );
    push_kv(
        &mut lines,
        "signature URL",
        json_string(remote, &["signature_url"]),
    );
    if let Some(p) = json_u64(remote, &["priority"]) {
        lines.push(format!("priority: {p}"));
    }
    if let Some(en) = remote.get("enabled").and_then(Value::as_bool) {
        lines.push(format!("enabled: {en}"));
    }
    if let Some(stale) = remote.get("allow_stale").and_then(Value::as_bool) {
        lines.push(format!("allow stale: {stale}"));
    }
    push_kv(
        &mut lines,
        "trust",
        remote.get("trust").and_then(Value::as_str),
    );
    let exclude = string_array(remote.get("exclude"));
    if !exclude.is_empty() {
        lines.push(format!("exclude: {}", exclude.join(", ")));
    }
    lines
}

fn remote_snapshot_lines(snapshot: &Value) -> Vec<String> {
    if snapshot.is_null() {
        return vec!["synced: no local snapshot record".to_owned()];
    }
    let mut lines = Vec::new();
    if let Some(packages) = json_u64(snapshot, &["package_count"]) {
        lines.push(format!("packages indexed: {packages}"));
    }
    if let Some(source) = json_string(snapshot, &["source"]) {
        lines.push(format!("source: {source}"));
    }
    if let Some(verified) = snapshot.get("verified").and_then(Value::as_bool) {
        lines.push(format!("verified: {verified}"));
    }
    if let Some(stale) = snapshot.get("stale").and_then(Value::as_bool) {
        lines.push(format!("stale: {stale}"));
    }
    push_kv(
        &mut lines,
        "selected key",
        json_string(snapshot, &["selected_key"]),
    );
    push_kv(&mut lines, "issue", json_string(snapshot, &["issue"]));
    lines
}

fn counted_name_lines(label: &str, names: &[String]) -> Vec<String> {
    let mut lines = vec![format!("{label}: {}", names.len())];
    for name in names.iter().take(16) {
        lines.push(format!("  {name}"));
    }
    if names.len() > 16 {
        lines.push(format!("  ... {} more", names.len() - 16));
    }
    lines
}

pub(super) fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
