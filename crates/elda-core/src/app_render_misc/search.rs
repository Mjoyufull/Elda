use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::json_string;
use crate::render_style::{paint, palette};

pub(crate) fn render_search_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let query = details.get("query")?.as_str()?;
    let results = details.get("results")?.as_array()?;

    let mut output = String::new();
    if results.is_empty() {
        output.push_str(&format!("query    {query}\n"));
        output.push_str("matches  0\n");
        output.push_str("action   try a broader query or run `elda sync`\n");
    } else {
        let rows = results
            .iter()
            .enumerate()
            .map(search_result_row)
            .collect::<Vec<_>>();
        let widths = search_table_widths(&rows);
        output.push_str(&format!("query    {query}\n"));
        output.push_str(&format!("matches  {}\n\n", rows.len()));
        output.push_str(&format!(
            "{:>2}  {:<badge$}  {:<name$}  {:<version$}  {}\n",
            "#",
            "src",
            "package",
            "version",
            "summary",
            badge = widths.badge,
            name = widths.name,
            version = widths.version,
        ));
        for (idx, result) in results.iter().enumerate() {
            let row = search_result_row((idx, result));
            let c_badge = paint(
                &format!("{:<badge$}", row.badge, badge = widths.badge),
                palette::PROVENANCE,
                true,
            );
            let c_name = paint(
                &format!("{:<name$}", row.name, name = widths.name),
                palette::IDENTITY,
                true,
            );
            let c_version = paint(
                &format!("{:<version$}", row.version, version = widths.version),
                palette::VERSION,
                false,
            );

            output.push_str(&format!(
                "{:>2}  {c_badge}  {c_name}  {c_version}  {}\n",
                row.index, row.summary,
            ));
        }
    }

    Some(output)
}

#[derive(Debug)]
struct SearchRow {
    index: usize,
    badge: &'static str,
    name: String,
    version: String,
    summary: String,
}

#[derive(Debug)]
struct SearchWidths {
    badge: usize,
    name: usize,
    version: usize,
}

fn search_result_row((idx, result): (usize, &Value)) -> SearchRow {
    let name = json_string(result, &["pkgname"]).unwrap_or("unknown");
    let remote = json_string(result, &["remote_name"]).unwrap_or("local");
    let summary = result
        .get("description")
        .and_then(Value::as_str)
        .or_else(|| result.get("summary").and_then(Value::as_str))
        .unwrap_or("No description available.");

    SearchRow {
        index: idx + 1,
        badge: search_provenance_badge(result),
        name: format!("{remote}/{name}"),
        version: search_result_version(result),
        summary: summary.to_owned(),
    }
}

fn search_table_widths(rows: &[SearchRow]) -> SearchWidths {
    let mut widths = SearchWidths {
        badge: "src".len(),
        name: "package".len(),
        version: "version".len(),
    };
    for row in rows {
        widths.badge = widths.badge.max(row.badge.len());
        widths.name = widths.name.max(row.name.len());
        widths.version = widths.version.max(row.version.len());
    }
    widths
}
fn search_provenance_badge(result: &Value) -> &'static str {
    match json_string(result, &["source_kind"]).unwrap_or("local_recipe") {
        "interbuild" | "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template" => "[I]",
        "repo_binary" => "[B]",
        "github_release" | "release_asset" | "url_archive" | "appimage" => "[F]",
        "interemote" => "[R]",
        "local_recipe" => "[L]",
        "git" | "local_path" => "[S]",
        _ => "[?]",
    }
}

fn search_result_version(result: &Value) -> String {
    let epoch = result.get("epoch").and_then(Value::as_u64).unwrap_or(0);
    let version = json_string(result, &["pkgver"]).unwrap_or("unknown");
    let rel = result.get("pkgrel").and_then(Value::as_u64).unwrap_or(0);
    format!("{epoch}:{version}-{rel}")
}
