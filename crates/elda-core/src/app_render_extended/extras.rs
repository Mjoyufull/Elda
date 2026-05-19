use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

use super::helpers::{push_kv, recipe_package_lines};

pub(super) fn render_qa(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if report.status == "planned" {
        let plan = details.get("plan")?;
        let mut lines = Vec::new();
        if let Some(t) = plan.get("requested_targets").and_then(Value::as_array) {
            lines.push(format!("requested targets: {}", t.len()));
        }
        if let Some(pkgs) = plan.get("packages").and_then(Value::as_array) {
            lines.push(format!("packages: {}", pkgs.len()));
            for p in pkgs.iter().take(12) {
                let pname = json_string(p, &["pkgname"]).unwrap_or("?");
                lines.push(format!("  {pname}"));
            }
            if pkgs.len() > 12 {
                lines.push(format!("  … {} more", pkgs.len() - 12));
            }
        }
        return Some(human_framed_report(
            report,
            "QA plan",
            &[("Plan".to_owned(), lines)],
            None,
        ));
    }

    if let Some(lint) = details.get("lint") {
        let mut lines = Vec::new();
        if let Some(recipes) = lint.get("recipes").and_then(Value::as_array) {
            lines.push(format!("recipes checked: {}", recipes.len()));
        }
        if let Some(issues) = lint.get("issues").and_then(Value::as_array) {
            lines.push(format!("issues: {}", issues.len()));
            for issue in issues.iter().take(16) {
                let recipe = json_string(issue, &["recipe"]).unwrap_or("?");
                let msg = json_string(issue, &["message"]).unwrap_or("");
                lines.push(format!("  {recipe}: {msg}"));
            }
            if issues.len() > 16 {
                lines.push(format!("  … {} more", issues.len() - 16));
            }
        }
        return Some(human_framed_report(
            report,
            "QA lint",
            &[("Lint".to_owned(), lines)],
            None,
        ));
    }

    if let Some(builds) = details.get("builds").and_then(Value::as_array) {
        let mut lines = vec![format!("builds: {}", builds.len())];
        for b in builds.iter().take(16) {
            let pname = json_string(b, &["pkgname"]).unwrap_or("?");
            lines.push(format!("  {pname}"));
        }
        return Some(human_framed_report(
            report,
            "QA build",
            &[("Builds".to_owned(), lines)],
            None,
        ));
    }

    Some(human_framed_report(
        report,
        "QA",
        &[(
            "Summary".to_owned(),
            vec!["see JSON output for full QA details".to_owned()],
        )],
        None,
    ))
}

pub(super) fn render_forge(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if let Some(results) = details.get("results").and_then(Value::as_array) {
        let mut lines = Vec::new();
        push_kv(&mut lines, "query", json_string(details, &["query"]));
        lines.push(format!("matches: {}", results.len()));
        for r in results.iter().take(16) {
            let pname = json_string(r, &["pkgname"]).unwrap_or("?");
            let src = json_string(r, &["source"]).unwrap_or("");
            lines.push(format!("  {pname} [{src}]"));
        }
        if results.len() > 16 {
            lines.push(format!("  … {} more", results.len() - 16));
        }
        return Some(human_framed_report(
            report,
            "Forge search",
            &[("Results".to_owned(), lines)],
            None,
        ));
    }

    if let Some(pkg) = details.get("package") {
        let mut lines = Vec::new();
        if let Some(name) = json_string(pkg, &["package"]) {
            lines.push(format!("package: {name}"));
        }
        push_kv(
            &mut lines,
            "local recipe",
            pkg.get("local_recipe_path").and_then(Value::as_str),
        );
        if pkg.get("published").is_some_and(|p| !p.is_null()) {
            lines.push("published metadata: present".to_owned());
        }
        return Some(human_framed_report(
            report,
            "Forge browse",
            &[("Package".to_owned(), lines)],
            None,
        ));
    }

    None
}

pub(super) fn render_vendor(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(v) = details.get("vendor") {
        let mut lines = Vec::new();
        push_kv(&mut lines, "package", json_string(v, &["package_name"]));
        sections.push(("Vendor add".to_owned(), lines));
    }

    if let Some(imported) = details.get("import")
        && let Some(pkgs) = imported.get("packages").and_then(Value::as_array)
    {
        let mut lines = vec![format!("imported packages: {}", pkgs.len())];
        for p in pkgs.iter().take(16) {
            if let Some(name) =
                json_string(p, &["package_name"]).or_else(|| json_string(p, &["pkgname"]))
            {
                lines.push(format!("  {name}"));
            }
        }
        sections.push(("Vendor import".to_owned(), lines));
    }

    if let Some(exp) = details.get("export")
        && let Some(pkgs) = exp.get("packages").and_then(Value::as_array)
    {
        let mut lines = vec![format!("exported packages: {}", pkgs.len())];
        for p in pkgs.iter().take(24).filter_map(Value::as_str) {
            lines.push(format!("  {p}"));
        }
        if pkgs.len() > 24 {
            lines.push(format!("  … {} more", pkgs.len() - 24));
        }
        sections.push(("Vendor export".to_owned(), lines));
    }

    if sections.is_empty() {
        return None;
    }

    Some(human_framed_report(report, "Vendor", &sections, None))
}

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
