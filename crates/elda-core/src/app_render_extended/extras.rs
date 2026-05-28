use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

use super::helpers::{push_kv, recipe_package_lines};

pub(super) fn render_recipe_show(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let show = details.get("show")?;
    let package = json_string(show, &["package"]).unwrap_or("?");
    let selected_source = json_string(show, &["selected_source"]);
    let source = selected_recipe_source(show)?;
    let recipe = source.get("recipe")?;
    let package_def = recipe.get("package")?;

    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut identity = vec![format!("package: {package}")];
    push_kv(&mut identity, "selected source", selected_source);
    push_kv(&mut identity, "path", json_string(recipe, &["path"]));
    sections.push(("Identity".to_owned(), identity));

    let mut metadata = Vec::new();
    recipe_package_lines(&mut metadata, package_def);
    push_kv(&mut metadata, "kind", json_string(package_def, &["kind"]));
    push_kv(
        &mut metadata,
        "source kind",
        json_string(package_def, &["source", "kind"]),
    );
    sections.push(("Metadata".to_owned(), metadata));

    let relationships = relationship_lines(package_def);
    if !relationships.is_empty() {
        sections.push(("Relationships".to_owned(), relationships));
    }

    let policy = recipe_policy_lines(package_def);
    if !policy.is_empty() {
        sections.push(("Policy".to_owned(), policy));
    }

    if let Some(validation) = source.get("validation") {
        sections.push(("Validation".to_owned(), validation_lines(validation)));
    }

    Some(human_framed_report(
        report,
        format!("Recipe: {package}"),
        &sections,
        None,
    ))
}

fn selected_recipe_source(show: &Value) -> Option<&Value> {
    show.get("local")
        .filter(|value| !value.is_null())
        .or_else(|| show.get("synced").filter(|value| !value.is_null()))
}

fn relationship_lines(package: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_count(&mut lines, "depends", package.get("depends"));
    push_count(&mut lines, "makedepends", package.get("makedepends"));
    push_count(&mut lines, "checkdepends", package.get("checkdepends"));
    push_count(&mut lines, "recommends", package.get("recommends"));
    push_count(&mut lines, "provides", package.get("provides"));
    push_count(&mut lines, "conflicts", package.get("conflicts"));
    push_count(&mut lines, "replaces", package.get("replaces"));
    lines
}

fn recipe_policy_lines(package: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_count(&mut lines, "conffiles", package.get("conffiles"));
    push_presence(&mut lines, "sysusers", package.get("sysusers"));
    push_presence(&mut lines, "tmpfiles", package.get("tmpfiles"));
    push_presence(&mut lines, "alternatives", package.get("alternatives"));
    push_presence(&mut lines, "hooks", package.get("hooks"));
    push_presence(
        &mut lines,
        "provider_assets",
        package.get("provider_assets"),
    );
    push_presence(&mut lines, "flags", package.get("flags_allowed"));
    lines
}

fn validation_lines(validation: &Value) -> Vec<String> {
    let errors = validation
        .get("errors")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let warnings = validation
        .get("warnings")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut lines = vec![format!("errors: {errors}"), format!("warnings: {warnings}")];
    if let Some(issues) = validation.get("issues").and_then(Value::as_array) {
        for issue in issues.iter().take(12) {
            let severity = json_string(issue, &["severity"]).unwrap_or("issue");
            let message = json_string(issue, &["message"]).unwrap_or("");
            lines.push(format!("  {severity}: {message}"));
        }
        if issues.len() > 12 {
            lines.push(format!("  … {} more", issues.len() - 12));
        }
    }
    lines
}

fn push_count(lines: &mut Vec<String>, label: &str, value: Option<&Value>) {
    if let Some(array) = value.and_then(Value::as_array) {
        lines.push(format!("{label}: {}", array.len()));
    }
}

fn push_presence(lines: &mut Vec<String>, label: &str, value: Option<&Value>) {
    if value.is_some_and(|value| !value.is_null()) {
        lines.push(format!("{label}: declared"));
    }
}

pub(super) fn render_extension(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = vec![
        format!(
            "configured: {}",
            details.get("total").and_then(Value::as_u64).unwrap_or(0)
        ),
        format!(
            "enabled: {}",
            details.get("enabled").and_then(Value::as_u64).unwrap_or(0)
        ),
    ];

    if let Some(extensions) = details.get("extensions").and_then(Value::as_array) {
        for extension in extensions {
            let name = json_string(extension, &["name"]).unwrap_or("?");
            let kind = json_string(extension, &["kind"]).unwrap_or("unknown");
            let version = json_string(extension, &["version"]).unwrap_or("unknown");
            let enabled = extension
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            lines.push(format!("  {name} {version} [{kind}] enabled={enabled}"));
        }
    }

    Some(human_framed_report(
        report,
        "Extensions",
        &[("Configured".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_command_stub(report: &CommandReport) -> Option<String> {
    if report.area != "command" {
        return None;
    }
    let implemented = report
        .details
        .as_ref()
        .and_then(|d| d.get("implemented"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut lines = vec![format!("implemented: {implemented}")];
    if let Some(action) = report
        .details
        .as_ref()
        .and_then(|d| d.get("action"))
        .and_then(Value::as_str)
    {
        lines.push(format!("action: {action}"));
    }
    Some(human_framed_report(
        report,
        "Unsupported Command",
        &[("Status".to_owned(), lines)],
        None,
    ))
}
