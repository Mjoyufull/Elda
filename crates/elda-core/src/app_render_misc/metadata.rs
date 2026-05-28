use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, json_string, kv_label_width,
};
use crate::app_render_tree::FrameFooter;

pub(crate) fn render_metadata_add_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let add = details.get("metadata_add")?;
    let targets = add.get("targets")?.as_array()?;

    let labels = [
        "target", "recipe", "path", "pkg.lua", "strategy", "ready", "field", "option",
    ];
    let width = kv_label_width(&labels);
    let mut blocks = Vec::with_capacity(targets.len());

    for target in targets {
        let target_name = json_string(target, &["target"]).unwrap_or("<unknown>");
        let recipe_name = json_string(target, &["recipe_name"]).unwrap_or(target_name);
        let recipe_dir = json_string(target, &["recipe_dir"]).unwrap_or("<unknown>");
        let pkg_lua = json_string(target, &["pkg_lua"]).unwrap_or("<unknown>");
        let selected_lane = json_string(target, &["selected_lane"]).unwrap_or("unknown");
        let selected_source_kind =
            json_string(target, &["selected_source_kind"]).unwrap_or("unknown");
        let publish_ready = target
            .get("publish_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut lines = vec![
            format_aligned_kv("target", target_name, width),
            format_aligned_kv("recipe", recipe_name, width),
            format_aligned_kv("path", recipe_dir, width),
            format_aligned_kv("pkg.lua", pkg_lua, width),
            format_aligned_kv(
                "strategy",
                &format!("{selected_lane} / {selected_source_kind}"),
                width,
            ),
            format_aligned_kv("ready", &publish_ready.to_string(), width),
        ];
        lines.extend(source_option_lines_aligned(target, add, width));
        lines.extend(metadata_field_lines_aligned(target, width));

        blocks.push(human_operator_frame(
            format!("metadata add {recipe_name}"),
            lines,
            Some(FrameFooter {
                glyph: None,
                text: format!("Output: {recipe_dir}"),
            }),
        ));
    }

    Some(blocks.join("\n\n"))
}

fn metadata_field_lines_aligned(target: &Value, width: usize) -> Vec<String> {
    let Some(fields) = target.get("fields").and_then(Value::as_array) else {
        return Vec::new();
    };
    fields
        .iter()
        .map(|field| {
            let name = json_string(field, &["field"]).unwrap_or("unknown");
            let confidence = json_string(field, &["confidence"]).unwrap_or("unknown");
            format_aligned_kv("field", &format!("{name}: {confidence}"), width)
        })
        .collect()
}

fn source_option_lines_aligned(target: &Value, parent: &Value, width: usize) -> Vec<String> {
    let mode = parent
        .get("link_option_mode")
        .and_then(Value::as_str)
        .unwrap_or("priority");
    if mode != "list-options" {
        return Vec::new();
    }

    let Some(options) = target.get("source_options").and_then(Value::as_array) else {
        return Vec::new();
    };
    options
        .iter()
        .map(|option| {
            let index = option.get("index").and_then(Value::as_u64).unwrap_or(0);
            let marker = if option
                .get("selected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "*"
            } else {
                " "
            };
            let strategy = json_string(option, &["strategy"]).unwrap_or("unknown");
            let source_kind = json_string(option, &["source_kind"]).unwrap_or("unknown");
            let confidence = json_string(option, &["confidence"]).unwrap_or("unknown");
            let summary = json_string(option, &["summary"]).unwrap_or("detected source option");
            let mut detail =
                format!("{marker} {index}. {strategy} [{source_kind}, {confidence}] - {summary}");
            if let Some(tag) = json_string(option, &["tag"]) {
                detail.push_str(&format!(" tag={tag}"));
            }
            if let Some(asset) = json_string(option, &["asset"]) {
                detail.push_str(&format!(" asset={asset}"));
            }
            if let Some(compatibility) = json_string(option, &["compatibility"]) {
                detail.push_str(&format!(" compatibility={compatibility}"));
            }
            if option
                .get("checksum_available")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                detail.push_str(" checksum=sha256");
            }
            format_aligned_kv("option", &detail, width)
        })
        .collect()
}

pub(crate) fn source_option_lines(target: &Value, parent: &Value) -> Vec<String> {
    source_option_lines_aligned(target, parent, 8)
        .into_iter()
        .map(|line| line.replace("option  ", "  "))
        .collect()
}
