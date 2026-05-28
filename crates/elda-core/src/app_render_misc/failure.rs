use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, json_string, kv_label_width,
};
use crate::app_render_tree::{FrameFooter, Glyph};

pub(crate) fn render_failure_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let blocked = json_string(details, &["blocked"]).unwrap_or(&report.summary);
    let kind = json_string(details, &["kind"]).unwrap_or("failure");
    let command = failure_command(details);
    let next_action =
        json_string(details, &["next_action"]).unwrap_or("fix the reported input and retry");

    let frame_command = if report.operands.is_empty() {
        command.clone()
    } else {
        report.operands.join(" ")
    };
    let labels = [
        "reason", "kind", "command", "dry-run", "system", "offline", "cause", "action",
    ];
    let width = kv_label_width(&labels);
    let mut lines = vec![
        format_aligned_kv("reason", blocked, width),
        format_aligned_kv("kind", kind, width),
        format_aligned_kv("command", &command, width),
    ];

    let dry_run = details
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let system_mode = details
        .get("system_mode")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let offline = details
        .get("offline")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    lines.push(format_aligned_kv("dry-run", &dry_run.to_string(), width));
    lines.push(format_aligned_kv("system", &system_mode.to_string(), width));
    lines.push(format_aligned_kv("offline", &offline.to_string(), width));

    for cause in details
        .get("causes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        lines.push(format_aligned_kv("cause", cause, width));
    }

    lines.push(format_aligned_kv("action", next_action, width));

    Some(human_operator_frame(
        format!("{} blocked: {frame_command}", report.area),
        lines,
        Some(FrameFooter {
            glyph: Some(Glyph::Blocked),
            text: report.status.to_owned(),
        }),
    ))
}

fn failure_command(details: &Value) -> String {
    let command_path = details
        .get("command_path")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let operands = details
        .get("operands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if command_path.is_empty() && operands.is_empty() {
        return "elda".to_owned();
    }

    ["elda"]
        .into_iter()
        .chain(command_path)
        .chain(operands)
        .collect::<Vec<_>>()
        .join(" ")
}
