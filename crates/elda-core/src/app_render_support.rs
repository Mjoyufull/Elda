use serde_json::Value;

use crate::CommandReport;
use crate::run_log::session_log_path;

#[must_use]
pub(crate) fn render_json_block(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_owned())
}

#[must_use]
pub(crate) fn render_header(area: &str, status: &str) -> String {
    format!("{area}: {status}")
}

pub(crate) fn render_install_plan_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if details
        .get("plan")?
        .get("kind")?
        .as_str()
        .filter(|kind| *kind == "install")
        .is_none()
    {
        return None;
    }

    let actions = details.get("plan")?.get("actions")?.as_array()?;
    Some(render_install_report_sections(
        report, details, actions, false,
    ))
}

pub(crate) fn render_install_success_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let installs = details.get("installs")?.as_array()?;
    Some(render_install_report_sections(
        report, details, installs, true,
    ))
}

pub(crate) fn render_session_log_section(details: &Value) -> Option<String> {
    let path = session_log_path(details)?;
    Some(render_section("Log", &[format!("path: {path}")]))
}

pub(crate) fn render_recipe_catalog_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let catalog = details.get("catalog")?;
    let recipes_dir = catalog.get("recipes_dir")?.as_str()?;
    let local = catalog.get("local_recipes")?.as_array()?;
    let synced = catalog.get("synced_packages")?.as_array()?;
    let local_entries = catalog.get("local_entries").and_then(|value| value.as_array());
    let synced_entries = catalog.get("synced_entries").and_then(|value| value.as_array());

    let mut blocks = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
        render_section("Local recipes", &[format!("directory: {recipes_dir}")]),
    ];

    let local_lines: Vec<String> = local_entries
        .map(|entries| render_catalog_entry_lines(entries.as_slice()))
        .unwrap_or_else(|| {
            local
                .iter()
                .filter_map(|value| value.as_str().map(|name| format!("- {name}")))
                .collect()
        });
    if local_lines.is_empty() {
        blocks.push(render_section("Local recipe names", &["(none)".to_owned()]));
    } else {
        blocks.push(render_section("Local recipe names", &local_lines));
    }

    let synced_lines: Vec<String> = synced_entries
        .map(|entries| render_catalog_entry_lines(entries.as_slice()))
        .unwrap_or_else(|| {
            synced
                .iter()
                .filter_map(|value| value.as_str().map(|name| format!("- {name}")))
                .collect()
        });
    if synced_lines.is_empty() {
        blocks.push(render_section(
            "Synced packages",
            &["(none — run `elda sync` after `rmt add`)".to_owned()],
        ));
    } else {
        blocks.push(render_section(
            "Synced packages (`elda i <name>`)",
            &synced_lines,
        ));
    }

    Some(blocks.join("\n\n"))
}

fn render_catalog_entry_lines(entries: &[Value]) -> Vec<String> {
    let mut lines = Vec::new();
    for entry in entries {
        let name = entry
            .get("pkgname")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let version = entry.get("version").and_then(|value| value.as_str());
        let source = entry
            .get("source")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        lines.push(format!(
            "- {source}/{name} {}",
            version.unwrap_or("unknown-version")
        ));
        if let Some(description) = entry.get("description").and_then(|value| value.as_str())
            && !description.is_empty()
        {
            lines.push(format!("  {description}"));
        }
        if let Some(upstream) = entry.get("upstream").and_then(|value| value.as_str())
            && !upstream.is_empty()
        {
            lines.push(format!("  upstream: {upstream}"));
        }
    }
    lines
}

pub(crate) fn render_recipe_removed_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let removed = details.get("removed")?;
    let pkgname = removed.get("pkgname")?.as_str()?;
    let path = removed.get("path")?.as_str()?;

    Some(format!(
        "{}\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        render_section(
            "Removed",
            &[format!("pkgname: {pkgname}"), format!("path: {path}")],
        ),
    ))
}

pub(crate) fn render_search_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let query = details.get("query")?.as_str()?;
    let results = details.get("results")?.as_array()?;

    let mut lines = vec![render_header(report.area, report.status), report.summary.clone()];
    if results.is_empty() {
        lines.push(String::new());
        lines.push(format!("No matches for `{query}`."));
        return Some(lines.join("\n"));
    }

    lines.push(String::new());
    for (idx, result) in results.iter().enumerate() {
        let name = result.get("pkgname").and_then(|v| v.as_str()).unwrap_or("unknown");
        let remote = result
            .get("remote_name")
            .and_then(|v| v.as_str())
            .unwrap_or("local");
        let version = if let Some(epoch) = result.get("epoch").and_then(|v| v.as_u64()) {
            let pkgver = result
                .get("pkgver")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0");
            let pkgrel = result.get("pkgrel").and_then(|v| v.as_u64()).unwrap_or(1);
            format!("{epoch}:{pkgver}-{pkgrel}")
        } else {
            "unknown".to_owned()
        };
        lines.push(format!("{} {}/{} {}", idx + 1, remote, name, version));
        let desc = result
            .get("description")
            .and_then(|v| v.as_str())
            .or_else(|| result.get("summary").and_then(|v| v.as_str()))
            .unwrap_or("No description available.");
        lines.push(format!("    {desc}"));
    }

    Some(lines.join("\n"))
}

fn render_install_report_sections(
    report: &CommandReport,
    details: &Value,
    actions: &[Value],
    include_result: bool,
) -> String {
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    sections.push(render_section("Target", &target_lines(report, details)));
    sections.push(render_section("Resolution", &resolution_lines(actions)));
    sections.push(render_section("Plan", &plan_lines(actions)));
    sections.push(render_section("Progress", &progress_lines(actions)));
    if include_result {
        sections.push(render_section("Result", &result_lines(actions)));
    }
    sections.join("\n\n")
}

fn target_lines(report: &CommandReport, details: &Value) -> Vec<String> {
    let requested = if report.operands.is_empty() {
        "(none)".to_owned()
    } else {
        report.operands.join(", ")
    };
    vec![
        format!("requested: {requested}"),
        format!(
            "mode: {}",
            json_string(details, &["layout", "mode"]).unwrap_or("unknown")
        ),
        format!(
            "root: {}",
            json_string(details, &["layout", "prefix"]).unwrap_or("unknown")
        ),
    ]
    .into_iter()
    .chain(
        primary_action(json_array(details, &["plan", "actions"]).unwrap_or_default())
            .or_else(|| primary_action(json_array(details, &["installs"]).unwrap_or_default()))
            .and_then(|action| json_string(action, &["activation_backend"]))
            .map(|backend| format!("backend: {backend}")),
    )
    .collect()
}

fn resolution_lines(actions: &[Value]) -> Vec<String> {
    let Some(action) = primary_action(actions) else {
        return vec!["selected package: none".to_owned()];
    };

    let mut lines = vec![
        format!("selected package: {}", action_package_name(action)),
        format!("version: {}", action_version(action)),
        format!(
            "lane: {}",
            json_string(action, &["selected_lane"]).unwrap_or("unknown")
        ),
        format!(
            "source kind: {}",
            json_string(action, &["selected_source_kind"]).unwrap_or("unknown")
        ),
    ];

    if let Some(remote_name) = json_string(action, &["remote_name"]) {
        lines.push(format!("remote: {remote_name}"));
    }
    if let Some(source_ref) = json_string(action, &["source_ref"]) {
        lines.push(format!("source ref: {source_ref}"));
    }
    if let Some(generated_metadata_path) = json_string(action, &["generated_metadata_path"]) {
        lines.push(format!("generated metadata: {generated_metadata_path}"));
    }
    if let Some(repo_commit) = json_string(action, &["package", "repo_commit"]) {
        lines.push(format!("repo commit: {repo_commit}"));
    }
    if let Some(trust) = binary_trust_summary(action) {
        lines.push(format!("trust: {trust}"));
    }

    lines
}

fn plan_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no actions".to_owned()];
    }

    actions
        .iter()
        .map(|action| {
            format!(
                "{} {} {} [{} / {}]",
                json_string(action, &["action"]).unwrap_or("plan"),
                action_package_name(action),
                action_version(action),
                json_string(action, &["selected_lane"]).unwrap_or("unknown"),
                json_string(action, &["selected_source_kind"]).unwrap_or("unknown"),
            )
        })
        .collect()
}

fn result_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no actions completed".to_owned()];
    }

    actions
        .iter()
        .map(|action| {
            let state_id = json_string(action, &["install", "state_id"]).unwrap_or("unknown");
            let installed_paths = json_u64(action, &["install", "installed_paths"]).unwrap_or(0);
            let activation_backend = json_string(action, &["activation_backend"]);
            let snapshot_summary = render_snapshot_summary(action);
            format!(
                "{} {} -> {}, state {}, {} path(s){}{}",
                action_package_name(action),
                action_version(action),
                json_string(action, &["status"]).unwrap_or("unknown"),
                state_id,
                installed_paths,
                activation_backend
                    .map(|backend| format!(", backend {backend}"))
                    .unwrap_or_default(),
                snapshot_summary
                    .map(|summary| format!(", snapshots {summary}"))
                    .unwrap_or_default(),
            )
        })
        .collect()
}

fn progress_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no steps".to_owned()];
    }

    let mut lines = Vec::new();
    for action in actions {
        let package_name = action_package_name(action);
        let Some(steps) = action.get("progress").and_then(|value| value.as_array()) else {
            lines.push(format!("{package_name}: progress unavailable"));
            continue;
        };

        lines.push(format!("{package_name}:"));
        for step in steps {
            let step_name = step
                .get("step")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown-step");
            let status = step
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let detail = step.get("detail").and_then(|value| value.as_str());
            lines.push(match detail {
                Some(detail) if !detail.is_empty() => {
                    format!("  {status} {step_name}: {detail}")
                }
                _ => format!("  {status} {step_name}"),
            });
        }
    }

    lines
}

fn primary_action(actions: &[Value]) -> Option<&Value> {
    actions
        .iter()
        .find(|action| json_string(action, &["install_reason"]) == Some("explicit"))
        .or_else(|| actions.first())
}

fn render_snapshot_summary(action: &Value) -> Option<String> {
    let snapshots = action.get("install")?.get("snapshots")?.as_array()?;
    if snapshots.is_empty() {
        return None;
    }

    let tool = snapshots
        .first()
        .and_then(|snapshot| snapshot.get("tool"))
        .and_then(|tool| tool.as_str())
        .unwrap_or("unknown");
    let captured = snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.get("status").and_then(|value| value.as_str()) == Some("captured")
        })
        .count();
    let failed = snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.get("status").and_then(|value| value.as_str()) == Some("failed")
        })
        .count();

    let mut summary = format!("{} via {tool}", snapshots.len());
    if captured > 0 {
        summary.push_str(&format!(", {captured} captured"));
    }
    if failed > 0 {
        summary.push_str(&format!(", {failed} failed"));
    }

    Some(summary)
}

fn action_package_name(action: &Value) -> String {
    json_string(action, &["package"])
        .or_else(|| json_string(action, &["package", "package_name"]))
        .unwrap_or("unknown")
        .to_owned()
}

fn action_version(action: &Value) -> String {
    if let Some(version) = json_string(action, &["version"]) {
        return version.to_owned();
    }

    let Some(epoch) = json_u64(action, &["package", "epoch"]) else {
        return "unknown".to_owned();
    };
    let Some(pkgver) = json_string(action, &["package", "pkgver"]) else {
        return "unknown".to_owned();
    };
    let Some(pkgrel) = json_u64(action, &["package", "pkgrel"]) else {
        return "unknown".to_owned();
    };

    format!("{epoch}:{pkgver}-{pkgrel}")
}

fn binary_trust_summary(action: &Value) -> Option<&'static str> {
    let verification = action.get("binary_source_verification")?;
    if !verification.is_object() {
        return None;
    }
    if verification
        .get("payload_signature")
        .is_some_and(|value| !value.is_null())
    {
        return Some("verified payload signature");
    }
    Some("verified remote payload")
}

pub(crate) fn render_section(title: &str, lines: &[String]) -> String {
    let mut rendered = String::from(title);
    for line in lines {
        rendered.push('\n');
        rendered.push_str("  ");
        rendered.push_str(line);
    }
    rendered
}

fn json_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_u64()
}

fn json_array<'a>(value: &'a Value, path: &[&str]) -> Option<&'a [Value]> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_array().map(Vec::as_slice)
}
