use serde_json::Value;

mod search;

pub(crate) use search::render_search_report;

use crate::CommandReport;
use crate::app_render_support::{json_string, render_header, render_section};
use crate::app_render_tree::{FrameFooter, Glyph, TreeStyle, frame_from_sections};
use crate::run_log::session_log_path;

pub(crate) fn render_session_log_section(details: &Value) -> Option<String> {
    let path = session_log_path(details)?;
    Some(render_section("Log", &[format!("path: {path}")]))
}

pub(crate) fn render_failure_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let blocked = json_string(details, &["blocked"]).unwrap_or(&report.summary);
    let kind = json_string(details, &["kind"]).unwrap_or("failure");
    let next_action =
        json_string(details, &["next_action"]).unwrap_or("retry after fixing the reported issue");
    let command = failure_command(details);

    let mut sections: Vec<(String, Vec<String>)> = vec![(
        "Blocked".to_owned(),
        vec![
            format!("kind: {kind}"),
            format!("reason: {blocked}"),
            format!("command: {command}"),
        ],
    )];

    if let Some(lines) = failure_context_lines(details) {
        sections.push(("Context".to_owned(), lines));
    }
    sections.push(("Action".to_owned(), vec![next_action.to_owned()]));

    let title = if report.operands.is_empty() {
        format!("{} blocked", report.area)
    } else {
        format!("{} blocked: {}", report.area, report.operands.join(", "))
    };
    let footer = FrameFooter {
        glyph: Some(Glyph::Blocked),
        text: format!("Status: {}", report.status),
    };
    let frame = frame_from_sections(title, &sections, Some(footer));
    let header = render_header(report.area, report.status);
    let body = frame.render(TreeStyle::detect());
    Some(format!("{header}\n{}\n\n{body}", report.summary))
}

fn failure_command(details: &Value) -> String {
    let command_path = details
        .get("command_path")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let operands = details
        .get("operands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if command_path.is_empty() && operands.is_empty() {
        return "elda".to_owned();
    }

    ["elda"]
        .into_iter()
        .chain(command_path)
        .chain(operands)
        .collect::<Vec<_>>()
        .join(" ")
}

fn failure_context_lines(details: &Value) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    let dry_run = details
        .get("dry_run")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let system_mode = details
        .get("system_mode")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let offline = details
        .get("offline")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    lines.push(format!("dry run: {dry_run}"));
    lines.push(format!("system mode: {system_mode}"));
    lines.push(format!("offline: {offline}"));

    for cause in details
        .get("causes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        lines.push(format!("cause: {cause}"));
    }

    if lines.is_empty() { None } else { Some(lines) }
}

pub(crate) fn render_metadata_add_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let add = details.get("metadata_add")?;
    let targets = add.get("targets")?.as_array()?;

    let header = render_header(report.area, report.status);
    let mut blocks = vec![header, report.summary.clone()];

    for target in targets {
        let target_name = json_string(target, &["target"]).unwrap_or("<unknown>");
        let recipe_name = json_string(target, &["recipe_name"]).unwrap_or(target_name);
        let recipe_dir = json_string(target, &["recipe_dir"]).unwrap_or("<unknown>");
        let pkg_lua = json_string(target, &["pkg_lua"]).unwrap_or("<unknown>");
        let selected_lane = json_string(target, &["selected_lane"]).unwrap_or("unknown");
        let selected_source_kind =
            json_string(target, &["selected_source_kind"]).unwrap_or("unknown");
        let publish_ready = target
            .get("publish_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut lines = vec![
            format!("target: {target_name}"),
            format!("recipe: {recipe_name}"),
            format!("path: {recipe_dir}"),
            format!("pkg.lua: {pkg_lua}"),
            format!("strategy: {selected_lane} / {selected_source_kind}"),
            format!("publish-ready: {publish_ready}"),
        ];
        lines.extend(source_option_lines(target, add));
        lines.extend(metadata_field_lines(target));

        let title = format!("Metadata Add: {recipe_name}");
        let footer = FrameFooter {
            glyph: None,
            text: format!("Output: {recipe_dir}"),
        };
        let frame = frame_from_sections(title, &[("Metadata".to_owned(), lines)], Some(footer));
        blocks.push(frame.render(TreeStyle::detect()));
    }

    Some(blocks.join("\n\n"))
}

fn metadata_field_lines(target: &Value) -> Vec<String> {
    let Some(fields) = target.get("fields").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut lines = Vec::with_capacity(fields.len() + 1);
    lines.push("field confidence:".to_owned());
    for field in fields {
        let name = json_string(field, &["field"]).unwrap_or("unknown");
        let confidence = json_string(field, &["confidence"]).unwrap_or("unknown");
        lines.push(format!("  {name}: {confidence}"));
    }
    lines
}

pub(crate) fn source_option_lines(target: &Value, parent: &Value) -> Vec<String> {
    let mode = parent
        .get("link_option_mode")
        .and_then(Value::as_str)
        .unwrap_or("priority");
    if mode != "list-options" {
        return Vec::new();
    }

    let Some(options) = target.get("source_options").and_then(Value::as_array) else {
        return Vec::new();
    };
    if options.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::with_capacity(options.len() + 1);
    lines.push("source options:".to_owned());
    for option in options {
        let index = option.get("index").and_then(Value::as_u64).unwrap_or(0);
        let marker = if option
            .get("selected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "*"
        } else {
            " "
        };
        let strategy = json_string(option, &["strategy"]).unwrap_or("unknown");
        let source_kind = json_string(option, &["source_kind"]).unwrap_or("unknown");
        let confidence = json_string(option, &["confidence"]).unwrap_or("unknown");
        let summary = json_string(option, &["summary"]).unwrap_or("detected source option");
        let mut detail =
            format!("{marker} {index}. {strategy} [{source_kind}, {confidence}] - {summary}");
        if let Some(tag) = json_string(option, &["tag"]) {
            detail.push_str(&format!(" tag={tag}"));
        }
        if let Some(asset) = json_string(option, &["asset"]) {
            detail.push_str(&format!(" asset={asset}"));
        }
        if let Some(compatibility) = json_string(option, &["compatibility"]) {
            detail.push_str(&format!(" compatibility={compatibility}"));
        }
        let checksum_available = option
            .get("checksum_available")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if checksum_available {
            detail.push_str(" checksum=sha256");
        }
        lines.push(detail);
    }
    lines
}

pub(crate) fn render_recipe_catalog_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let catalog = details.get("catalog")?;
    let recipes_dir = catalog.get("recipes_dir")?.as_str()?;
    let local = catalog.get("local_recipes")?.as_array()?;
    let synced = catalog.get("synced_packages")?.as_array()?;
    let local_entries = catalog.get("local_entries").and_then(Value::as_array);
    let synced_entries = catalog.get("synced_entries").and_then(Value::as_array);

    let header = render_header(report.area, report.status);
    let mut blocks = vec![header, report.summary.clone()];

    let frame = frame_from_sections(
        "Recipe Catalog",
        &[(
            "Local recipes".to_owned(),
            vec![format!("directory: {recipes_dir}")],
        )],
        None,
    );
    blocks.push(frame.render(TreeStyle::detect()));

    let local_blocks = render_catalog_entry_blocks(local, local_entries);
    if local_blocks.is_empty() {
        blocks.push(render_section("Local recipe names", &["(none)".to_owned()]));
    } else {
        blocks.extend(local_blocks);
    }

    let synced_blocks = render_catalog_entry_blocks(synced, synced_entries);
    if synced_blocks.is_empty() {
        blocks.push(render_section(
            "Synced packages (`elda i <name>`)",
            &["(none - run `elda sync` after `rmt add`)".to_owned()],
        ));
    } else {
        blocks.push(render_section(
            "Synced packages (`elda i <name>`)",
            &[format!("count: {}", synced_blocks.len())],
        ));
        blocks.extend(synced_blocks);
    }

    Some(blocks.join("\n\n"))
}

pub(crate) fn render_recipe_removed_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let removed = details.get("removed")?;
    let pkgname = removed.get("pkgname")?.as_str()?;
    let path = removed.get("path")?.as_str()?;

    let header = render_header(report.area, report.status);
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
    Some(format!(
        "{header}\n{}\n\n{}",
        report.summary,
        frame.render(TreeStyle::detect())
    ))
}

fn render_catalog_entry_blocks(names: &[Value], entries: Option<&Vec<Value>>) -> Vec<String> {
    entries.map_or_else(
        || {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(render_catalog_name_block)
                .collect()
        },
        |entries| entries.iter().map(render_catalog_entry_block).collect(),
    )
}

fn render_catalog_name_block(name: &str) -> String {
    format!("Name:           {name}")
}

fn render_catalog_entry_block(entry: &Value) -> String {
    let name = json_string(entry, &["pkgname"]).unwrap_or("unknown");
    let version = json_string(entry, &["version"]).unwrap_or("unknown-version");
    let source = json_string(entry, &["source"]).unwrap_or("unknown");
    let provenance = catalog_provenance(source);

    let mut lines = vec![
        format!("Name:           {name}"),
        format!("Version:        {version}"),
        format!("Provenance:     {provenance}"),
    ];

    if let Some(description) = json_string(entry, &["description"])
        && !description.is_empty()
    {
        lines.push(format!("Description:    {description}"));
    }
    if let Some(upstream) = json_string(entry, &["upstream"])
        && !upstream.is_empty()
    {
        lines.push(format!("Upstream:       {upstream}"));
    }
    if let Some(licenses) = entry.get("licenses").and_then(Value::as_array) {
        let formatted = licenses
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !formatted.is_empty() {
            lines.push(format!("Licenses:       {formatted}"));
        }
    }

    lines.join("\n")
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
