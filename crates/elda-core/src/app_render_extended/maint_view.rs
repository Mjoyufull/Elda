use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

use super::helpers::push_kv;

pub(super) fn render_maint(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections = Vec::new();

    if let Some(modules) = details.get("modules").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for module in modules {
            let name = json_string(module, &["name"]).unwrap_or("?");
            let status = json_string(module, &["status"]).unwrap_or("?");
            lines.push(format!("{name}: {status}"));
        }
        if !lines.is_empty() {
            sections.push(("Modules".to_owned(), lines));
        }
    }

    if let Some(actions) = details.get("actions").and_then(Value::as_array) {
        let mut lines = Vec::new();
        for action in actions {
            let name = json_string(action, &["module"]).unwrap_or("?");
            let status = json_string(action, &["status"]).unwrap_or("?");
            lines.push(format!("{name}: {status}"));
        }
        if !lines.is_empty() {
            sections.push(("Actions".to_owned(), lines));
        }
    }

    Some(human_framed_report(report, "Maintenance", &sections, None))
}

pub(super) fn render_init(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections = Vec::new();
    let mut summary = Vec::new();
    push_kv(
        &mut summary,
        "config created",
        json_string(details, &["config_created"]),
    );
    push_kv(
        &mut summary,
        "config path",
        json_string(details, &["config_path"]),
    );
    if !summary.is_empty() {
        sections.push(("Summary".to_owned(), summary));
    }

    for (label, key) in [("Created", "created"), ("Existing", "existing")] {
        if let Some(entries) = details.get(key).and_then(Value::as_array) {
            let lines = entries
                .iter()
                .filter_map(|entry| {
                    let path = json_string(entry, &["path"])?;
                    let name = json_string(entry, &["label"]).unwrap_or(key);
                    Some(format!("{name}: {path}"))
                })
                .collect::<Vec<_>>();
            if !lines.is_empty() {
                sections.push((label.to_owned(), lines));
            }
        }
    }

    Some(human_framed_report(report, "Bootstrap", &sections, None))
}
