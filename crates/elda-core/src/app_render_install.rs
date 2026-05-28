use serde_json::Value;

mod data;

pub(crate) use data::{action_package_name, action_version, provenance_badge};

use crate::CommandReport;
use crate::app_render_support::{json_string, json_u64};
use crate::app_render_tree::{Frame, FrameFooter, Glyph, TreeStyle};
use crate::render_style::{color_enabled, highlight_operator_frame, paint};
use data::{
    action_activation_backend, binary_trust_summary, compact_plan_interbuild_summary, is_weak,
    object_summary, primary_action, render_snapshot_summary, version_change_rgb,
};

pub(crate) fn render_install_plan_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let plan_kind = details.get("plan")?.get("kind")?.as_str()?;
    if !matches!(plan_kind, "install" | "upgrade" | "source-ref-downgrade") {
        return None;
    }

    let actions = details.get("plan")?.get("actions")?.as_array()?;
    Some(render_compact_install_frame(
        report,
        details,
        actions,
        RenderShape::DryRunPlan,
    ))
}

pub(crate) fn render_install_plan_frame(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let actions = details.get("plan")?.get("actions")?.as_array()?;
    Some(render_compact_install_frame(
        report,
        details,
        actions,
        RenderShape::DryRunPlan,
    ))
}

pub(crate) fn render_install_success_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let installs = details.get("installs")?.as_array()?;
    Some(render_compact_install_frame(
        report,
        details,
        installs,
        RenderShape::Result,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderShape {
    DryRunPlan,
    Result,
}

fn render_compact_install_frame(
    report: &CommandReport,
    details: &Value,
    actions: &[Value],
    shape: RenderShape,
) -> String {
    let title = compact_title(report, actions, shape);
    let footer = compact_footer(report, actions, shape);
    let mut frame = Frame::new(title);
    apply_compact_rows(&mut frame, &compact_lines(report, details, actions, shape));
    if let Some(footer) = footer {
        frame.footer(footer);
    }

    highlight_operator_frame(&frame.render(TreeStyle::detect()))
}

fn compact_title(report: &CommandReport, actions: &[Value], shape: RenderShape) -> String {
    let subject = compact_subject(report, actions);
    let verb = plan_verb(report);
    match shape {
        RenderShape::DryRunPlan => format!("{verb} {subject}"),
        RenderShape::Result => format!("installed {subject}"),
    }
}

fn plan_verb(report: &CommandReport) -> &str {
    report
        .details
        .as_ref()
        .and_then(|details| details.get("plan"))
        .and_then(|plan| plan.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("install")
}

fn compact_subject(report: &CommandReport, actions: &[Value]) -> String {
    if !report.operands.is_empty() {
        return report.operands.join(", ");
    }
    primary_action(actions)
        .map(action_package_name)
        .unwrap_or_else(|| "target".to_owned())
}

fn compact_footer(
    report: &CommandReport,
    actions: &[Value],
    shape: RenderShape,
) -> Option<FrameFooter> {
    match shape {
        RenderShape::DryRunPlan => Some(FrameFooter {
            glyph: None,
            text: if report.dry_run {
                "dry run".to_owned()
            } else {
                "Proceed? [Y/n/e]".to_owned()
            },
        }),
        RenderShape::Result => Some(FrameFooter {
            glyph: Some(Glyph::Done),
            text: compact_result_summary(report, actions),
        }),
    }
}

fn compact_result_summary(report: &CommandReport, actions: &[Value]) -> String {
    if !report.summary.trim().is_empty() {
        return report.summary.trim().to_owned();
    }
    format!("{} action(s) completed", actions.len())
}

fn compact_lines(
    report: &CommandReport,
    details: &Value,
    actions: &[Value],
    shape: RenderShape,
) -> Vec<(String, String)> {
    let Some(action) = primary_action(actions) else {
        return vec![("status".to_owned(), "no package actions".to_owned())];
    };

    if shape == RenderShape::Result {
        return compact_result_rows(action, actions);
    }

    let mut rows = vec![
        ("target".to_owned(), action_package_name(action)),
        ("version".to_owned(), format_version_value(action)),
        (
            "source".to_owned(),
            format!(
                "{} {}/{}",
                provenance_badge(action),
                json_string(action, &["selected_lane"]).unwrap_or("unknown"),
                json_string(action, &["selected_source_kind"]).unwrap_or("unknown"),
            ),
        ),
        (
            "activate".to_owned(),
            format!(
                "{} ({}) via {}",
                json_string(details, &["layout", "prefix"]).unwrap_or("unknown"),
                json_string(details, &["layout", "mode"]).unwrap_or("unknown"),
                action_activation_backend(action).unwrap_or("unknown"),
            ),
        ),
    ];

    push_source_context_rows(&mut rows, action, shape);
    push_policy_rows(&mut rows, action);
    push_review_row(&mut rows, details);
    push_change_row(&mut rows, actions);
    push_space_row(&mut rows, details);
    push_safety_row(&mut rows, details, actions);

    if report.dry_run {
        rows.push((
            "steps".to_owned(),
            "acquire, build, stage, activate".to_owned(),
        ));
    }

    rows
}

fn apply_compact_rows(frame: &mut Frame, rows: &[(String, String)]) {
    for (key, value) in rows {
        frame.kv(key, value);
    }
}

fn compact_result_rows(action: &Value, actions: &[Value]) -> Vec<(String, String)> {
    let mut rows = vec![("target".to_owned(), action_package_name(action))];
    let state = json_string(action, &["install", "state_id"]).unwrap_or("unknown");
    let paths = json_u64(action, &["install", "installed_paths"]).unwrap_or(0);
    rows.push(("state".to_owned(), state.to_owned()));
    rows.push(("paths".to_owned(), paths.to_string()));
    if let Some(summary) = object_summary(action) {
        rows.push(("objects".to_owned(), summary));
    }
    if let Some(summary) = actions.iter().filter_map(render_snapshot_summary).next() {
        rows.push(("snapshots".to_owned(), summary));
    }
    rows
}

fn push_policy_rows(rows: &mut Vec<(String, String)>, action: &Value) {
    if let Some(reason) = json_string(action, &["blocked_reason"]) {
        rows.push(("policy".to_owned(), reason.to_owned()));
    }
    if json_string(action, &["blocked_reason"]) == Some("git-ref-pinned") {
        rows.push(("git ref".to_owned(), "pinned".to_owned()));
    }
}

fn push_source_context_rows(rows: &mut Vec<(String, String)>, action: &Value, shape: RenderShape) {
    if let Some(path) = json_string(action, &["generated_metadata_path"]) {
        rows.push(("metadata".to_owned(), path.to_owned()));
    }
    if shape == RenderShape::DryRunPlan
        && let Some(summary) = compact_plan_interbuild_summary(action)
    {
        let value = summary.strip_prefix("parser ").unwrap_or(summary.as_str());
        rows.push(("interbuild parser".to_owned(), value.to_owned()));
    }
    if let Some(trust) = binary_trust_summary(action) {
        rows.push(("trust".to_owned(), trust.to_owned()));
    }
}

fn format_version_value(action: &Value) -> String {
    let version = action_version(action);
    if !color_enabled() {
        return version;
    }
    let Some(rgb) = version_change_rgb(action) else {
        return version;
    };
    paint(&version, rgb, false)
}

fn push_review_row(rows: &mut Vec<(String, String)>, details: &Value) {
    let Some(review) = details
        .get("preflight")
        .and_then(|value| value.get("review"))
    else {
        return;
    };
    let needs = json_u64(review, &["needs_review"]).unwrap_or(0);
    if needs == 0 {
        rows.push(("review".to_owned(), "current".to_owned()));
    } else {
        rows.push(("review".to_owned(), format!("{needs} pending")));
    }
}

fn push_change_row(rows: &mut Vec<(String, String)>, actions: &[Value]) {
    let install = actions
        .iter()
        .filter(|action| !is_keep_action(action))
        .count();
    let keep = actions.len().saturating_sub(install);
    let replaced_names = actions
        .iter()
        .filter_map(|action| action.get("replaced_packages").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let replace = replaced_names.len();
    let weak = actions.iter().filter(|action| is_weak(action)).count();
    let replace_detail = if replaced_names.is_empty() {
        String::new()
    } else {
        format!(" ({})", replaced_names.join(", "))
    };
    rows.push((
        "change".to_owned(),
        format!("install {install}, keep {keep}, replace {replace}, weak {weak}{replace_detail}"),
    ));
}

fn is_keep_action(action: &Value) -> bool {
    matches!(
        json_string(action, &["action"]).unwrap_or("install"),
        "keep" | "keep-installed"
    )
}

fn push_space_row(rows: &mut Vec<(String, String)>, details: &Value) {
    let Some(preflight) = details.get("preflight") else {
        return;
    };
    let payload = json_u64(preflight, &["estimated_post_build_managed_bytes"])
        .or_else(|| json_u64(preflight, &["net_reinstall_managed_bytes"]))
        .map(human_bytes)
        .unwrap_or_else(|| {
            json_string(preflight, &["candidate_size_status"])
                .unwrap_or("unknown")
                .to_owned()
        });
    let root = json_u64(preflight, &["root_free_bytes"])
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let tmp = json_u64(preflight, &["tmp_free_bytes"])
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    rows.push((
        "space".to_owned(),
        format!("{payload} payload, {root} root free, {tmp} tmp free"),
    ));
}

fn push_safety_row(rows: &mut Vec<(String, String)>, details: &Value, actions: &[Value]) {
    let Some(preflight) = details.get("preflight") else {
        rows.push(("safety".to_owned(), "preflight unavailable".to_owned()));
        return;
    };
    let review = preflight
        .get("review")
        .and_then(|review| json_u64(review, &["needs_review"]))
        .unwrap_or(0);
    let config = json_u64(preflight, &["pending_config_files"]).unwrap_or(0);
    let triggers = preflight
        .get("triggers")
        .and_then(|value| value.get("pending_repair"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let snapshots = actions
        .iter()
        .filter_map(render_snapshot_summary)
        .next()
        .unwrap_or_else(|| "none/off".to_owned());
    rows.push((
        "safety review".to_owned(),
        format!("{review}, config {config}, triggers {triggers}, snapshots {snapshots}"),
    ));
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{} B", bytes as u64)
    }
}
