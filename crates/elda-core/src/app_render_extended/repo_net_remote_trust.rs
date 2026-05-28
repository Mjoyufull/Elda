use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;
use super::repo_net_remote::{remote_identity_lines, string_array};

pub(super) fn render_remote_trust_report(
    report: &CommandReport,
    details: &Value,
    remote: &Value,
    trust_report: &Value,
) -> String {
    let sections = vec![
        ("Remote".to_owned(), remote_identity_lines(remote, details)),
        ("Policy".to_owned(), remote_trust_policy_lines(trust_report)),
        ("Keys".to_owned(), remote_trust_key_lines(trust_report)),
        ("State".to_owned(), remote_trust_state_lines(trust_report)),
    ];
    human_framed_report(report, "Remote Trust", &sections, None)
}

pub(super) fn remote_trust_summary_lines(trust_report: &Value) -> Vec<String> {
    let mut lines = remote_trust_policy_lines(trust_report);
    lines.extend(remote_trust_state_lines(trust_report));
    lines
}

fn remote_trust_policy_lines(trust_report: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "trust", json_string(trust_report, &["trust"]));
    push_kv(
        &mut lines,
        "rotation",
        json_string(trust_report, &["rotation_policy"]),
    );
    push_kv(
        &mut lines,
        "payload verification",
        json_string(trust_report, &["payload_verification"]),
    );
    lines
}

fn remote_trust_key_lines(trust_report: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    let configured = string_array(trust_report.get("configured_trusted_keys"));
    lines.push(format!("configured trusted keys: {}", configured.len()));
    for key in configured.iter().take(8) {
        lines.push(format!("  {key}"));
    }
    let fingerprints = string_array(trust_report.get("persisted_trusted_fingerprints"));
    lines.push(format!("persisted fingerprints: {}", fingerprints.len()));
    if let Some(public_keys) = trust_report
        .get("persisted_trusted_public_keys")
        .and_then(Value::as_array)
    {
        lines.push(format!("persisted public keys: {}", public_keys.len()));
    }
    lines
}

fn remote_trust_state_lines(trust_report: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_bool(
        &mut lines,
        trust_report,
        "allow_stale",
        "allow stale snapshot",
    );
    push_bool(
        &mut lines,
        trust_report,
        "snapshot_present",
        "snapshot present",
    );
    push_bool(&mut lines, trust_report, "snapshot_verified", "verified");
    if let Some(stale) = trust_report.get("snapshot_stale").and_then(Value::as_bool) {
        lines.push(format!(
            "snapshot stale: {stale}{}",
            if stale {
                " (offline/stale-cache fallback may apply)"
            } else {
                ""
            }
        ));
    }
    push_kv(
        &mut lines,
        "selected key",
        json_string(trust_report, &["selected_key"]),
    );
    if let Some(last_sync) = json_u64(trust_report, &["last_sync_unix"]) {
        lines.push(format!("last sync unix: {last_sync}"));
    }
    if let Some(last_verified) = json_u64(trust_report, &["last_verified_unix"]) {
        lines.push(format!("last verified unix: {last_verified}"));
    }
    push_kv(
        &mut lines,
        "last error",
        json_string(trust_report, &["last_error"]),
    );
    if trust_report
        .get("pending_rotation")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        lines.push("pending key rotation: remote signing key changed".to_owned());
    }
    if trust_report
        .get("rotation_accept_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        lines.push(
            "action required: re-run sync with --accept-rotated-key <remote> after verifying signed metadata"
                .to_owned(),
        );
    }
    if trust_report
        .get("metadata_url")
        .and_then(Value::as_str)
        .is_some_and(|url| !url.is_empty())
    {
        lines.push(
            "rotation metadata: signed metadata_url may authorize TOFU key rotation".to_owned(),
        );
    }
    lines
}

fn push_bool(lines: &mut Vec<String>, value: &Value, field: &str, label: &str) {
    if let Some(flag) = value.get(field).and_then(Value::as_bool) {
        lines.push(format!("{label}: {flag}"));
    }
}
