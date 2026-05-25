use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, human_square_report, json_string, kv_label_width,
    push_aligned_kv,
};

use super::helpers::package_version;

pub(super) fn render_forge(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if let Some(results) = details.get("results").and_then(Value::as_array) {
        return Some(render_forge_search(report, details, results));
    }

    if let Some(package) = details.get("package") {
        return Some(render_forge_package(report, package));
    }

    if details.get("repo").is_some() {
        let labels = ["repo", "tool", "forked", "url", "name"];
        let width = kv_label_width(&labels);
        let mut lines = Vec::new();
        push_aligned_kv(&mut lines, "repo", json_string(details, &["repo"]), width);
        push_aligned_kv(&mut lines, "tool", json_string(details, &["tool"]), width);
        if let Some(forked) = details.get("forked").and_then(Value::as_bool) {
            lines.push(format_aligned_kv("forked", &forked.to_string(), width));
        }
        push_aligned_kv(
            &mut lines,
            "url",
            json_string(details, &["result", "url"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "name",
            json_string(details, &["result", "nameWithOwner"]),
            width,
        );
        return Some(human_operator_frame("forge fork", lines, None));
    }

    None
}

fn render_forge_search(report: &CommandReport, details: &Value, results: &[Value]) -> String {
    let query = json_string(details, &["query"]).unwrap_or("?");
    let mut lines = vec![
        format!("query    {query}"),
        format!("matches  {}", results.len()),
        String::new(),
    ];
    for result in results.iter().take(16) {
        let package = json_string(result, &["pkgname"]).unwrap_or("?");
        let source = json_string(result, &["source"]).unwrap_or("unknown");
        let path = json_string(result, &["packages_repo_path"])
            .or_else(|| json_string(result, &["payload_path"]))
            .or_else(|| json_string(result, &["index_path"]))
            .unwrap_or("");
        if path.is_empty() {
            lines.push(format!("{package:<24}  {source}"));
        } else {
            lines.push(format!("{package:<24}  {source}  {path}"));
        }
    }
    if results.len() > 16 {
        lines.push(format!("... {} more", results.len() - 16));
    }
    human_square_report(report, "forge search", lines, None)
}

fn render_forge_package(_report: &CommandReport, package: &Value) -> String {
    let title = json_string(package, &["package"]).unwrap_or("?");
    let labels = [
        "package",
        "channel",
        "recipe",
        "published",
        "repo",
        "index",
        "version",
        "arch",
        "payload",
        "commit",
    ];
    let width = kv_label_width(&labels);
    let mut lines = Vec::new();
    push_aligned_kv(
        &mut lines,
        "package",
        json_string(package, &["package"]),
        width,
    );
    push_aligned_kv(
        &mut lines,
        "channel",
        json_string(package, &["channel"]),
        width,
    );
    push_aligned_kv(
        &mut lines,
        "recipe",
        json_string(package, &["local_recipe_path"]),
        width,
    );
    lines.push(format_aligned_kv(
        "published",
        &package
            .get("published")
            .is_some_and(|value| !value.is_null())
            .to_string(),
        width,
    ));
    push_aligned_kv(
        &mut lines,
        "repo",
        json_string(package, &["packages_repo_path"]),
        width,
    );
    push_aligned_kv(
        &mut lines,
        "index",
        json_string(package, &["index_path"]),
        width,
    );
    if let Some(published) = package.get("published").filter(|value| !value.is_null()) {
        if let Some(version) = package_version(published, "pkgver", "pkgrel") {
            lines.push(format_aligned_kv("version", &version, width));
        }
        push_aligned_kv(&mut lines, "arch", json_string(published, &["arch"]), width);
        push_aligned_kv(
            &mut lines,
            "payload",
            json_string(published, &["payload_path"]),
            width,
        );
        push_aligned_kv(
            &mut lines,
            "commit",
            json_string(published, &["repo_commit"]),
            width,
        );
    }
    human_operator_frame(format!("forge {title}"), lines, None)
}
