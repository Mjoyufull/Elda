use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{format_detail_row, human_operator_frame, json_string};
use crate::app_render_tree::{FrameFooter, Glyph, TreeStyle, frame_from_sections};

pub(crate) fn render_recipe_catalog_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let catalog = details.get("catalog")?;
    let recipes_dir = catalog.get("recipes_dir")?.as_str()?;
    let local = catalog.get("local_recipes")?.as_array()?;
    let synced = catalog.get("synced_packages")?.as_array()?;
    let local_entries = catalog.get("local_entries").and_then(Value::as_array);
    let synced_entries = catalog.get("synced_entries").and_then(Value::as_array);

    let mut lines = vec![
        format_detail_row("Directory", recipes_dir, 16),
        String::new(),
    ];

    let local_blocks = render_catalog_entry_lines(local, local_entries);
    if local_blocks.is_empty() {
        lines.push("(no local recipes)".to_owned());
    } else {
        lines.extend(local_blocks);
    }

    lines.push(String::new());
    lines.push("Synced packages (`elda i <name>`)".to_owned());
    let synced_blocks = render_catalog_entry_lines(synced, synced_entries);
    if synced_blocks.is_empty() {
        lines.push("(none - run `elda sync` after `rmt add`)".to_owned());
    } else {
        lines.extend(synced_blocks);
    }

    Some(human_operator_frame("recipe catalog", lines, None))
}

pub(crate) fn render_recipe_removed_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let removed = details.get("removed")?;
    let pkgname = removed.get("pkgname")?.as_str()?;
    let path = removed.get("path")?.as_str()?;

    let frame = frame_from_sections(
        format!("Recipe Removed: {pkgname}"),
        &[(
            "Removed".to_owned(),
            vec![format!("pkgname: {pkgname}"), format!("path: {path}")],
        )],
        Some(FrameFooter {
            glyph: Some(Glyph::Done),
            text: format!("Removed {pkgname}"),
        }),
    );
    Some(frame.render(TreeStyle::detect()))
}

fn render_catalog_entry_lines(names: &[Value], entries: Option<&Vec<Value>>) -> Vec<String> {
    entries.map_or_else(
        || {
            names
                .iter()
                .filter_map(Value::as_str)
                .flat_map(|name| vec![format_detail_row("Name", name, 16), String::new()])
                .collect()
        },
        |entries| {
            entries
                .iter()
                .flat_map(render_catalog_entry_lines_for_entry)
                .collect()
        },
    )
}

fn render_catalog_entry_lines_for_entry(entry: &Value) -> Vec<String> {
    let name = json_string(entry, &["pkgname"]).unwrap_or("unknown");
    let version = json_string(entry, &["version"]).unwrap_or("unknown-version");
    let source = json_string(entry, &["source"]).unwrap_or("unknown");
    let provenance = catalog_provenance(source);

    let mut lines = vec![
        format_detail_row("Name", name, 16),
        format_detail_row("Version", version, 16),
        format_detail_row("Provenance", &provenance, 16),
    ];

    if let Some(description) = json_string(entry, &["description"])
        && !description.is_empty()
    {
        lines.push(format_detail_row("Description", description, 16));
    }
    if let Some(upstream) = json_string(entry, &["upstream"])
        && !upstream.is_empty()
    {
        lines.push(format_detail_row("Upstream", upstream, 16));
    }
    if let Some(licenses) = entry.get("licenses").and_then(Value::as_array) {
        let formatted = licenses
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !formatted.is_empty() {
            lines.push(format_detail_row("Licenses", &formatted, 16));
        }
    }
    lines.push(String::new());
    lines
}

fn catalog_provenance(source: &str) -> String {
    if source == "local_recipe" {
        return "[E] local_recipe (Native)".to_owned();
    }
    if let Some(remote) = source.strip_prefix("synced:") {
        return format!("[E] synced/{remote} (Native, remote)");
    }
    format!("[?] {source}")
}
