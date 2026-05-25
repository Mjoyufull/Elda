use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, json_string, kv_label_width, push_aligned_kv,
};

pub(super) fn render_vendor(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let labels = [
        "package", "recipe", "source", "url", "asset", "binary", "sha256", "import", "format",
        "count", "entry", "more", "output",
    ];
    let width = kv_label_width(&labels);
    let mut lines = Vec::new();

    if let Some(vendor) = details.get("vendor") {
        push_aligned_kv(
            &mut lines,
            "package",
            json_string(vendor, &["package_name"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "recipe",
            json_string(vendor, &["recipe_dir"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "source",
            json_string(vendor, &["source_kind"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "url",
            json_string(vendor, &["source_url"]),
            width,
        );
        push_aligned_kv(&mut lines, "asset", json_string(vendor, &["asset"]), width);
        push_aligned_kv(
            &mut lines,
            "binary",
            json_string(vendor, &["binary"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "sha256",
            json_string(vendor, &["sha256"]),
            width,
        );
    }

    if let Some(imported) = details.get("import") {
        push_aligned_kv(
            &mut lines,
            "import",
            json_string(imported, &["source_path"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "format",
            json_string(imported, &["format"]),
            width,
        );
        if let Some(packages) = imported.get("packages").and_then(Value::as_array) {
            lines.push(format_aligned_kv(
                "count",
                &packages.len().to_string(),
                width,
            ));
            for package in packages.iter().take(16) {
                let name = json_string(package, &["package_name"])
                    .or_else(|| json_string(package, &["pkgname"]))
                    .unwrap_or("?");
                let kind = json_string(package, &["source_kind"]).unwrap_or("unknown");
                let recipe = json_string(package, &["recipe_dir"]).unwrap_or("");
                let value = if recipe.is_empty() {
                    format!("{name}: {kind}")
                } else {
                    format!("{name}: {kind} {recipe}")
                };
                lines.push(format_aligned_kv("entry", &value, width));
            }
            if packages.len() > 16 {
                lines.push(format_aligned_kv(
                    "more",
                    &format!("{}", packages.len() - 16),
                    width,
                ));
            }
        }
    }

    if let Some(exported) = details.get("export") {
        push_aligned_kv(
            &mut lines,
            "output",
            json_string(exported, &["output_path"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "format",
            json_string(exported, &["format"]),
            width,
        );
        if let Some(packages) = exported.get("packages").and_then(Value::as_array) {
            lines.push(format_aligned_kv(
                "count",
                &packages.len().to_string(),
                width,
            ));
            for package in packages.iter().take(24).filter_map(Value::as_str) {
                lines.push(format_aligned_kv("entry", package, width));
            }
            if packages.len() > 24 {
                lines.push(format_aligned_kv(
                    "more",
                    &format!("{}", packages.len() - 24),
                    width,
                ));
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    Some(human_operator_frame("vendor", lines, None))
}
