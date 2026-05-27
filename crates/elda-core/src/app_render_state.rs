use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{json_array, json_string, json_u64, render_header};
use crate::render_style::{paint, palette};

pub(crate) fn render_installed_packages_report(report: &CommandReport) -> Option<String> {
    match report.command_path.as_slice() {
        [command] if command == "ls" => render_installed_ls_report(report),
        [command] if command == "list" => render_installed_list_report(report),
        _ => None,
    }
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

fn render_installed_ls_report(report: &CommandReport) -> Option<String> {
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

    let rows = packages.iter().map(installed_ls_row).collect::<Vec<_>>();
    let widths = installed_ls_widths(&rows);
    let mut table = String::new();
    table.push_str(&format!(
        "{:<name$}  {:<version$}  {:<reason$}  {:<origin$}  STATUS\n",
        "NAME",
        "VERSION",
        "REASON",
        "ORIGIN",
        name = widths.name,
        version = widths.version,
        reason = widths.reason,
        origin = widths.origin,
    ));
    for row in &rows {
        let name = paint(
            &format!("{:<name$}", row.name, name = widths.name),
            palette::IDENTITY,
            true,
        );
        let version = paint(
            &format!("{:<version$}", row.version, version = widths.version),
            palette::VERSION,
            false,
        );
        let badge = paint(
            &format!("{:<reason$}", row.reason, reason = widths.reason),
            palette::MUTED,
            false,
        );
        let origin = paint(
            &format!("{:<origin$}", row.origin, origin = widths.origin),
            palette::PROVENANCE,
            false,
        );
        let status = paint_status_marker(&row.status);
        table.push_str(&format!("{name}  {version}  {badge}  {origin}  {status}\n",));
    }

    blocks.push(table.trim_end().to_owned());
    Some(blocks.join("\n\n"))
}

fn render_installed_list_report(report: &CommandReport) -> Option<String> {
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
        blocks.push(render_installed_list_entry(package));
    }

    Some(blocks.join("\n\n"))
}

#[derive(Debug)]
struct InstalledLsRow {
    name: String,
    version: String,
    reason: String,
    origin: String,
    status: String,
}

#[derive(Debug)]
struct InstalledLsWidths {
    name: usize,
    version: usize,
    reason: usize,
    origin: usize,
}

fn installed_ls_row(package: &Value) -> InstalledLsRow {
    InstalledLsRow {
        name: json_string(package, &["pkgname"])
            .unwrap_or("<unknown>")
            .to_owned(),
        version: json_string(package, &["version"])
            .unwrap_or("<unknown>")
            .to_owned(),
        reason: json_string(package, &["install_reason"])
            .unwrap_or("<unknown>")
            .to_owned(),
        origin: package_origin(package),
        status: package_status_marker(package),
    }
}

fn installed_ls_widths(rows: &[InstalledLsRow]) -> InstalledLsWidths {
    let mut widths = InstalledLsWidths {
        name: "NAME".len(),
        version: "VERSION".len(),
        reason: "REASON".len(),
        origin: "ORIGIN".len(),
    };
    for row in rows {
        widths.name = widths.name.max(row.name.len());
        widths.version = widths.version.max(row.version.len());
        widths.reason = widths.reason.max(row.reason.len());
        widths.origin = widths.origin.max(row.origin.len());
    }
    widths
}

fn package_origin(package: &Value) -> String {
    if let Some(remote) = json_string(package, &["remote_name"]) {
        return remote.to_owned();
    }
    match json_string(package, &["source_kind"]).unwrap_or("unknown") {
        "local_recipe" | "repo_binary" => "native".to_owned(),
        other => other.to_owned(),
    }
}

fn package_status_marker(package: &Value) -> String {
    let pinned = package
        .get("pinned_version")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty());
    let held = package
        .get("held")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut markers = Vec::new();
    if pinned {
        markers.push("[pinned]");
    }
    if held {
        markers.push("[held]");
    }
    if markers.is_empty() {
        "-".to_owned()
    } else {
        markers.join(", ")
    }
}

fn paint_status_marker(status: &str) -> String {
    if status == "-" {
        return paint(status, palette::MUTED, false);
    }
    paint(status, palette::WARNING, false)
}

fn render_installed_list_entry(package: &Value) -> String {
    let name = json_string(package, &["pkgname"]).unwrap_or("<unknown>");
    let version = json_string(package, &["version"]).unwrap_or("<unknown>");
    let reason = json_string(package, &["install_reason"]).unwrap_or("<unknown>");
    let source_kind = json_string(package, &["source_kind"]).unwrap_or("<unknown>");
    let tag = provenance_tag(source_kind);

    let mut lines = vec![format!("{tag} {name}")];
    lines.push(format!("  Version: {version}"));
    lines.push(format!("  Reason: {reason}"));
    lines.push(format!("  Origin: {}", package_origin(package)));

    if let Some(state) = json_string(package, &["state_id"]) {
        lines.push(format!("  State ID: {state}"));
    }
    if let Some(manifest) = json_string(package, &["manifest_hash"]) {
        lines.push(format!("  Manifest: {manifest}"));
    }
    if let Some(payload) = json_string(package, &["payload_sha256"]) {
        lines.push(format!("  Payload: {payload}"));
    }
    if let Some(remote) = json_string(package, &["remote_name"]) {
        lines.push(format!("  Remote: {remote}"));
    }
    if let Some(reference) = json_string(package, &["source_ref"]) {
        lines.push(format!("  Source ref: {reference}"));
    }
    if let Some(commit) = json_string(package, &["repo_commit"]) {
        lines.push(format!("  Repo commit: {commit}"));
    }
    if let Some(variant) = json_string(package, &["variant_id"])
        && variant != "default"
    {
        lines.push(format!("  Variant: {variant}"));
    }
    if let Some(pin) = json_string(package, &["pinned_version"]) {
        lines.push(format!("  Pinned: {pin}"));
    }
    let held = package
        .get("held")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if held {
        let source = json_string(package, &["hold_source"]).unwrap_or("operator");
        lines.push(format!("  Hold: yes ({source})"));
    }
    let kind = json_string(package, &["package_kind"]).unwrap_or("normal");
    if kind != "normal" {
        lines.push(format!("  Kind: {kind}"));
    }

    lines.join("\n")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandReport, ExitStatus, OutputMode};
    use serde_json::json;

    fn sample_packages() -> Value {
        json!({
            "packages": [
                {
                    "pkgname": "bfetch",
                    "version": "0:0.1.0-1",
                    "install_reason": "explicit",
                    "source_kind": "local_recipe",
                    "held": false
                },
                {
                    "pkgname": "fsel",
                    "version": "0:3.3.1-1",
                    "install_reason": "dependency",
                    "source_kind": "interbuild",
                    "remote_name": "yoka-core",
                    "pinned_version": "3.3.1",
                    "held": true
                }
            ]
        })
    }

    #[test]
    fn ls_renders_scan_table_without_detail_fields() {
        let report = CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: vec!["ls".to_owned()],
            operands: Vec::new(),
            output_mode: OutputMode::Human,
            dry_run: false,
            summary: "listed 2 installed package(s).".to_owned(),
            details: Some(sample_packages()),
        };

        let rendered = render_installed_ls_report(&report).expect("ls render");

        assert!(rendered.contains("NAME"));
        assert!(rendered.contains("bfetch"));
        assert!(rendered.contains("fsel"));
        assert!(rendered.contains("[pinned]"));
        assert!(!rendered.contains("Manifest:"));
        assert!(!rendered.contains("Name:"));
    }

    #[test]
    fn list_renders_verbose_blocks_with_provenance_prefix() {
        let report = CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: vec!["list".to_owned()],
            operands: Vec::new(),
            output_mode: OutputMode::Human,
            dry_run: false,
            summary: "listed 2 installed package(s).".to_owned(),
            details: Some(sample_packages()),
        };

        let rendered = render_installed_list_report(&report).expect("list render");

        assert!(rendered.contains("[E] bfetch"));
        assert!(rendered.contains("  Version: 0:0.1.0-1"));
        assert!(rendered.contains("[I] fsel"));
        assert!(rendered.contains("  Pinned: 3.3.1"));
        assert!(!rendered.contains("NAME"));
    }
}
