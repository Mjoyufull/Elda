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
