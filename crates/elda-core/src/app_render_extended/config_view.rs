use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

pub(super) fn render_config(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if let Some(diff) = details.get("config_diff") {
        return Some(render_config_diff(report, diff));
    }
    if let Some(action) = details.get("config_action") {
        return Some(render_config_action(report, action));
    }

    let config = details.get("config")?;
    let pending = config.get("pending").and_then(Value::as_array)?;
    let mut lines = vec![format!("pending files: {}", pending.len())];
    for record in pending.iter().take(32) {
        let package = json_string(record, &["package"]).unwrap_or("?");
        let path = json_string(record, &["path"]).unwrap_or("?");
        let state = json_string(record, &["state"]).unwrap_or("?");
        lines.push(format!("{package}: {path} ({state})"));
    }
    if pending.len() > 32 {
        lines.push(format!("... {} more", pending.len() - 32));
    }

    Some(human_framed_report(
        report,
        "Config Queue",
        &[("Pending".to_owned(), lines)],
        None,
    ))
}

fn render_config_diff(report: &CommandReport, diff: &Value) -> String {
    let mut summary = vec![
        format!(
            "package: {}",
            json_string(diff, &["package"]).unwrap_or("?")
        ),
        format!("path: {}", json_string(diff, &["path"]).unwrap_or("?")),
        format!("live: {}", json_string(diff, &["live_path"]).unwrap_or("?")),
        format!(
            "sidecar: {}",
            json_string(diff, &["sidecar_path"]).unwrap_or("?")
        ),
        format!(
            "sidecar kind: {}",
            json_string(diff, &["sidecar_kind"]).unwrap_or("?")
        ),
        format!(
            "changed: {}",
            diff.get("changed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        ),
    ];
    let diff_lines = diff
        .get("diff")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if diff_lines.is_empty() {
        summary.push("diff: unavailable".to_owned());
    }

    let mut sections = vec![("Target".to_owned(), summary)];
    if !diff_lines.is_empty() {
        sections.push(("Diff".to_owned(), diff_lines));
    }
    human_framed_report(report, "Config Diff", &sections, None)
}

fn render_config_action(report: &CommandReport, action: &Value) -> String {
    let lines = vec![
        format!(
            "action: {}",
            json_string(action, &["action"]).unwrap_or("?")
        ),
        format!(
            "package: {}",
            json_string(action, &["package"]).unwrap_or("?")
        ),
        format!("path: {}", json_string(action, &["path"]).unwrap_or("?")),
        format!(
            "live: {}",
            json_string(action, &["live_path"]).unwrap_or("?")
        ),
        format!(
            "sidecar: {}",
            json_string(action, &["sidecar_path"]).unwrap_or("none")
        ),
        format!(
            "changed: {}",
            action
                .get("changed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        ),
    ];
    human_framed_report(
        report,
        "Config Action",
        &[("Result".to_owned(), lines)],
        None,
    )
}
