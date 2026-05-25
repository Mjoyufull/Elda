use serde_json::Value;

use crate::CommandReport;
use crate::app_render_install::{action_package_name, action_version, provenance_badge};
use crate::app_render_support::json_string;
use crate::app_render_tree::{Frame, FrameFooter, TreeStyle};

pub(crate) fn render_remove_plan_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if details.get("plan")?.get("kind")?.as_str()? != "remove" {
        return None;
    }
    let actions = details.get("plan")?.get("actions")?.as_array()?;
    Some(render_compact_remove_frame(report, details, actions))
}

pub(crate) fn render_remove_plan_frame(report: &CommandReport) -> Option<String> {
    render_remove_plan_report(report)
}

pub(crate) fn render_remove_success_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let removals = details.get("removals")?.as_array()?;
    if removals.is_empty() {
        return Some(format!("✔ {}", report.summary.trim()));
    }
    if removals.len() == 1 {
        let removal = &removals[0];
        let name = json_string(removal, &["package_name"]).unwrap_or("package");
        let paths = removal
            .get("removed_paths")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        return Some(format!("✔ removed {name} ({paths} paths)"));
    }
    Some(format!("✔ {}", report.summary.trim()))
}

fn render_compact_remove_frame(
    report: &CommandReport,
    details: &Value,
    actions: &[Value],
) -> String {
    let subject = if !report.operands.is_empty() {
        report
            .operands
            .iter()
            .filter(|operand| !operand.starts_with('-'))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        action_package_name(actions.first().unwrap_or(&Value::Null))
    };

    let mut frame = Frame::new(format!("remove {subject}"));
    for action in actions {
        for line in remove_action_lines(action, details) {
            frame.line(line);
        }
    }
    if actions.len() > 1 {
        frame.line(format!("packages {}", actions.len()));
    }
    let plan = details.get("plan").unwrap_or(details);
    let cascade = plan
        .get("cascade")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let purge = plan
        .get("purge_conffiles")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    frame.line(format!(
        "policy cascade {}, purge-conffiles {}",
        if cascade { "on" } else { "off" },
        if purge { "on" } else { "off" }
    ));
    frame.footer(FrameFooter {
        glyph: None,
        text: if report.dry_run {
            "dry run".to_owned()
        } else {
            "Proceed? [Y/n/e]".to_owned()
        },
    });
    frame.render(TreeStyle::detect())
}

fn remove_action_lines(action: &Value, details: &Value) -> Vec<String> {
    let prefix = json_string(details, &["layout", "prefix"]).unwrap_or("/usr");
    let paths = action
        .get("installed_paths")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    vec![
        format!("target {}", action_package_name(action)),
        format!("version {}", action_version(action)),
        format!(
            "source {} {}",
            provenance_badge(action),
            json_string(action, &["selected_source_kind"]).unwrap_or("unknown")
        ),
        format!("paths {paths} under {prefix}"),
    ]
}
