use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};
use crate::app_render_tree::FrameFooter;

pub(crate) fn render_extended_plan_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let plan = details.get("plan")?;
    if let Some(kind) = plan.get("kind").and_then(|k| k.as_str()) {
        return match kind {
            "remove" | "autoremove" => Some(render_removal_plan(report, plan, kind)),
            "rollback" => Some(render_rollback_plan(report, plan)),
            "downgrade" => Some(render_downgrade_plan(report, plan)),
            "state-import" => Some(render_state_import_plan(report, plan)),
            _ => None,
        };
    }
    // Rollback dry-run embeds `RollbackPlan` without a `kind` field.
    if plan.get("to_state").is_some() && plan.get("removed_packages").is_some() {
        return Some(render_rollback_plan(report, plan));
    }
    None
}

fn render_removal_plan(report: &CommandReport, plan: &Value, kind: &str) -> String {
    let title = if kind == "autoremove" {
        "Autoremove Plan"
    } else {
        "Elda Removal Plan"
    };
    let actions = plan
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let lines: Vec<String> = actions
        .iter()
        .map(|a| {
            let action = json_string(a, &["action"]).unwrap_or("action");
            let target = json_string(a, &["target"]).unwrap_or("?");
            format!("{action} {target}")
        })
        .collect();
    human_framed_report(
        report,
        title,
        &[("Targets".to_owned(), lines)],
        Some(FrameFooter {
            glyph: None,
            text: "Proceed? [Y/n]".to_owned(),
        }),
    )
}

fn render_rollback_plan(report: &CommandReport, plan: &Value) -> String {
    let mut lines = vec![
        format!(
            "to state: {}",
            json_string(plan, &["to_state"]).unwrap_or("unknown")
        ),
        format!(
            "from state: {}",
            json_string(plan, &["from_state"]).unwrap_or("none")
        ),
    ];
    if let Some(rm) = plan.get("removed_packages").and_then(Value::as_array) {
        lines.push(format!(
            "packages removed in rollback: {}",
            rm.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(rs) = plan.get("restored_packages").and_then(Value::as_array) {
        lines.push(format!(
            "packages restored: {}",
            rs.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    human_framed_report(
        report,
        "State Rollback Plan",
        &[("Plan".to_owned(), lines)],
        Some(FrameFooter {
            glyph: None,
            text: "Proceed? [Y/n]".to_owned(),
        }),
    )
}

fn render_downgrade_plan(report: &CommandReport, plan: &Value) -> String {
    let pkg = json_string(plan, &["package"]).unwrap_or("?");
    let installed = json_string(plan, &["installed_version"]).unwrap_or("?");
    let cand_ver = plan
        .get("candidate")
        .map(|cand| {
            format!(
                "{}:{}-{}",
                json_u64(cand, &["epoch"]).unwrap_or(0),
                json_string(cand, &["pkgver"]).unwrap_or("?"),
                json_u64(cand, &["pkgrel"]).unwrap_or(1)
            )
        })
        .unwrap_or_else(|| "unknown".to_owned());
    let lines = vec![
        format!("package: {pkg}"),
        format!("installed: {installed}"),
        format!("candidate: {cand_ver}"),
    ];
    human_framed_report(
        report,
        format!("Downgrade: {pkg}"),
        &[("Plan".to_owned(), lines)],
        Some(FrameFooter {
            glyph: None,
            text: "Proceed? [Y/n]".to_owned(),
        }),
    )
}

fn render_state_import_plan(report: &CommandReport, plan: &Value) -> String {
    let mut lines = Vec::new();
    if let Some(remotes) = plan.get("remotes").and_then(Value::as_array) {
        lines.push(format!("remotes: {}", remotes.len()));
        for r in remotes.iter().take(12) {
            if let Some(name) = json_string(r, &["name"]) {
                lines.push(format!("  remote {name}"));
            }
        }
    }
    if let Some(world) = plan.get("world").and_then(Value::as_array) {
        lines.push(format!("world anchors: {}", world.len()));
        for w in world.iter().take(20).filter_map(Value::as_str) {
            lines.push(format!("  - {w}"));
        }
    }
    human_framed_report(
        report,
        "State Import Plan",
        &[("Plan".to_owned(), lines)],
        Some(FrameFooter {
            glyph: None,
            text: "Proceed? [Y/n]".to_owned(),
        }),
    )
}
