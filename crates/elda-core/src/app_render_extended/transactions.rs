use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;

pub(super) fn render_upgrade(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();
    if let Some(actions) = details.get("actions").and_then(Value::as_array) {
        lines.push(format!("actions evaluated: {}", actions.len()));
        for a in actions.iter().take(18) {
            let action = json_string(a, &["action"]).unwrap_or("?");
            let target = json_string(a, &["target"])
                .or_else(|| json_string(a, &["package"]))
                .unwrap_or("?");
            let cand = json_string(a, &["candidate_version"]).unwrap_or("");
            let replacement = replacement_detail(a)
                .map(|detail| format!(" ({detail})"))
                .unwrap_or_default();
            if cand.is_empty() {
                lines.push(format!("  {action} {target}{replacement}"));
            } else {
                lines.push(format!("  {action} {target} -> {cand}{replacement}"));
            }
        }
        if actions.len() > 18 {
            lines.push(format!("  … {} more", actions.len() - 18));
        }
    }
    if let Some(up) = details.get("upgrades").and_then(Value::as_array) {
        lines.push(format!("install reports: {}", up.len()));
        for u in up.iter().take(12) {
            let pname = json_string(u, &["package_name"])
                .or_else(|| json_string(u, &["install", "package_name"]))
                .or_else(|| json_string(u, &["target"]))
                .unwrap_or("?");
            let paths = json_u64(u, &["installed_paths"])
                .or_else(|| json_u64(u, &["install", "installed_paths"]))
                .unwrap_or(0);
            let replacement = applied_replacement_detail(u)
                .map(|detail| format!(", {detail}"))
                .unwrap_or_default();
            lines.push(format!("  {pname}: {paths} paths{replacement}"));
        }
        if up.len() > 12 {
            lines.push(format!("  … {} more", up.len() - 12));
        }
    }
    Some(human_framed_report(
        report,
        "Upgrade",
        &[("Summary".to_owned(), lines)],
        None,
    ))
}

fn replacement_detail(action: &Value) -> Option<String> {
    let targets = string_array(action, "replaced_packages");
    if targets.is_empty() {
        None
    } else {
        Some(format!("replaces {}", targets.join(", ")))
    }
}

fn applied_replacement_detail(action: &Value) -> Option<String> {
    let targets = action
        .get("replacements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            json_string(entry, &["package_name"]).or_else(|| json_string(entry, &["pkgname"]))
        })
        .collect::<Vec<_>>();
    if targets.is_empty() {
        None
    } else {
        Some(format!("replaced {}", targets.join(", ")))
    }
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(super) fn render_remove(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let removals = details
        .get("removals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = vec![format!("removals: {}", removals.len())];
    for r in removals.iter().take(24) {
        let pname = json_string(r, &["package_name"]).unwrap_or("?");
        let paths = json_u64(r, &["removed_paths"]).unwrap_or(0);
        lines.push(format!("  {pname}: removed {paths} path(s)"));
    }
    if removals.len() > 24 {
        lines.push(format!("  … {} more", removals.len() - 24));
    }
    Some(human_framed_report(
        report,
        "Remove",
        &[("Removals".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_downgrade(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if json_string(details, &["kind"]) == Some("source-ref-downgrade") {
        let mut lines = Vec::new();
        push_kv(&mut lines, "package", json_string(details, &["package"]));
        push_kv(
            &mut lines,
            "installed",
            json_string(details, &["installed_version"]),
        );
        if let Some(actions) = details.get("actions").and_then(Value::as_array) {
            lines.push(format!("actions: {}", actions.len()));
        }
        if let Some(up) = details.get("upgrades").and_then(Value::as_array) {
            lines.push(format!("upgrade reports: {}", up.len()));
        }
        return Some(human_framed_report(
            report,
            "Source-ref downgrade",
            &[("Summary".to_owned(), lines)],
            None,
        ));
    }

    let mut lines = Vec::new();
    push_kv(&mut lines, "package", json_string(details, &["package"]));
    push_kv(
        &mut lines,
        "installed",
        json_string(details, &["installed_version"]),
    );
    if let Some(cand) = details.get("candidate") {
        push_kv(
            &mut lines,
            "candidate state",
            json_string(cand, &["state_id"]),
        );
        if let (Some(e), Some(pv), Some(pr)) = (
            json_u64(cand, &["epoch"]),
            json_string(cand, &["pkgver"]),
            json_u64(cand, &["pkgrel"]),
        ) {
            lines.push(format!("candidate version: {e}:{pv}-{pr}"));
        }
        push_kv(
            &mut lines,
            "source kind",
            json_string(cand, &["source_kind"]),
        );
    }
    Some(human_framed_report(
        report,
        "Downgrade",
        &[("Summary".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_recovery(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let rec = details.get("recovery")?;
    let recovered = rec
        .get("recovered")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut lines = vec![format!("recovered journals: {}", recovered.len())];
    for entry in recovered.iter().take(20) {
        let jid = json_string(entry, &["journal_id"]).unwrap_or("?");
        let pkg = json_string(entry, &["package_name"]).unwrap_or("?");
        let action = json_string(entry, &["action"]).unwrap_or("");
        lines.push(format!("  {jid} {pkg} ({action})"));
    }
    if recovered.len() > 20 {
        lines.push(format!("  … {} more", recovered.len() - 20));
    }
    Some(human_framed_report(
        report,
        "Recovery",
        &[("Journals".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_rollback_done(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let rb = details.get("rollback")?;
    let mut lines = Vec::new();
    let from = rb
        .get("from_state")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    lines.push(format!("from state: {from}"));
    push_kv(&mut lines, "to state", json_string(rb, &["to_state"]));
    if let Some(rm) = rb.get("removed_packages").and_then(Value::as_array) {
        lines.push(format!("packages removed: {}", rm.len()));
    }
    if let Some(rs) = rb.get("restored_packages").and_then(Value::as_array) {
        lines.push(format!("packages restored: {}", rs.len()));
        for p in rs.iter().take(12) {
            let pname = json_string(p, &["package_name"]).unwrap_or("?");
            let paths = json_u64(p, &["installed_paths"]).unwrap_or(0);
            lines.push(format!("  {pname}: {paths} paths"));
        }
        if rs.len() > 12 {
            lines.push(format!("  … {} more", rs.len() - 12));
        }
    }
    Some(human_framed_report(
        report,
        "Rollback",
        &[("Result".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_ops(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();
    if let Some(p) = details.get("pending_handlers").and_then(Value::as_array) {
        lines.push(format!("pending handlers: {}", p.len()));
    }
    if let Some(p) = details
        .get("provider_asset_repair")
        .and_then(Value::as_object)
    {
        lines.push(format!("provider asset repair keys: {}", p.len()));
    }
    if details.get("trigger_repair").is_some() {
        lines.push("trigger repair report: present".to_owned());
    }
    if let Some(b) = details.get("backend") {
        if let Some(s) = b.as_str() {
            lines.push(format!("backend: {s}"));
        } else {
            lines.push("backend: (structured)".to_owned());
        }
    }
    Some(human_framed_report(
        report,
        "Operators",
        &[("Backend ops".to_owned(), lines)],
        None,
    ))
}
