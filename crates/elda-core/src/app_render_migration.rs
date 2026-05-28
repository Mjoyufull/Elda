use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{json_string, json_u64, render_header};
use crate::app_render_tree::{FrameFooter, Glyph, TreeStyle, frame_from_sections};

pub(crate) fn render_migration_report(report: &CommandReport) -> Option<String> {
    if report.status == "blocked" {
        return None;
    }
    let details = report.details.as_ref()?;
    if let Some(adoption) = details.get("adoption") {
        return render_adoption(report, adoption);
    }
    if let Some(migration) = details.get("migration") {
        return render_migration(report, migration);
    }
    None
}

fn render_adoption(report: &CommandReport, adoption: &Value) -> Option<String> {
    let pkgname = json_string(adoption, &["pkgname"])?;
    let source_pm = json_string(adoption, &["source_pm"]).unwrap_or("unknown");
    let version = version_line(adoption);
    let file_count = adoption
        .get("files")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let dependency_count = adoption
        .get("dependencies")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let mut lines = vec![
        format!("package: {pkgname}"),
        format!("source pm: {source_pm}"),
        format!("version: {version}"),
        "source kind: adopted".to_owned(),
        "live files modified: no".to_owned(),
        format!("owned paths imported: {file_count}"),
        format!("dependency edges imported: {dependency_count}"),
    ];
    if let Some(arch) = json_string(adoption, &["arch"]) {
        lines.push(format!("arch: {arch}"));
    }

    let frame = frame_from_sections(
        format!("Package Adoption: {pkgname}"),
        &[("Adopted state".to_owned(), lines)],
        Some(FrameFooter {
            glyph: Some(Glyph::Done),
            text: report.summary.clone(),
        }),
    );
    Some(format!(
        "{}\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        frame.render(TreeStyle::detect())
    ))
}

fn render_migration(report: &CommandReport, migration: &Value) -> Option<String> {
    let source_pm = json_string(migration, &["source_pm"])?;
    let package_count = json_u64(migration, &["package_count"]).unwrap_or(0);
    let packages = migration.get("packages").and_then(Value::as_array);
    let mut lines = vec![
        format!("source pm: {source_pm}"),
        format!("packages: {package_count}"),
        "source kind: adopted".to_owned(),
        "live files modified: no".to_owned(),
    ];
    if let Some(packages) = packages {
        let names = packages
            .iter()
            .take(8)
            .filter_map(|package| json_string(package, &["pkgname"]))
            .collect::<Vec<_>>()
            .join(", ");
        if !names.is_empty() {
            lines.push(format!("sample: {names}"));
        }
        if packages.len() > 8 {
            lines.push(format!("sample truncated: {} more", packages.len() - 8));
        }
    }

    let frame = frame_from_sections(
        format!("Foreign Migration: {source_pm}"),
        &[("Imported state".to_owned(), lines)],
        Some(FrameFooter {
            glyph: Some(Glyph::Done),
            text: report.summary.clone(),
        }),
    );
    Some(format!(
        "{}\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        frame.render(TreeStyle::detect())
    ))
}

fn version_line(package: &Value) -> String {
    let raw = json_string(package, &["version", "raw"]);
    let epoch = json_u64(package, &["version", "epoch"]).unwrap_or(0);
    let pkgver = json_string(package, &["version", "pkgver"]).unwrap_or("0");
    let pkgrel = json_u64(package, &["version", "pkgrel"]).unwrap_or(1);
    match raw {
        Some(raw) => format!("{epoch}:{pkgver}-{pkgrel} (foreign: {raw})"),
        None => format!("{epoch}:{pkgver}-{pkgrel}"),
    }
}
