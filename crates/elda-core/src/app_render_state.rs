use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{json_array, json_string, json_u64, render_header};

pub(crate) fn render_installed_packages_report(report: &CommandReport) -> Option<String> {
    if !is_installed_list_command(&report.command_path) {
        return None;
    }
    let details = report.details.as_ref()?;
    let packages = json_array(details, &["packages"])?;

    let mut blocks = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];

    if packages.is_empty() {
        blocks.push("No installed packages.".to_owned());
        return Some(blocks.join("\n\n"));
    }

    for package in packages {
        blocks.push(render_installed_package_entry(package));
    }

    Some(blocks.join("\n\n"))
}

pub(crate) fn render_state_show_report(report: &CommandReport) -> Option<String> {
    if report.command_path != ["state", "show"] {
        return None;
    }
    let details = report.details.as_ref()?;

    let mut blocks = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];

    blocks.push(render_state_overview(details));

    if let Some(world) = json_array(details, &["world"]) {
        blocks.push(render_world_block(world));
    }

    if let Some(backend) = details.get("backend") {
        blocks.push(render_backend_block(backend));
    }

    if let Some(packages) = json_array(details, &["packages"]) {
        if packages.is_empty() {
            blocks.push("No installed packages.".to_owned());
        } else {
            for package in packages {
                blocks.push(render_installed_package_entry(package));
            }
        }
    }

    Some(blocks.join("\n\n"))
}

fn render_state_overview(details: &Value) -> String {
    let schema = json_u64(details, &["schema_version"])
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let active_state = json_string(details, &["active_state"]).unwrap_or("none");
    let lines = vec![
        format!("schema version: {schema}"),
        format!("active state:   {active_state}"),
    ];
    render_kv_block("Snapshot", &lines)
}

fn render_world_block(world: &[Value]) -> String {
    if world.is_empty() {
        return render_kv_block("World", &["(no anchored world targets)".to_owned()]);
    }
    let entries = world
        .iter()
        .filter_map(Value::as_str)
        .map(|name| format!("- {name}"))
        .collect::<Vec<_>>();
    render_kv_block("World", &entries)
}

fn render_backend_block(backend: &Value) -> String {
    let kind = json_string(backend, &["kind"]).unwrap_or("unknown");
    let mode = json_string(backend, &["mode"]).unwrap_or("unknown");
    let lines = vec![format!("backend: {kind}"), format!("mode:    {mode}")];
    render_kv_block("Backend", &lines)
}

fn render_installed_package_entry(package: &Value) -> String {
    let name = json_string(package, &["pkgname"]).unwrap_or("<unknown>");
    let version = json_string(package, &["version"]).unwrap_or("<unknown>");
    let arch = json_string(package, &["arch"]).unwrap_or("<unknown>");
    let reason = json_string(package, &["install_reason"]).unwrap_or("<unknown>");
    let source_kind = json_string(package, &["source_kind"]).unwrap_or("<unknown>");
    let provenance = format!(
        "{} {source_kind} ({})",
        provenance_tag(source_kind),
        provenance_tier(source_kind),
    );

    let mut lines = vec![
        format!("Name:           {name}"),
        format!("Version:        {version}"),
        format!("Architecture:   {arch}"),
        format!("Reason:         {reason}"),
        format!("Provenance:     {provenance}"),
    ];

    if let Some(remote) = json_string(package, &["remote_name"]) {
        lines.push(format!("Remote:         {remote}"));
    }
    if let Some(reference) = json_string(package, &["source_ref"]) {
        lines.push(format!("Source ref:     {reference}"));
    }
    if let Some(commit) = json_string(package, &["repo_commit"]) {
        lines.push(format!("Repo commit:    {commit}"));
    }
    if let Some(variant) = json_string(package, &["variant_id"])
        && variant != "default"
    {
        lines.push(format!("Variant:        {variant}"));
    }
    if let Some(state) = json_string(package, &["state_id"]) {
        lines.push(format!("State id:       {state}"));
    }
    if let Some(backend) = json_string(package, &["activation_backend"]) {
        lines.push(format!("Backend:        {backend}"));
    }
    if let Some(manifest) = json_string(package, &["manifest_hash"]) {
        lines.push(format!("Manifest:       {manifest}"));
    }
    if let Some(payload) = json_string(package, &["payload_sha256"]) {
        lines.push(format!("Payload:        {payload}"));
    }
    if let Some(pin) = json_string(package, &["pinned_version"]) {
        lines.push(format!("Pinned:         {pin}"));
    }
    let held = package
        .get("held")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if held {
        let source = json_string(package, &["hold_source"]).unwrap_or("operator");
        lines.push(format!("Hold:           yes ({source})"));
    }
    let kind = json_string(package, &["package_kind"]).unwrap_or("normal");
    if kind != "normal" {
        lines.push(format!("Kind:           {kind}"));
    }

    lines.join("\n")
}

fn render_kv_block(title: &str, lines: &[String]) -> String {
    let mut rendered = title.to_owned();
    for line in lines {
        rendered.push('\n');
        rendered.push_str("  ");
        rendered.push_str(line);
    }
    rendered
}

fn provenance_tag(source_kind: &str) -> &'static str {
    match source_kind {
        "local_recipe" | "repo_binary" => "[E]",
        "interbuild" => "[I]",
        "interepo" => "[F]",
        "adopted" => "[A]",
        "git" | "vendor" | "url_archive" => "[V]",
        _ => "[?]",
    }
}

fn provenance_tier(source_kind: &str) -> &'static str {
    match source_kind {
        "local_recipe" => "Native",
        "repo_binary" => "Native, remote",
        "interbuild" => "Interbuild",
        "interepo" => "Interepo",
        "adopted" => "Adopted",
        "git" => "Vendor / Ad-Hoc",
        "vendor" => "Vendor",
        "url_archive" => "Vendor / Ad-Hoc",
        _ => "Unknown",
    }
}

fn is_installed_list_command(command_path: &[String]) -> bool {
    matches!(command_path.iter().map(String::as_str).next(), Some("ls")) && command_path.len() == 1
}
