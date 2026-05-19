use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string, json_u64};

use super::helpers::{
    files_summary, installed_record_lines, push_kv, recipe_package_lines, synced_record_lines,
};

pub(super) fn render_verify(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();
    if let Some(vr) = details.get("verify_report") {
        if let Some(pkgs) = vr.get("packages").and_then(Value::as_array) {
            lines.push(format!("packages: {}", pkgs.len()));
        }
        if let Some(n) = json_u64(vr, &["checked_paths"]) {
            lines.push(format!("paths checked: {n}"));
        }
        if let Some(issues) = vr.get("issues").and_then(Value::as_array) {
            lines.push(format!("issues: {}", issues.len()));
            for issue in issues.iter().take(24) {
                let pkg = json_string(issue, &["package"]).unwrap_or("?");
                let path = json_string(issue, &["path"]).unwrap_or("?");
                let kind = json_string(issue, &["kind"]).unwrap_or("?");
                let detail = json_string(issue, &["detail"]).unwrap_or("");
                lines.push(format!("  {pkg} {kind}: {path} — {detail}"));
            }
            if issues.len() > 24 {
                lines.push(format!("  … {} more", issues.len() - 24));
            }
        }
    }
    Some(human_framed_report(
        report,
        "Verify",
        &[("Summary".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_diff(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let pkg = json_string(details, &["package"]).unwrap_or("?");
    let mut lines = vec![format!("package: {pkg}")];
    push_kv(
        &mut lines,
        "installed",
        json_string(details, &["installed_version"]),
    );
    if let Some(cand) = details.get("candidate") {
        push_kv(
            &mut lines,
            "candidate version",
            json_string(cand, &["version"]),
        );
        push_kv(&mut lines, "lane", json_string(cand, &["selected_lane"]));
    }
    if let Some(changes) = details.get("changes").and_then(Value::as_array) {
        lines.push(format!("changes: {}", changes.len()));
        for ch in changes.iter().take(32) {
            let path = json_string(ch, &["path"]).unwrap_or("?");
            let change = json_string(ch, &["change"]).unwrap_or("?");
            let detail = json_string(ch, &["detail"]).unwrap_or("");
            if detail.is_empty() {
                lines.push(format!("  {change}: {path}"));
            } else {
                lines.push(format!("  {change}: {path} ({detail})"));
            }
        }
        if changes.len() > 32 {
            lines.push(format!("  … {} more", changes.len() - 32));
        }
    }
    Some(human_framed_report(
        report,
        format!("Diff: {pkg}"),
        &[("Compare".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_check(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();
    if let Some(h) = details.get("health")
        && let Some(issues) = h.get("issues").and_then(Value::as_array)
    {
        lines.push(format!("health issues: {}", issues.len()));
        for issue in issues.iter().filter_map(Value::as_str).take(28) {
            lines.push(format!("  - {issue}"));
        }
        if issues.len() > 28 {
            lines.push(format!("  … {} more", issues.len() - 28));
        }
    }
    if let Some(pt) = details.get("pending_triggers").and_then(Value::as_array) {
        lines.push(format!("pending triggers: {}", pt.len()));
    }
    if let Some(b) = details.get("backend") {
        push_kv(&mut lines, "backend kind", json_string(b, &["kind"]));
        push_kv(&mut lines, "backend mode", json_string(b, &["mode"]));
    }
    Some(human_framed_report(
        report,
        "System Check",
        &[("Health".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_doctor(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    let mut summary = Vec::new();
    push_kv(&mut summary, "mode", json_string(details, &["mode"]));
    push_kv(&mut summary, "root", json_string(details, &["root"]));
    push_kv(&mut summary, "prefix", json_string(details, &["prefix"]));
    if let Some(counts) = details.get("counts") {
        if let Some(value) = counts.get("installed_packages").and_then(Value::as_u64) {
            summary.push(format!("installed packages: {value}"));
        }
        if let Some(value) = counts.get("configured_remotes").and_then(Value::as_u64) {
            summary.push(format!("configured remotes: {value}"));
        }
        if let Some(value) = counts.get("pending_triggers").and_then(Value::as_u64) {
            summary.push(format!("pending triggers: {value}"));
        }
    }
    sections.push(("Summary".to_owned(), summary));

    if let Some(blockers) = details.get("blockers").and_then(Value::as_array) {
        let mut lines = vec![format!("blockers: {}", blockers.len())];
        for blocker in blockers.iter().filter_map(Value::as_str).take(24) {
            lines.push(format!("  - {blocker}"));
        }
        sections.push(("Blockers".to_owned(), lines));
    }

    if let Some(advisories) = details.get("advisories").and_then(Value::as_array)
        && !advisories.is_empty()
    {
        let mut lines = Vec::new();
        for advisory in advisories.iter().filter_map(Value::as_str).take(24) {
            lines.push(format!("  - {advisory}"));
        }
        sections.push(("Advisories".to_owned(), lines));
    }

    if let Some(paths) = details.get("paths").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for path in paths.iter().take(24) {
            let label = json_string(path, &["label"]).unwrap_or("path");
            let exists = path
                .get("exists")
                .and_then(Value::as_bool)
                .map(|value| if value { "ok" } else { "missing" })
                .unwrap_or("unknown");
            let location = json_string(path, &["path"]).unwrap_or("");
            lines.push(format!("{label}: {exists} {location}"));
        }
        sections.push(("Paths".to_owned(), lines));
    }

    if let Some(readiness) = details.get("release_readiness") {
        let mut lines = Vec::new();
        for key in [
            "unsupported_commands_fail_closed",
            "check_and_verify_fail_on_issues",
            "dry_run_preflight_visible",
        ] {
            if let Some(value) = readiness.get(key).and_then(Value::as_bool) {
                lines.push(format!("{key}: {value}"));
            }
        }
        sections.push(("Release Readiness".to_owned(), lines));
    }

    Some(human_framed_report(report, "Doctor", &sections, None))
}

pub(super) fn render_info(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let pkg = json_string(details, &["package"]).unwrap_or("?");
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    let overview = vec![format!("package: {pkg}")];
    sections.push(("Overview".to_owned(), overview));

    if let Some(recipe_info) = details.get("recipe").filter(|v| !v.is_null())
        && let Some(p) = recipe_info.get("package")
    {
        let mut rp = Vec::new();
        if let Some(src) = json_string(recipe_info, &["source"]) {
            rp.push(format!("source: {src}"));
        }
        recipe_package_lines(&mut rp, p);
        if !rp.is_empty() {
            sections.push(("Recipe".to_owned(), rp));
        }
    }

    if let Some(inst) = details.get("installed").filter(|v| !v.is_null()) {
        sections.push(("Installed".to_owned(), installed_record_lines(inst)));
    }

    if let Some(synced) = details.get("synced").filter(|v| !v.is_null()) {
        sections.push(("Synced".to_owned(), synced_record_lines(synced)));
    }

    if let Some(fs) = details
        .get("installed_files_summary")
        .filter(|v| !v.is_null())
    {
        let fl = files_summary(fs);
        if !fl.is_empty() {
            sections.push(("Installed files".to_owned(), fl));
        }
    }

    if let Some(pav) = details
        .get("provider_asset_visibility")
        .and_then(Value::as_object)
    {
        sections.push((
            "Providers".to_owned(),
            vec![format!("visibility entries: {}", pav.len())],
        ));
    }

    Some(human_framed_report(
        report,
        format!("Info: {pkg}"),
        &sections,
        None,
    ))
}

pub(super) fn render_publish_ready(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let ready = details.get("publish_ready")?;
    let package = json_string(ready, &["package"]).unwrap_or("?");
    let is_ready = ready.get("ready").and_then(Value::as_bool).unwrap_or(false);
    let mut sections = vec![(
        "Result".to_owned(),
        vec![format!("package: {package}"), format!("ready: {is_ready}")],
    )];
    if let Some(blockers) = ready.get("blockers").and_then(Value::as_array) {
        sections.push(("Blockers".to_owned(), string_rows(blockers)));
    }
    if let Some(warnings) = ready.get("warnings").and_then(Value::as_array)
        && !warnings.is_empty()
    {
        sections.push(("Warnings".to_owned(), string_rows(warnings)));
    }
    Some(human_framed_report(
        report,
        format!("Publish Readiness: {package}"),
        &sections,
        None,
    ))
}

fn string_rows(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn render_recipe_check(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let check = details.get("check")?;
    let strict = details
        .get("strict")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut lines = Vec::new();
    if strict {
        lines.push("mode: strict (warnings fail)".to_owned());
    }
    if let Some(recipes) = check.get("recipes").and_then(Value::as_array) {
        lines.push(format!("recipes: {}", recipes.len()));
    }
    if let Some(issues) = check.get("issues").and_then(Value::as_array) {
        let mut errors = 0_usize;
        let mut warnings = 0_usize;
        for issue in issues {
            match json_string(issue, &["severity"]) {
                Some("error") => errors += 1,
                Some("warning") => warnings += 1,
                _ => {}
            }
        }
        lines.push(format!("errors: {errors}"));
        lines.push(format!("warnings: {warnings}"));
        for issue in issues.iter().take(24) {
            let recipe = json_string(issue, &["recipe"]).unwrap_or("?");
            let severity = json_string(issue, &["severity"]).unwrap_or("?");
            let message = json_string(issue, &["message"]).unwrap_or("");
            lines.push(format!("  {recipe} [{severity}]: {message}"));
        }
        if issues.len() > 24 {
            lines.push(format!("  … {} more", issues.len() - 24));
        }
    }
    Some(human_framed_report(
        report,
        "Recipe Check",
        &[("Summary".to_owned(), lines)],
        None,
    ))
}

pub(super) fn render_recipe_diff(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let diff = details.get("diff")?;
    let package = json_string(diff, &["package"]).unwrap_or("?");
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    sections.push((
        "Inputs".to_owned(),
        vec![
            format!("package: {package}"),
            format!("local: {}", source_state(diff.get("local"))),
            format!("synced: {}", source_state(diff.get("synced"))),
        ],
    ));

    if let Some(changes) = diff.get("changes").and_then(Value::as_array) {
        let changed_count = changes
            .iter()
            .filter(|change| {
                change
                    .get("changed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
            .count();
        let mut lines = vec![format!("changed fields: {changed_count}/{}", changes.len())];
        for change in changes.iter().take(20) {
            let field = json_string(change, &["field"]).unwrap_or("?");
            let local = json_string(change, &["local"]).unwrap_or("<none>");
            let synced = json_string(change, &["synced"]).unwrap_or("<none>");
            let mark = if change
                .get("changed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                "changed"
            } else {
                "same"
            };
            lines.push(format!("{field}: {mark} (local={local}, synced={synced})"));
        }
        sections.push(("Fields".to_owned(), lines));
    }

    Some(human_framed_report(
        report,
        format!("Recipe Diff: {package}"),
        &sections,
        None,
    ))
}

fn source_state(value: Option<&Value>) -> &'static str {
    match value {
        Some(value) if !value.is_null() => "present",
        _ => "missing",
    }
}
