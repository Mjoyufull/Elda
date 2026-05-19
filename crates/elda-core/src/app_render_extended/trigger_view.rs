use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_string};

use super::helpers::push_kv;

pub(super) fn render_trigger(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if let Some(triggers) = details.get("triggers") {
        return Some(render_trigger_list(report, triggers));
    }
    if details.get("before").is_some() || details.get("repair").is_some() {
        return Some(render_trigger_run(report, details));
    }
    if let Some(changed) = details.get("changed").and_then(Value::as_bool) {
        return Some(render_trigger_diff(report, details, changed));
    }
    details
        .get("trigger")
        .map(|trigger| render_trigger_info(report, trigger))
}

fn render_trigger_run(report: &CommandReport, details: &Value) -> String {
    let name = details
        .get("trigger")
        .and_then(|trigger| json_string(trigger, &["name"]))
        .unwrap_or("?");
    let mut sections = Vec::new();
    if let Some(trigger) = details.get("trigger") {
        sections.push(("After".to_owned(), render_trigger_info_lines(trigger)));
    }
    if let Some(before) = details.get("before") {
        sections.push(("Before".to_owned(), render_trigger_info_lines(before)));
    }
    if let Some(repair) = details.get("repair") {
        let mut lines = Vec::new();
        if let Some(repaired) = repair.get("repaired").and_then(Value::as_array) {
            lines.push(format!("repaired: {}", repaired.len()));
        }
        if let Some(pending) = repair.get("pending").and_then(Value::as_array) {
            lines.push(format!("still pending: {}", pending.len()));
        }
        if !lines.is_empty() {
            sections.push(("Repair".to_owned(), lines));
        }
    }
    human_framed_report(report, format!("Trigger Run: {name}"), &sections, None)
}

fn render_trigger_diff(report: &CommandReport, details: &Value, changed: bool) -> String {
    let name = details
        .get("trigger")
        .and_then(|trigger| json_string(trigger, &["name"]))
        .unwrap_or("?");
    let mut lines = vec![format!("changed: {changed}")];
    if let Some(trigger) = details.get("trigger") {
        lines.extend(render_trigger_info_lines(trigger));
    }
    human_framed_report(
        report,
        format!("Trigger Diff: {name}"),
        &[("Summary".to_owned(), lines)],
        None,
    )
}

fn render_trigger_info_lines(trigger: &Value) -> Vec<String> {
    let mut lines = Vec::new();
    push_kv(&mut lines, "backend", json_string(trigger, &["backend"]));
    lines.push(format!("known: {}", json_bool(trigger, "known")));
    lines.push(format!(
        "pending: {}",
        trigger.get("pending").is_some_and(|value| !value.is_null())
    ));
    lines.push(format!(
        "last run: {}",
        trigger
            .get("last_run")
            .is_some_and(|value| !value.is_null())
    ));
    lines
}

fn render_trigger_list(report: &CommandReport, triggers: &Value) -> String {
    let mut summary = Vec::new();
    push_kv(&mut summary, "backend", json_string(triggers, &["backend"]));
    summary.push(format!(
        "system mode: {}",
        json_bool(triggers, "system_mode")
    ));
    push_count(&mut summary, triggers, "pending");
    push_count(&mut summary, triggers, "last_run");

    let mut sections = vec![("Summary".to_owned(), summary)];
    push_records_section(&mut sections, triggers, "pending", "Pending");
    push_records_section(&mut sections, triggers, "last_run", "Last Run");
    push_boot_section(&mut sections, triggers);

    human_framed_report(report, "System Triggers", &sections, None)
}

fn render_trigger_info(report: &CommandReport, trigger: &Value) -> String {
    let name = json_string(trigger, &["name"]).unwrap_or("?");
    let mut lines = vec![format!("name: {name}")];
    lines.extend(render_trigger_info_lines(trigger));
    lines.push(format!("boot path: {}", json_bool(trigger, "boot_path")));
    lines.push(format!("critical: {}", json_bool(trigger, "critical")));
    push_kv(
        &mut lines,
        "output path",
        json_string(trigger, &["output_path"]),
    );
    lines.push(format!(
        "output present: {}",
        trigger.get("output").is_some_and(|value| !value.is_null())
    ));

    let mut sections = vec![("Trigger".to_owned(), lines)];
    push_optional_record_section(&mut sections, trigger, "pending", "Pending");
    push_optional_record_section(&mut sections, trigger, "last_run", "Last Run");

    human_framed_report(report, format!("Trigger: {name}"), &sections, None)
}

fn push_count(lines: &mut Vec<String>, value: &Value, key: &str) {
    if let Some(records) = value.get(key).and_then(Value::as_array) {
        lines.push(format!("{}: {}", key.replace('_', " "), records.len()));
    }
}

fn push_records_section(
    sections: &mut Vec<(String, Vec<String>)>,
    value: &Value,
    key: &str,
    title: &str,
) {
    if let Some(records) = value.get(key).and_then(Value::as_array)
        && !records.is_empty()
    {
        sections.push((title.to_owned(), trigger_record_rows(records)));
    }
}

fn push_optional_record_section(
    sections: &mut Vec<(String, Vec<String>)>,
    value: &Value,
    key: &str,
    title: &str,
) {
    if let Some(record) = value.get(key).filter(|value| !value.is_null()) {
        sections.push((title.to_owned(), trigger_record_row(record)));
    }
}

fn push_boot_section(sections: &mut Vec<(String, Vec<String>)>, triggers: &Value) {
    let Some(boot) = triggers.get("boot_status") else {
        return;
    };
    let mut boot_lines = Vec::new();
    push_count(&mut boot_lines, boot, "managed_inputs");
    push_count(&mut boot_lines, boot, "pending_triggers");
    if !boot_lines.is_empty() {
        sections.push(("Boot".to_owned(), boot_lines));
    }
}

fn json_bool(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn trigger_record_rows(records: &[Value]) -> Vec<String> {
    records
        .iter()
        .take(32)
        .flat_map(trigger_record_row)
        .collect()
}

fn trigger_record_row(record: &Value) -> Vec<String> {
    let name = json_string(record, &["name"]).unwrap_or("?");
    let mut line = name.to_owned();
    if let Some(reason) = json_string(record, &["reason"]) {
        line.push_str(": ");
        line.push_str(reason);
    }
    if let Some(output_path) = json_string(record, &["output_path"]) {
        line.push_str(" -> ");
        line.push_str(output_path);
    }
    vec![line]
}
