use serde_json::Value;

mod data;

use crate::CommandReport;
use crate::app_render_misc::source_option_lines;
use crate::app_render_support::{json_string, json_u64};
use crate::app_render_tree::{FrameFooter, Glyph, TreeStyle, frame_from_sections};
use data::{
    action_activation_backend, action_package_name, action_version, binary_trust_summary,
    confidence_modifier, interbuild_summary, is_weak, object_summary, primary_action,
    provenance_badge, push_optional_line, render_snapshot_summary, report_actions,
    snapshot_risk_lines, weak_suffix,
};

pub(crate) fn render_install_plan_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let plan_kind = details.get("plan")?.get("kind")?.as_str()?;
    if !matches!(plan_kind, "install" | "upgrade" | "source-ref-downgrade") {
        return None;
    }

    let actions = details.get("plan")?.get("actions")?.as_array()?;
    Some(render_install_report_sections(
        report,
        details,
        actions,
        RenderShape::DryRunPlan,
    ))
}

pub(crate) fn render_install_success_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let installs = details.get("installs")?.as_array()?;
    Some(render_install_report_sections(
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

fn render_install_report_sections(
    report: &CommandReport,
    details: &Value,
    actions: &[Value],
    shape: RenderShape,
) -> String {
    let mut sections: Vec<(String, Vec<String>)> = vec![
        ("Target".to_owned(), target_lines(report, details)),
        ("Resolution".to_owned(), resolution_lines(actions)),
        ("Provenance".to_owned(), provenance_lines(actions)),
        ("Plan".to_owned(), plan_lines(actions)),
        ("Risk".to_owned(), risk_lines(actions)),
        ("Preflight".to_owned(), preflight_lines(details)),
    ];

    // The live progress sink already streamed every step to the operator;
    // re-emitting the same step list under a Progress section would just
    // duplicate what they saw. Keep it only on dry-run plans, where no
    // live tree ever fired.
    if shape == RenderShape::DryRunPlan {
        sections.push(("Progress".to_owned(), progress_lines(actions)));
    }
    sections.push(("Artifacts".to_owned(), artifact_lines(actions)));
    if shape == RenderShape::Result {
        let mut result = result_lines(actions);
        if let Some(policy) = details
            .get("preflight")
            .and_then(|preflight| preflight.get("shared_path_policy"))
        {
            if let Some(terminfo) = json_string(policy, &["terminfo"]) {
                result.push(format!("shared terminfo: {terminfo}"));
            }
            if let Some(collisions) = json_string(policy, &["collisions"]) {
                result.push(format!("shared-path collisions: {collisions}"));
            }
        }
        sections.push(("Result".to_owned(), result));
    }

    let title = frame_title(report, shape);
    let footer = frame_footer(report, shape, actions);
    let frame = frame_from_sections(title, &sections, footer);

    let header = format!("{}: {}", report.area, report.status);
    let summary = report.summary.clone();
    let body = frame.render(TreeStyle::detect());
    format!("{header}\n{summary}\n\n{body}")
}

fn frame_title(report: &CommandReport, shape: RenderShape) -> String {
    let head = report
        .command_path
        .first()
        .map(String::as_str)
        .unwrap_or("");
    let prefix = match (shape, head) {
        (RenderShape::DryRunPlan, _) => "Elda Transaction Plan",
        (RenderShape::Result, "u" | "upgrade") => "Elda System Upgrade",
        (RenderShape::Result, _) => "Install Result",
    };
    if report.operands.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {}", report.operands.join(", "))
    }
}

fn frame_footer(
    report: &CommandReport,
    shape: RenderShape,
    actions: &[Value],
) -> Option<FrameFooter> {
    match shape {
        RenderShape::DryRunPlan => Some(FrameFooter {
            glyph: None,
            text: "Proceed? [Y/n/e] (dry run)".to_owned(),
        }),
        RenderShape::Result => {
            let count = actions.len();
            let summary = report.summary.trim();
            let text = if summary.is_empty() {
                format!("{count} action(s) completed")
            } else {
                summary.to_owned()
            };
            Some(FrameFooter {
                glyph: Some(Glyph::Done),
                text,
            })
        }
    }
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
        primary_action(report_actions(details))
            .and_then(action_activation_backend)
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
        format!("provenance: {}", provenance_badge(action)),
    ];

    if let Some(remote_name) = json_string(action, &["remote_name"]) {
        lines.push(format!("remote: {remote_name}"));
    }
    if let Some(source_ref) = json_string(action, &["source_ref"]) {
        lines.push(format!("source ref: {source_ref}"));
    }
    if json_string(action, &["persisted_source_kind"]) == Some("git") {
        let ref_policy = match action.get("ad_hoc_git_moving").and_then(Value::as_bool) {
            Some(true) => "moving",
            Some(false) => "pinned",
            None => "unknown",
        };
        lines.push(format!("git ref policy: {ref_policy}"));
    }
    if let Some(commit) = json_string(action, &["installed_repo_commit"]) {
        lines.push(format!("installed commit: {commit}"));
    }
    if let Some(commit) = json_string(action, &["candidate_repo_commit"]) {
        lines.push(format!("candidate commit: {commit}"));
    }
    if let Some(generated_metadata_path) = json_string(action, &["generated_metadata_path"]) {
        lines.push(format!("generated metadata: {generated_metadata_path}"));
    }
    lines.extend(source_option_lines(action, source_option_parent(action)));
    if let Some(repo_commit) = json_string(action, &["package", "repo_commit"]) {
        lines.push(format!("repo commit: {repo_commit}"));
    }
    if let Some(trust) = binary_trust_summary(action) {
        lines.push(format!("trust: {trust}"));
    }

    lines
}

fn source_option_parent(action: &Value) -> &Value {
    action
}

fn provenance_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no package provenance".to_owned()];
    }

    actions
        .iter()
        .flat_map(|action| {
            let remote = json_string(action, &["remote_name"]).unwrap_or("local");
            let source_ref = json_string(action, &["source_ref"]).unwrap_or("none");
            let mut lines = vec![format!(
                "{}: {} {}, remote {}, source ref {}",
                action_package_name(action),
                provenance_badge(action),
                confidence_modifier(action),
                remote,
                source_ref,
            )];
            if let Some(summary) = interbuild_summary(action) {
                lines.push(format!(
                    "{} interbuild: {summary}",
                    action_package_name(action)
                ));
            }
            lines
        })
        .collect()
}

fn plan_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no actions".to_owned()];
    }

    actions
        .iter()
        .map(|action| {
            let blocked = json_string(action, &["blocked_reason"])
                .map(|reason| format!(", reason {reason}"))
                .unwrap_or_default();
            let replacement = replacement_detail(action)
                .map(|detail| format!(", {detail}"))
                .unwrap_or_default();
            format!(
                "{} {} {} [{} / {}{}]{}{}",
                json_string(action, &["action"]).unwrap_or("plan"),
                action_package_name(action),
                action_version(action),
                json_string(action, &["selected_lane"]).unwrap_or("unknown"),
                json_string(action, &["selected_source_kind"]).unwrap_or("unknown"),
                blocked,
                weak_suffix(action),
                replacement,
            )
        })
        .collect()
}

fn replacement_detail(action: &Value) -> Option<String> {
    let targets = replacement_targets(action);
    if targets.is_empty() {
        None
    } else {
        Some(format!("replaces {}", targets.join(", ")))
    }
}

fn replacement_targets(action: &Value) -> Vec<String> {
    action
        .get("replaced_packages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn risk_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no transaction risk data".to_owned()];
    }

    let mut lines = Vec::new();
    let replacements = actions
        .iter()
        .filter_map(|action| action.get("replaced_packages").and_then(Value::as_array))
        .map(Vec::len)
        .sum::<usize>();
    let weak_deps = actions.iter().filter(|action| is_weak(action)).count();
    let generated = actions
        .iter()
        .filter(|action| json_string(action, &["generated_metadata_path"]).is_some())
        .count();
    let foreign = actions
        .iter()
        .filter(|action| matches!(provenance_badge(action), "[I]" | "[F]" | "[A]" | "[V]"))
        .count();

    lines.push(format!("actions: {} total", actions.len()));
    lines.push(format!("replacements: {replacements}"));
    if replacements > 0 {
        for action in actions {
            if let Some(detail) = replacement_detail(action) {
                lines.push(format!("  {}: {detail}", action_package_name(action)));
            }
        }
    }
    lines.push(format!("weak dependencies: {weak_deps}"));
    lines.push(format!("generated metadata reviews: {generated}"));
    lines.push(format!("non-native provenance actions: {foreign}"));
    lines.extend(snapshot_risk_lines(actions));
    lines
}

fn preflight_lines(details: &Value) -> Vec<String> {
    let Some(preflight) = details.get("preflight") else {
        return vec!["preflight data unavailable".to_owned()];
    };

    let mut lines = Vec::new();
    if let Some(actions) = json_u64(preflight, &["actions"]) {
        lines.push(format!("actions: {actions}"));
    }
    if let Some(changing) = json_u64(preflight, &["changing_actions"]) {
        lines.push(format!("changing actions: {changing}"));
    }
    if let Some(replacements) = json_u64(preflight, &["replacement_targets"]) {
        lines.push(format!("replacement targets: {replacements}"));
    }
    if let Some(bytes) = json_u64(preflight, &["known_existing_managed_bytes"]) {
        lines.push(format!("known existing managed bytes: {bytes}"));
    }
    if let Some(bytes) = json_u64(preflight, &["known_replaced_managed_bytes"]) {
        lines.push(format!("known replaced managed bytes: {bytes}"));
    }
    if let Some(status) = json_string(preflight, &["candidate_size_status"]) {
        lines.push(format!("candidate size: {status}"));
    }
    for (label, key) in [
        ("root free bytes", "root_free_bytes"),
        ("cache free bytes", "cache_free_bytes"),
        ("tmp free bytes", "tmp_free_bytes"),
    ] {
        match json_u64(preflight, &[key]) {
            Some(bytes) => lines.push(format!("{label}: {bytes}")),
            None => lines.push(format!("{label}: unknown")),
        }
    }
    if let Some(review) = preflight.get("review") {
        if let Some(needs) = json_u64(review, &["needs_review"]) {
            lines.push(format!("review gates needing attention: {needs}"));
        }
        if let Some(entries) = review.get("entries").and_then(Value::as_array) {
            for entry in entries.iter().take(8) {
                let package = json_string(entry, &["package"]).unwrap_or("?");
                let kind = json_string(entry, &["review_kind"]).unwrap_or("?");
                let status = json_string(entry, &["status"]).unwrap_or("?");
                lines.push(format!("  {package} ({kind}): {status}"));
            }
            if entries.len() > 8 {
                lines.push(format!("  … {} more review entries", entries.len() - 8));
            }
        }
    }
    if let Some(bytes) = json_u64(preflight, &["net_reinstall_managed_bytes"]) {
        lines.push(format!("net reinstall managed bytes: {bytes}"));
    }
    if let Some(bytes) = json_u64(preflight, &["estimated_post_build_managed_bytes"]) {
        lines.push(format!("estimated post-build managed bytes: {bytes}"));
    }
    if let Some(method) = json_string(preflight, &["payload_size_estimate_method"]) {
        lines.push(format!("payload size estimate: {method}"));
    }
    if let Some(deps) = preflight
        .get("temporary_build_dependencies")
        .and_then(|value| value.get("packages"))
        .and_then(Value::as_array)
        && !deps.is_empty()
    {
        lines.push(format!("temporary build dependencies: {}", deps.len()));
    }
    if let Some(keys) = preflight
        .get("source_keys")
        .and_then(|value| value.get("missing_for_plan"))
        .and_then(Value::as_array)
        && !keys.is_empty()
    {
        lines.push(format!("missing release trust keys: {}", keys.len()));
    }
    if let Some(count) = json_u64(preflight, &["weak_dependencies"]) {
        lines.push(format!("weak dependencies: {count}"));
    }
    if let Some(count) = json_u64(preflight, &["pending_config_files"]) {
        lines.push(format!("pending configuration files: {count}"));
    }
    if let Some(triggers) = preflight.get("triggers") {
        if let Some(pending) = triggers.get("pending_repair").and_then(Value::as_array) {
            lines.push(format!("pending trigger repairs: {}", pending.len()));
        }
    }
    if let Some(snapshot) = preflight.get("snapshot_intent") {
        if let Some(tool) = json_string(snapshot, &["configured_tool"]) {
            lines.push(format!("snapshot tool: {tool}"));
        }
        if let Some(will) = snapshot.get("will_request").and_then(Value::as_bool) {
            lines.push(format!("snapshot requested: {will}"));
        }
    }
    if let Some(keys) = preflight.get("source_keys") {
        if let Some(count) = json_u64(keys, &["release_keys_configured"]) {
            lines.push(format!("configured release keys: {count}"));
        }
    }
    if let Some(policy) = preflight.get("shared_path_policy") {
        if let Some(terminfo) = json_string(policy, &["terminfo"]) {
            lines.push(format!("shared terminfo: {terminfo}"));
        }
        if let Some(collisions) = json_string(policy, &["collisions"]) {
            lines.push(format!("shared-path collisions: {collisions}"));
        }
    }
    if let Some(privilege) = preflight.get("privilege") {
        if let Some(escalation) = json_string(privilege, &["escalation"]) {
            lines.push(format!("privilege: {escalation}"));
        }
    }
    if let Some(policy) = json_string(preflight, &["policy"]) {
        lines.push(format!("policy: {policy}"));
    }
    lines
}

fn progress_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no steps".to_owned()];
    }

    let mut lines = Vec::new();
    for action in actions {
        let package_name = action_package_name(action);
        let Some(steps) = action.get("progress").and_then(Value::as_array) else {
            lines.push(format!("{package_name}: progress unavailable"));
            continue;
        };

        lines.push(format!("{package_name}:"));
        for step in steps {
            let step_name = json_string(step, &["step"]).unwrap_or("unknown-step");
            let status = json_string(step, &["status"]).unwrap_or("unknown");
            let detail = step.get("detail").and_then(Value::as_str);
            lines.push(match detail {
                Some(detail) if !detail.is_empty() => format!("  {status} {step_name}: {detail}"),
                _ => format!("  {status} {step_name}"),
            });
        }
    }

    lines
}

fn artifact_lines(actions: &[Value]) -> Vec<String> {
    if actions.is_empty() {
        return vec!["no artifacts".to_owned()];
    }

    let mut lines = Vec::new();
    for action in actions {
        lines.push(format!("{}:", action_package_name(action)));
        push_optional_line(
            &mut lines,
            "  payload sha256",
            json_string(action, &["package", "payload_sha256"]),
        );
        push_optional_line(
            &mut lines,
            "  manifest hash",
            json_string(action, &["package", "manifest_hash"]),
        );
        push_optional_line(
            &mut lines,
            "  state id",
            json_string(action, &["install", "state_id"]),
        );
        if let Some(paths) = json_u64(action, &["install", "installed_paths"]) {
            lines.push(format!("  managed paths: {paths}"));
        }
        if let Some(summary) = object_summary(action) {
            lines.push(format!("  objects: {summary}"));
        }
        if let Some(summary) = render_snapshot_summary(action) {
            lines.push(format!("  snapshots: {summary}"));
        }
    }

    lines
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
            let activation_backend = action_activation_backend(action);
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
