use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{human_framed_report, json_array, json_string};

use super::helpers::push_kv;

pub(super) fn render_profile(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if report.status == "planned" {
        let plan = details.get("plan")?;
        let mut lines = Vec::new();
        push_kv(&mut lines, "kind", json_string(plan, &["kind"]));
        if let Some(prev) = json_array(plan, &["previous_active_profiles"]) {
            lines.push(format!("previous anchors: {}", prev.len()));
        }
        if let Some(next) = json_array(plan, &["next_active_profiles"]) {
            lines.push(format!("next anchors: {}", next.len()));
            for p in next.iter().filter_map(Value::as_str).take(12) {
                lines.push(format!("  - {p}"));
            }
            if next.len() > 12 {
                lines.push(format!("  … {} more", next.len() - 12));
            }
        }
        if let Some(actions) = json_array(plan, &["install_actions"]) {
            lines.push(format!("install actions: {}", actions.len()));
        }
        return Some(human_framed_report(
            report,
            "Profile plan",
            &[("Plan".to_owned(), lines)],
            None,
        ));
    }

    let mut overview = Vec::new();
    if let Some(prev) = details
        .get("previous_active_profiles")
        .and_then(Value::as_array)
    {
        let s = prev
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !s.is_empty() {
            overview.push(format!("previous anchors: {s}"));
        }
    }
    if let Some(next) = details
        .get("next_active_profiles")
        .and_then(Value::as_array)
    {
        let s = next
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !s.is_empty() {
            overview.push(format!("next anchors: {s}"));
        }
    }
    push_kv(
        &mut overview,
        "previous native arch",
        json_string(details, &["previous_native_arch"]),
    );
    push_kv(
        &mut overview,
        "next native arch",
        json_string(details, &["next_native_arch"]),
    );
    if let Some(actions) = details.get("install_actions").and_then(Value::as_array) {
        overview.push(format!("install actions recorded: {}", actions.len()));
    }
    if let Some(removed) = details
        .get("removed_profile_anchors")
        .and_then(Value::as_array)
    {
        overview.push(format!("removed anchors: {}", removed.len()));
    }

    Some(human_framed_report(
        report,
        "Profile",
        &[("Overview".to_owned(), overview)],
        None,
    ))
}

pub(super) fn render_flags(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();

    if let Some(pkg) = json_string(details, &["package"]) {
        let flag_state = details.get("flag_state").unwrap_or(details);
        let mut overview = vec![format!("package: {pkg}")];
        push_kv(
            &mut overview,
            "lane",
            json_string(details, &["selected_lane"]),
        );
        push_kv(
            &mut overview,
            "source kind",
            json_string(details, &["source_kind"]),
        );
        push_kv(
            &mut overview,
            "installed variant",
            json_string(details, &["installed_variant_id"]),
        );
        let resolved_variant = json_string(details, &["resolved_variant_id"])
            .or_else(|| json_string(flag_state, &["variant_id"]));
        push_kv(&mut overview, "resolved variant", resolved_variant);
        if let Some(customized) = flag_state.get("customized").and_then(Value::as_bool) {
            overview.push(format!("customized: {customized}"));
        }
        if let Some(profiles) = json_array(flag_state, &["active_profiles"])
            && !profiles.is_empty()
        {
            let value = profiles
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            if !value.is_empty() {
                overview.push(format!("active profiles: {value}"));
            }
        }
        if let Some(v) = details.get("variant_changed").and_then(Value::as_bool) {
            overview.push(format!("variant changed: {v}"));
        }
        let changes = details
            .get("changes")
            .and_then(Value::as_array)
            .cloned()
            .or_else(|| compute_flag_deltas_from_state(flag_state));
        if let Some(changes) = changes.as_ref() {
            overview.push(format!("flag deltas: {}", changes.len()));
            for change in changes.iter().take(20) {
                let flag = json_string(change, &["flag"]).unwrap_or("?");
                let default = change
                    .get("default")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let effective = change
                    .get("effective")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                overview.push(format!(
                    "  {flag}: {} -> {}",
                    bool_marker(default),
                    bool_marker(effective)
                ));
            }
        }
        sections.push(("Package flags".to_owned(), overview));
        if let Some(layers) = json_array(flag_state, &["package_flag_layers"])
            && !layers.is_empty()
        {
            let mut lines = Vec::new();
            for layer in layers {
                let source = json_string(layer, &["source"]).unwrap_or("?");
                let flags = layer
                    .get("flags")
                    .and_then(Value::as_object)
                    .map(|map| map.len())
                    .unwrap_or(0);
                lines.push(format!("  {source}: {flags} flag(s)"));
            }
            sections.push(("Package overrides".to_owned(), lines));
        }

        if let Some(groups) = json_array(flag_state, &["cardinality_groups"])
            && !groups.is_empty()
        {
            let mut lines = Vec::new();
            for group in groups {
                let kind = json_string(group, &["kind"]).unwrap_or("?");
                let name = json_string(group, &["name"]).unwrap_or("?");
                let members = json_array(group, &["members"])
                    .map(|members| {
                        members
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                let selected = json_array(group, &["selected"])
                    .map(|members| {
                        members
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                lines.push(format!("  {name} ({kind}): {{{members}}} -> [{selected}]"));
            }
            sections.push(("Cardinality groups".to_owned(), lines));
        }

        if let Some(descriptions) = flag_state.get("descriptions").and_then(Value::as_object)
            && !descriptions.is_empty()
        {
            let mut lines = Vec::new();
            for (flag, value) in descriptions.iter().take(20) {
                if let Some(text) = value.as_str() {
                    lines.push(format!("  {flag}: {text}"));
                }
            }
            sections.push(("Flag descriptions".to_owned(), lines));
        }
    } else {
        let mut lines = Vec::new();
        if let Some(dr) = details.get("drift").and_then(Value::as_array) {
            lines.push(format!("packages with variant drift: {}", dr.len()));
            for d in dr.iter().take(12) {
                let pname = json_string(d, &["package"]).unwrap_or("?");
                let installed = json_string(d, &["installed_variant_id"]).unwrap_or("?");
                let resolved = json_string(d, &["resolved_variant_id"]).unwrap_or("?");
                lines.push(format!("  {pname}: {installed} -> {resolved}"));
            }
            if dr.len() > 12 {
                lines.push(format!("  … {} more", dr.len() - 12));
            }
        }
        if let Some(un) = details.get("unresolved").and_then(Value::as_array) {
            lines.push(format!("unresolved targets: {}", un.len()));
        }
        if let Some(ap) = details.get("active_profiles").and_then(Value::as_array) {
            lines.push(format!(
                "active profiles: {}",
                ap.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if details.get("global_flags").is_some() {
            lines.push("global flags: configured".to_owned());
        }
        if details.get("profile_flags").is_some() {
            lines.push("profile flags: configured".to_owned());
        }
        if details.get("package_flags").is_some() {
            lines.push("package flags: configured".to_owned());
        }
        if lines.is_empty() {
            lines.push("(no flag summary extracted)".to_owned());
        }
        sections.push(("Flag layers".to_owned(), lines));
    }

    Some(human_framed_report(report, "Flags", &sections, None))
}

fn bool_marker(value: bool) -> &'static str {
    if value { "+" } else { "-" }
}

fn compute_flag_deltas_from_state(flag_state: &Value) -> Option<Vec<Value>> {
    let defaults = flag_state.get("default_flags")?.as_object()?;
    let effective = flag_state.get("effective_flags")?.as_object()?;
    let mut changes = Vec::new();
    for (flag, eff) in effective {
        let eff_bool = eff.as_bool().unwrap_or(false);
        let def_bool = defaults.get(flag).and_then(Value::as_bool).unwrap_or(false);
        if eff_bool == def_bool {
            continue;
        }
        changes.push(serde_json::json!({
            "flag": flag,
            "default": def_bool,
            "effective": eff_bool,
        }));
    }
    if changes.is_empty() {
        None
    } else {
        Some(changes)
    }
}

pub(super) fn render_policy(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let mut lines = Vec::new();

    push_kv(&mut lines, "package", json_string(details, &["package"]));
    push_kv(&mut lines, "version", json_string(details, &["version"]));

    if let Some(rd) = details
        .get("reverse_dependencies")
        .and_then(Value::as_array)
    {
        lines.push(format!("reverse dependencies: {}", rd.len()));
        for r in rd.iter().take(20) {
            let pname = json_string(r, &["pkgname"]).unwrap_or("?");
            let kind = json_string(r, &["dependency_kind"]).unwrap_or("");
            let weak = r.get("is_weak").and_then(Value::as_bool).unwrap_or(false);
            lines.push(format!("  {pname} ({kind}, weak={weak})"));
        }
        if rd.len() > 20 {
            lines.push(format!("  … {} more", rd.len() - 20));
        }
    }

    if let Some(pv) = details.get("pinned_version") {
        if pv.is_null() {
            lines.push("pinned version: (cleared)".to_owned());
        } else if let Some(s) = pv.as_str() {
            lines.push(format!("pinned version: {s}"));
        }
    }

    if let Some(h) = details.get("held").and_then(Value::as_bool) {
        lines.push(format!("held: {h}"));
        push_kv(
            &mut lines,
            "hold source",
            json_string(details, &["hold_source"]),
        );
    }

    if let Some(rec) = details.get("recursive").and_then(Value::as_bool) {
        lines.push(format!("recursive: {rec}"));
    }
    if let Some(iw) = details.get("include_weak").and_then(Value::as_bool) {
        lines.push(format!("include weak: {iw}"));
    }

    Some(human_framed_report(
        report,
        "Policy",
        &[("Details".to_owned(), lines)],
        None,
    ))
}
