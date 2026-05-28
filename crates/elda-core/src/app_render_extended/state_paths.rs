use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;

pub(super) fn render_state_files(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    match report.command_path.as_slice() {
        [a] if a == "files" => {
            let pkg = json_string(details, &["package"]).unwrap_or("?");
            let mut lines = vec![format!("package: {pkg}")];
            if let Some(files) = details.get("files").and_then(Value::as_array) {
                lines.push(format!("paths: {}", files.len()));
                for f in files.iter().take(40) {
                    let path = json_string(f, &["path"]).unwrap_or("?");
                    let kind = json_string(f, &["path_kind"]).unwrap_or("");
                    if kind.is_empty() {
                        lines.push(format!("  {path}"));
                    } else {
                        lines.push(format!("  [{kind}] {path}"));
                    }
                }
                if files.len() > 40 {
                    lines.push(format!("  … {} more", files.len() - 40));
                }
            }
            Some(human_framed_report(
                report,
                format!("Files: {pkg}"),
                &[("Paths".to_owned(), lines)],
                None,
            ))
        }
        [a, b] if a == "files" && b == "search" => {
            let query = json_string(details, &["query"]).unwrap_or("?");
            let mut lines = vec![format!("query: {query}")];
            if let Some(matches) = details.get("matches").and_then(Value::as_array) {
                lines.push(format!("matches: {}", matches.len()));
                for m in matches.iter().take(40) {
                    let path = json_string(m, &["path"]).unwrap_or("?");
                    let pkg = json_string(m, &["pkgname"]).unwrap_or("?");
                    let kind = json_string(m, &["path_kind"]).unwrap_or("");
                    let suffix = if kind.is_empty() {
                        pkg.to_owned()
                    } else {
                        format!("{pkg} {kind}")
                    };
                    lines.push(format!("  {path} [{suffix}]"));
                }
                if matches.len() > 40 {
                    lines.push(format!("  … {} more", matches.len() - 40));
                }
            }
            Some(human_framed_report(
                report,
                "File Search",
                &[("Matches".to_owned(), lines)],
                None,
            ))
        }
        [a, b] if a == "files" && b == "owner" => {
            let path = json_string(details, &["path"]).unwrap_or("?");
            let mut lines = vec![format!("path: {path}")];
            if let Some(owners) = details.get("owners").and_then(Value::as_array) {
                lines.push(format!("owners: {}", owners.len()));
                for o in owners.iter().take(24) {
                    let pname = json_string(o, &["pkgname"]).unwrap_or("?");
                    let ver = json_string(o, &["version"]).unwrap_or("");
                    lines.push(format!("  {pname} {ver}").trim_end().to_owned());
                }
                if owners.len() > 24 {
                    lines.push(format!("  … {} more", owners.len() - 24));
                }
            }
            Some(human_framed_report(
                report,
                "Path owners",
                &[("Owners".to_owned(), lines)],
                None,
            ))
        }
        _ => None,
    }
}

pub(super) fn render_state_export_import(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    match report.command_path.as_slice() {
        [a, b] if a == "state" && b == "export" => {
            let mut lines = Vec::new();
            if let Some(doc) = details.get("exported") {
                if let Some(v) = json_u64(doc, &["format_version"]) {
                    lines.push(format!("format version: {v}"));
                }
                push_kv(
                    &mut lines,
                    "installation mode",
                    json_string(doc, &["installation_mode"]),
                );
                push_kv(&mut lines, "prefix", json_string(doc, &["prefix"]));
                if let Some(r) = doc.get("remotes").and_then(Value::as_array) {
                    lines.push(format!("remotes: {}", r.len()));
                }
                if let Some(w) = doc.get("world").and_then(Value::as_array) {
                    lines.push(format!("world anchors: {}", w.len()));
                }
                if let Some(i) = doc.get("installed").and_then(Value::as_array) {
                    lines.push(format!("installed records: {}", i.len()));
                }
            }
            Some(human_framed_report(
                report,
                "State export",
                &[("Exported".to_owned(), lines)],
                None,
            ))
        }
        [a, b] if a == "state" && b == "import" => {
            let mut lines = Vec::new();
            if let Some(imp) = details.get("imported") {
                if let Some(r) = imp.get("remotes").and_then(Value::as_array) {
                    lines.push(format!("remotes written: {}", r.len()));
                }
                if let Some(w) = imp.get("world").and_then(Value::as_array) {
                    lines.push(format!("world targets: {}", w.len()));
                }
                if imp.get("profile").is_some() {
                    lines.push("profile: present".to_owned());
                }
                if imp.get("profile_backend_reconciliation").is_some() {
                    lines.push("profile backend reconciliation: present".to_owned());
                }
            }
            Some(human_framed_report(
                report,
                "State import",
                &[("Imported".to_owned(), lines)],
                None,
            ))
        }
        _ => None,
    }
}
