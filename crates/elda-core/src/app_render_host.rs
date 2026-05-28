use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{render_header, render_section};

pub(crate) fn render_host_report(report: &CommandReport) -> Option<String> {
    if report.area != "host" {
        return None;
    }
    let details = report.details.as_ref()?;
    if let Some(scan) = details.get("scan") {
        return Some(render_scan_tree(report, scan));
    }
    if details.get("remotes_fragment").is_some() || details.get("commands").is_some() {
        return Some(render_client_bundle(report, details));
    }
    None
}

fn render_scan_tree(report: &CommandReport, scan: &Value) -> String {
    let mut lines = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    if let Some(packages) = scan.get("packages").and_then(Value::as_array) {
        let mut rows = Vec::new();
        for package in packages {
            let name = package
                .get("package")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let status = package
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let blockers = package
                .get("blockers")
                .and_then(Value::as_array)
                .map(|values| values.len())
                .unwrap_or(0);
            rows.push(format!("{name:<24} {status:<8} blockers={blockers}"));
        }
        lines.push(render_section("Packages", &rows));
    }
    lines.join("\n")
}

fn render_client_bundle(report: &CommandReport, details: &Value) -> String {
    let mut lines = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    if let Some(fragment) = details.get("remotes_fragment").and_then(Value::as_str) {
        lines.push(render_section("Remotes fragment", &[fragment.to_owned()]));
    }
    if let Some(commands) = details.get("commands").and_then(Value::as_object) {
        let mut rows = Vec::new();
        for (key, value) in commands {
            if let Some(command) = value.as_str() {
                rows.push(format!("{key}: {command}"));
            }
        }
        lines.push(render_section("Suggested commands", &rows));
    }
    lines.join("\n")
}
