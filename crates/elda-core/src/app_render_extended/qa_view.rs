use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{
    format_aligned_kv, human_operator_frame, json_string, kv_label_width,
};

pub(super) fn render_qa(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if report.status == "planned" {
        let plan = details.get("plan")?;
        let labels = ["targets", "packages", "entry", "more"];
        let width = kv_label_width(&labels);
        let mut lines = Vec::new();
        if let Some(targets) = plan.get("requested_targets").and_then(Value::as_array) {
            lines.push(format_aligned_kv(
                "targets",
                &targets.len().to_string(),
                width,
            ));
        }
        if let Some(packages) = plan.get("packages").and_then(Value::as_array) {
            lines.push(format_aligned_kv(
                "packages",
                &packages.len().to_string(),
                width,
            ));
            for package in packages.iter().take(12) {
                let package_name = json_string(package, &["pkgname"]).unwrap_or("?");
                lines.push(format_aligned_kv("entry", package_name, width));
            }
            if packages.len() > 12 {
                lines.push(format_aligned_kv(
                    "more",
                    &format!("{}", packages.len() - 12),
                    width,
                ));
            }
        }
        return Some(human_operator_frame("qa plan", lines, None));
    }

    if let Some(lint) = details.get("lint") {
        return Some(render_lint(lint));
    }

    if let Some(builds) = details.get("builds").and_then(Value::as_array) {
        return Some(render_builds(builds));
    }

    Some(human_operator_frame(
        "qa",
        vec![format_aligned_kv(
            "detail",
            "see --json for full QA report",
            6,
        )],
        None,
    ))
}

fn render_lint(lint: &Value) -> String {
    let labels = ["recipes", "issues", "issue", "more"];
    let width = kv_label_width(&labels);
    let mut lines = Vec::new();
    if let Some(recipes) = lint.get("recipes").and_then(Value::as_array) {
        lines.push(format_aligned_kv(
            "recipes",
            &recipes.len().to_string(),
            width,
        ));
    }
    if let Some(issues) = lint.get("issues").and_then(Value::as_array) {
        lines.push(format_aligned_kv(
            "issues",
            &issues.len().to_string(),
            width,
        ));
        for issue in issues.iter().take(16) {
            let recipe = json_string(issue, &["recipe"]).unwrap_or("?");
            let message = json_string(issue, &["message"]).unwrap_or("");
            lines.push(format_aligned_kv(
                "issue",
                &format!("{recipe}: {message}"),
                width,
            ));
        }
        if issues.len() > 16 {
            lines.push(format_aligned_kv(
                "more",
                &format!("{}", issues.len() - 16),
                width,
            ));
        }
    }
    human_operator_frame("qa lint", lines, None)
}

fn render_builds(builds: &[Value]) -> String {
    let labels = ["builds", "entry", "more"];
    let width = kv_label_width(&labels);
    let mut lines = vec![format_aligned_kv(
        "builds",
        &builds.len().to_string(),
        width,
    )];
    for build in builds.iter().take(16) {
        let package_name = json_string(build, &["pkgname"]).unwrap_or("?");
        lines.push(format_aligned_kv("entry", package_name, width));
    }
    if builds.len() > 16 {
        lines.push(format_aligned_kv(
            "more",
            &format!("{}", builds.len() - 16),
            width,
        ));
    }
    human_operator_frame("qa build", lines, None)
}
