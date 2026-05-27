use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::push_kv;

pub(super) fn render_forge(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if let Some(results) = details.get("results").and_then(Value::as_array) {
        return Some(human_framed_report(
            report,
            "Forge search",
            &[("Results".to_owned(), forge_search_lines(details, results))],
            None,
        ));
    }

    if let Some(package) = details.get("package") {
        let title = json_string(package, &["package"]).unwrap_or("?");
        return Some(human_framed_report(
            report,
            format!("Forge {title}"),
            &[("Package".to_owned(), forge_package_lines(package))],
            None,
        ));
    }

    if details.get("repo").is_some() {
        return Some(human_framed_report(
            report,
            "Forge fork",
            &[("Repository".to_owned(), forge_repo_lines(details))],
            None,
        ));
    }

    None
}

fn forge_search_lines(details: &Value, results: &[Value]) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "query", json_string(details, &["query"]));
    lines.push(format!("matches: {}", results.len()));
    for result in results.iter().take(16) {
        let package = json_string(result, &["pkgname"]).unwrap_or("?");
        let source = json_string(result, &["source"]).unwrap_or("unknown");
        let path = json_string(result, &["packages_repo_path"])
            .or_else(|| json_string(result, &["payload_path"]))
            .or_else(|| json_string(result, &["index_path"]))
            .unwrap_or("");
        if path.is_empty() {
            lines.push(format!("entry: {package} [{source}]"));
        } else {
            lines.push(format!("entry: {package} [{source}] {path}"));
        }
    }
    if results.len() > 16 {
        lines.push(format!("more: {}", results.len() - 16));
    }
    lines
}

fn forge_package_lines(package: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "package", json_string(package, &["package"]));
    push_kv(&mut lines, "channel", json_string(package, &["channel"]));
    push_kv(
        &mut lines,
        "recipe",
        json_string(package, &["local_recipe_path"]),
    );
    lines.push(format!(
        "published: {}",
        package
            .get("published")
            .is_some_and(|value| !value.is_null())
    ));
    push_kv(
        &mut lines,
        "repo",
        json_string(package, &["packages_repo_path"]),
    );
    push_kv(&mut lines, "index", json_string(package, &["index_path"]));
    if let Some(published) = package.get("published").filter(|value| !value.is_null()) {
        if let Some(version) = package_version(published) {
            lines.push(format!("version: {version}"));
        }
        push_kv(&mut lines, "arch", json_string(published, &["arch"]));
        push_kv(
            &mut lines,
            "payload",
            json_string(published, &["payload_path"]),
        );
        push_kv(
            &mut lines,
            "commit",
            json_string(published, &["repo_commit"]),
        );
    }
    lines
}

fn forge_repo_lines(details: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "repo", json_string(details, &["repo"]));
    push_kv(&mut lines, "tool", json_string(details, &["tool"]));
    if let Some(forked) = details.get("forked").and_then(Value::as_bool) {
        lines.push(format!("forked: {forked}"));
    }
    push_kv(&mut lines, "url", json_string(details, &["result", "url"]));
    push_kv(
        &mut lines,
        "name",
        json_string(details, &["result", "nameWithOwner"]),
    );
    lines
}

fn package_version(package: &Value) -> Option<String> {
    let epoch = json_u64(package, &["epoch"]).unwrap_or(0);
    let version = json_string(package, &["pkgver"])?;
    let rel = json_u64(package, &["pkgrel"]).unwrap_or(1);
    Some(format!("{epoch}:{version}-{rel}"))
}
