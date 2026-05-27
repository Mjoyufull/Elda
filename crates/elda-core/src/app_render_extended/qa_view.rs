use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

pub(super) fn render_qa(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if report.status == "planned" {
        let plan = details.get("plan")?;
        return Some(human_framed_report(
            report,
            "QA plan",
            &[("Plan".to_owned(), plan_lines(plan))],
            None,
        ));
    }

    if let Some(lint) = details.get("lint") {
        return Some(human_framed_report(
            report,
            "QA lint",
            &[("Lint".to_owned(), lint_lines(lint))],
            None,
        ));
    }

    if let Some(builds) = details.get("builds").and_then(Value::as_array) {
        return Some(human_framed_report(
            report,
            "QA build",
            &[("Builds".to_owned(), build_lines(builds))],
            None,
        ));
    }

    Some(human_framed_report(
        report,
        "QA",
        &[(
            "Summary".to_owned(),
            vec!["see --json for full QA report".to_owned()],
        )],
        None,
    ))
}

fn plan_lines(plan: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(targets) = plan.get("requested_targets").and_then(Value::as_array) {
        lines.push(format!("targets: {}", targets.len()));
    }
    if let Some(packages) = plan.get("packages").and_then(Value::as_array) {
        lines.push(format!("packages: {}", packages.len()));
        for package in packages.iter().take(12) {
            lines.push(format!(
                "entry: {}",
                json_string(package, &["pkgname"]).unwrap_or("?")
            ));
        }
        push_more(&mut lines, packages.len(), 12);
    }
    lines
}

fn lint_lines(lint: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(recipes) = lint.get("recipes").and_then(Value::as_array) {
        lines.push(format!("recipes: {}", recipes.len()));
    }
    if let Some(issues) = lint.get("issues").and_then(Value::as_array) {
        lines.push(format!("issues: {}", issues.len()));
        for issue in issues.iter().take(16) {
            let recipe = json_string(issue, &["recipe"]).unwrap_or("?");
            let message = json_string(issue, &["message"]).unwrap_or("");
            lines.push(format!("issue: {recipe}: {message}"));
        }
        push_more(&mut lines, issues.len(), 16);
    }
    lines
}

fn build_lines(builds: &[Value]) -> Vec<String> {
    let mut lines = vec![format!("builds: {}", builds.len())];
    for build in builds.iter().take(16) {
        lines.push(format!(
            "entry: {}",
            json_string(build, &["pkgname"]).unwrap_or("?")
        ));
    }
    push_more(&mut lines, builds.len(), 16);
    lines
}

fn push_more(lines: &mut Vec<String>, total: usize, shown: usize) {
    if total > shown {
        lines.push(format!("more: {}", total - shown));
    }
}
