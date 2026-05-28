use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

pub(super) fn render_review(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();

    if let Some(stamps) = details.get("stamps").and_then(Value::as_array) {
        lines.push(format!("stamps: {}", stamps.len()));
        for stamp in stamps.iter().take(32) {
            let package = json_string(stamp, &["package"]).unwrap_or("?");
            let kind = json_string(stamp, &["review_kind"]).unwrap_or("?");
            let path = json_string(stamp, &["recipe_path"]).unwrap_or("?");
            lines.push(format!("  {package} ({kind}): {path}"));
        }
        if stamps.len() > 32 {
            lines.push(format!("  … {} more", stamps.len() - 32));
        }
    }

    if let Some(stamps) = details.get("stamps").and_then(Value::as_array)
        && stamps.is_empty()
        && details.get("package").is_some()
    {
        let package = json_string(details, &["package"]).unwrap_or("?");
        lines.push(format!("package: {package}"));
        lines.push("no review stamps recorded".to_owned());
    }

    if let Some(history) = details.get("history").and_then(Value::as_array)
        && !history.is_empty()
    {
        lines.push(format!("review history: {}", history.len()));
        for entry in history.iter().take(8) {
            let package = json_string(entry, &["package"]).unwrap_or("?");
            let kind = json_string(entry, &["review_kind"]).unwrap_or("?");
            let accepted = json_string(entry, &["accepted_at"]).unwrap_or("?");
            let digest =
                json_string(entry, &["recipe_hash"]).map(|hash| &hash[..16.min(hash.len())]);
            lines.push(format!(
                "  {package} ({kind}) @ {accepted}{}",
                digest
                    .map(|value| format!(" digest {value}"))
                    .unwrap_or_default()
            ));
        }
    }

    if let Some(package) = json_string(details, &["package"])
        && let Some(stamps) = details.get("stamps").and_then(Value::as_array)
        && !stamps.is_empty()
    {
        lines.push(format!("package: {package}"));
        for stamp in stamps {
            let kind = json_string(stamp, &["review_kind"]).unwrap_or("?");
            let path = json_string(stamp, &["recipe_path"]).unwrap_or("?");
            let digest = json_string(stamp, &["recipe_hash"]).unwrap_or("?");
            let accepted = json_string(stamp, &["accepted_at"]).unwrap_or("unknown");
            lines.push(format!("  {kind}: {path}"));
            lines.push(format!("    digest: {digest}"));
            lines.push(format!("    accepted_at: {accepted}"));
            if let Some(source) = json_string(stamp, &["source_url"]) {
                lines.push(format!("    source: {source}"));
            }
            if let Some(remote) = json_string(stamp, &["remote_name"]) {
                lines.push(format!("    remote: {remote}"));
            }
            if let Some(parser) = json_string(stamp, &["parser"]) {
                lines.push(format!("    parser: {parser}"));
            }
        }
    }

    if let Some(removed) = details.get("removed").and_then(Value::as_bool) {
        lines.push(format!("removed: {removed}"));
    }

    if let Some(unchanged) = details.get("unchanged").and_then(Value::as_bool) {
        lines.push(format!("unchanged: {unchanged}"));
    }
    if let Some(path) = json_string(details, &["recipe_path"]) {
        lines.push(format!("recipe: {path}"));
    }
    if let Some(pager) = json_string(details, &["opened_pager"]) {
        lines.push(format!("pager: {pager}"));
    }

    let title = match report.status {
        "changed" => "Review Diff: changed",
        "current" => "Review Diff: current",
        "new" => "Review Diff: new",
        "missing" => "Review Info: missing",
        _ => "Review",
    };

    Some(human_framed_report(
        report,
        title,
        &[("Summary".to_owned(), lines)],
        None,
    ))
}
