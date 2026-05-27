use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

use super::helpers::push_kv;

pub(super) fn render_vendor(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections = Vec::new();

    if let Some(vendor) = details.get("vendor") {
        sections.push(("Vendor add".to_owned(), vendor_lines(vendor)));
    }
    if let Some(imported) = details.get("import") {
        sections.push(("Vendor import".to_owned(), import_lines(imported)));
    }
    if let Some(exported) = details.get("export") {
        sections.push(("Vendor export".to_owned(), export_lines(exported)));
    }

    if sections.is_empty() {
        return None;
    }

    Some(human_framed_report(report, "Vendor", &sections, None))
}

fn vendor_lines(vendor: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(
        &mut lines,
        "package",
        json_string(vendor, &["package_name"]),
    );
    push_kv(&mut lines, "recipe", json_string(vendor, &["recipe_dir"]));
    push_kv(&mut lines, "source", json_string(vendor, &["source_kind"]));
    push_kv(&mut lines, "url", json_string(vendor, &["source_url"]));
    push_kv(&mut lines, "asset", json_string(vendor, &["asset"]));
    push_kv(&mut lines, "binary", json_string(vendor, &["binary"]));
    push_kv(&mut lines, "sha256", json_string(vendor, &["sha256"]));
    lines
}

fn import_lines(imported: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(
        &mut lines,
        "import",
        json_string(imported, &["source_path"]),
    );
    push_kv(&mut lines, "format", json_string(imported, &["format"]));
    if let Some(packages) = imported.get("packages").and_then(Value::as_array) {
        lines.push(format!("count: {}", packages.len()));
        for package in packages.iter().take(16) {
            let name = json_string(package, &["package_name"])
                .or_else(|| json_string(package, &["pkgname"]))
                .unwrap_or("?");
            let kind = json_string(package, &["source_kind"]).unwrap_or("unknown");
            let recipe = json_string(package, &["recipe_dir"]).unwrap_or("");
            if recipe.is_empty() {
                lines.push(format!("entry: {name}: {kind}"));
            } else {
                lines.push(format!("entry: {name}: {kind} {recipe}"));
            }
        }
        push_more(&mut lines, packages.len(), 16);
    }
    lines
}

fn export_lines(exported: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(
        &mut lines,
        "output",
        json_string(exported, &["output_path"]),
    );
    push_kv(&mut lines, "format", json_string(exported, &["format"]));
    if let Some(packages) = exported.get("packages").and_then(Value::as_array) {
        lines.push(format!("count: {}", packages.len()));
        for package in packages.iter().take(24).filter_map(Value::as_str) {
            lines.push(format!("entry: {package}"));
        }
        push_more(&mut lines, packages.len(), 24);
    }
    lines
}

fn push_more(lines: &mut Vec<String>, total: usize, shown: usize) {
    if total > shown {
        lines.push(format!("more: {}", total - shown));
    }
}
