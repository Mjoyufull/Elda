use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{json_array, json_string, json_u64, render_header, render_section};

pub(crate) fn render_appimage_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let inspect = details.get("appimage_inspect")?;

    let mut overview = vec![
        format!(
            "path: {}",
            json_string(inspect, &["path"]).unwrap_or("<unknown>")
        ),
        format!(
            "generation: {}",
            json_u64(inspect, &["generation"])
                .map_or_else(|| "<unknown>".to_owned(), |g| g.to_string(),),
        ),
        format!(
            "squashfs_offset_bytes: {}",
            json_u64(inspect, &["squashfs_offset"]).unwrap_or(0),
        ),
    ];

    if let Some(primary) = json_string(inspect, &["primary_desktop_path"]) {
        overview.push(format!("primary_desktop: {primary}"));
    }
    if let Some(name) = json_string(inspect, &["desktop_name"]) {
        overview.push(format!("desktop_name: {name}"));
    }
    if let Some(exec) = json_string(inspect, &["desktop_exec_original"]) {
        overview.push(format!("desktop_exec_upstream: {exec}"));
    }
    if let Some(icon) = json_string(inspect, &["desktop_icon_raw"]) {
        overview.push(format!("desktop_icon: {icon}"));
    }
    if let Some(apprun) = json_string(inspect, &["apprun_path"]) {
        overview.push(format!("apprun: {apprun}"));
    }

    let desktop_lines = list_preview(inspect, &["desktop_candidates"], 24);
    let icon_lines = list_preview(inspect, &["icon_candidates"], 16);
    let metainfo_lines = list_preview(inspect, &["metainfo_candidates"], 16);

    let fuse = json_string(inspect, &["fuse_note"]).unwrap_or_default();
    let mut fuse_section = Vec::new();
    if !fuse.is_empty() {
        fuse_section.push(fuse.to_owned());
    }

    let body = format!(
        "{}\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        render_section("Overview", &overview),
        render_section("Desktop files inside SquashFS", &desktop_lines),
        render_section("Icon paths inside SquashFS", &icon_lines),
        render_section("AppStream metainfo inside SquashFS", &metainfo_lines),
    );

    if fuse_section.is_empty() {
        Some(body)
    } else {
        Some(format!(
            "{}\n\n{}",
            body,
            render_section("Runtime note", &fuse_section),
        ))
    }
}

fn list_preview(root: &Value, path: &[&str], max: usize) -> Vec<String> {
    let Some(entries) = json_array(root, path) else {
        return vec!["(none)".to_owned()];
    };
    if entries.is_empty() {
        return vec!["(none)".to_owned()];
    }
    let mut lines = Vec::new();
    let total = entries.len();
    let take = total.min(max);
    for entry in entries.iter().take(take) {
        let line = entry
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| entry.to_string());
        lines.push(line);
    }
    if total > take {
        lines.push(format!("… and {} more", total.saturating_sub(take)));
    }
    lines
}
