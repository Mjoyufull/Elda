use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, json_string, kv_label_width,
};

pub(crate) fn render_publish_report(report: &CommandReport) -> Option<String> {
    if report.area != "publish" {
        return None;
    }
    let details = report.details.as_ref()?;
    let labels = ["channel", "target", "package", "layer", "path"];
    let width = kv_label_width(&labels);
    let mut lines = Vec::new();
    push_aligned(
        &mut lines,
        "channel",
        json_string(details, &["channel"]),
        width,
    );
    if let Some(targets) = details.get("requested_targets").and_then(Value::as_array) {
        for target in targets.iter().filter_map(Value::as_str).take(16) {
            lines.push(format_aligned_kv("target", target, width));
        }
    }
    if let Some(packages) = details.get("packages").and_then(Value::as_array) {
        for package in packages.iter().take(16) {
            let name = json_string(package, &["package"]).unwrap_or("?");
            let layer = json_string(package, &["layer"]).unwrap_or("?");
            lines.push(format_aligned_kv(
                "package",
                &format!("{name} layer={layer}"),
                width,
            ));
            push_aligned(
                &mut lines,
                "path",
                json_string(package, &["recipe_path"]),
                width,
            );
        }
    }
    if lines.is_empty() {
        return None;
    }
    Some(human_operator_frame("publish plan", lines, None))
}

fn push_aligned(lines: &mut Vec<String>, label: &str, value: Option<&str>, width: usize) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        lines.push(format_aligned_kv(label, value, width));
    }
}
