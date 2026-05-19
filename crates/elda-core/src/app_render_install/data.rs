use serde_json::Value;

use crate::app_render_support::{json_array, json_string, json_u64};

pub(crate) fn primary_action(actions: &[Value]) -> Option<&Value> {
    actions
        .iter()
        .find(|action| json_string(action, &["install_reason"]) == Some("explicit"))
        .or_else(|| actions.first())
}

pub(crate) fn report_actions(details: &Value) -> &[Value] {
    json_array(details, &["plan", "actions"])
        .or_else(|| json_array(details, &["installs"]))
        .unwrap_or_default()
}

pub(crate) fn provenance_badge(action: &Value) -> &'static str {
    match json_string(action, &["persisted_source_kind"])
        .or_else(|| json_string(action, &["package", "source_kind"]))
        .or_else(|| json_string(action, &["selected_source_kind"]))
        .unwrap_or("unknown")
    {
        "repo_binary" => "[E]",
        "interbuild" | "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template" => "[I]",
        "adopted" => "[A]",
        "local_recipe" | "git" if json_string(action, &["source_ref"]).is_some() => "[V]",
        "local_recipe" | "git" => "[E]",
        _ => "[?]",
    }
}

pub(crate) fn confidence_modifier(action: &Value) -> &'static str {
    match provenance_badge(action) {
        "[E]" => "native",
        "[I]" => "parsed",
        "[F]" => "translated",
        "[A]" => "adopted",
        "[V]" => "vendor/ad-hoc",
        _ => "unknown",
    }
}

pub(crate) fn binary_trust_summary(action: &Value) -> Option<&'static str> {
    let verification = action.get("binary_source_verification")?;
    if !verification.is_object() {
        return None;
    }
    if verification
        .get("payload_signature")
        .is_some_and(|value| !value.is_null())
    {
        return Some("verified payload signature");
    }
    Some("verified remote payload")
}

pub(crate) fn render_snapshot_summary(action: &Value) -> Option<String> {
    let snapshots = action.get("install")?.get("snapshots")?.as_array()?;
    if snapshots.is_empty() {
        return None;
    }

    let tool = snapshots
        .first()
        .and_then(|snapshot| snapshot.get("tool"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let captured = snapshots
        .iter()
        .filter(|snapshot| json_string(snapshot, &["status"]) == Some("captured"))
        .count();
    let failed = snapshots
        .iter()
        .filter(|snapshot| json_string(snapshot, &["status"]) == Some("failed"))
        .count();

    let mut summary = format!("{} via {tool}", snapshots.len());
    if captured > 0 {
        summary.push_str(&format!(", {captured} captured"));
    }
    if failed > 0 {
        summary.push_str(&format!(", {failed} failed"));
    }

    Some(summary)
}

pub(crate) fn snapshot_risk_lines(actions: &[Value]) -> Vec<String> {
    let snapshots = actions
        .iter()
        .filter_map(render_snapshot_summary)
        .collect::<Vec<_>>();
    if snapshots.is_empty() {
        return vec!["snapshots: none recorded in current report".to_owned()];
    }

    snapshots
        .into_iter()
        .map(|summary| format!("snapshots: {summary}"))
        .collect()
}

pub(crate) fn action_package_name(action: &Value) -> String {
    json_string(action, &["package"])
        .or_else(|| json_string(action, &["package", "package_name"]))
        .or_else(|| json_string(action, &["target"]))
        .unwrap_or("unknown")
        .to_owned()
}

pub(crate) fn action_version(action: &Value) -> String {
    if let Some(version) =
        json_string(action, &["version"]).or_else(|| json_string(action, &["candidate_version"]))
    {
        return version.to_owned();
    }

    let Some(epoch) = json_u64(action, &["package", "epoch"]) else {
        return "unknown".to_owned();
    };
    let Some(pkgver) = json_string(action, &["package", "pkgver"]) else {
        return "unknown".to_owned();
    };
    let Some(pkgrel) = json_u64(action, &["package", "pkgrel"]) else {
        return "unknown".to_owned();
    };

    format!("{epoch}:{pkgver}-{pkgrel}")
}

pub(crate) fn action_activation_backend(action: &Value) -> Option<&str> {
    json_string(action, &["activation_backend"])
}

pub(crate) fn is_weak(action: &Value) -> bool {
    action
        .get("is_weak")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn weak_suffix(action: &Value) -> &'static str {
    if is_weak(action) { " weak" } else { "" }
}

pub(crate) fn interbuild_summary(action: &Value) -> Option<String> {
    let details = action.get("interbuild")?;
    if details.is_null() {
        return None;
    }

    let parser = json_string(details, &["parser"]).unwrap_or("unknown");
    let engine = json_string(details, &["engine"]).unwrap_or("unknown");
    let confidence = json_string(details, &["confidence"]).unwrap_or("unknown");
    let external_cli = details
        .get("external_cli_required")
        .and_then(Value::as_bool)
        .map(|required| {
            if required {
                "requires external CLI"
            } else {
                "no external CLI"
            }
        })
        .unwrap_or("external CLI unknown");

    let target = json_string(details, &["target"])
        .map(|value| format!(", target {value}"))
        .unwrap_or_default();
    let lockfile = lockfile_summary(details)
        .map(|value| format!(", lockfile {value}"))
        .unwrap_or_default();
    let ecosystem = ecosystem_summary(details)
        .map(|value| format!(", {value}"))
        .unwrap_or_default();

    Some(format!(
        "parser {parser}, engine {engine}, confidence {confidence}, {external_cli}{target}{lockfile}{ecosystem}"
    ))
}

fn lockfile_summary(details: &Value) -> Option<String> {
    let lockfile = details.get("lockfile")?;
    if lockfile.is_null() {
        return None;
    }
    let present = lockfile.get("present").and_then(Value::as_bool)?;
    let locked_inputs = lockfile
        .get("locked_inputs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(if present {
        format!("present, {locked_inputs} locked input(s)")
    } else {
        "absent".to_owned()
    })
}

fn ecosystem_summary(details: &Value) -> Option<String> {
    gentoo_summary(details)
        .or_else(|| aur_summary(details))
        .or_else(|| xbps_summary(details))
}

fn gentoo_summary(details: &Value) -> Option<String> {
    let gentoo = details.get("gentoo")?;
    if gentoo.is_null() {
        return None;
    }
    let eapi = json_string(gentoo, &["eapi"]).unwrap_or("unknown");
    let depend = gentoo
        .get("depend")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let rdepend = gentoo
        .get("rdepend")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let iuse = gentoo
        .get("iuse")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let phases = array_len(gentoo, "phases");
    let commands = phase_command_count(gentoo);
    Some(format!(
        "gentoo EAPI {eapi}, {depend} DEPEND, {rdepend} RDEPEND, {iuse} IUSE, {phases} phase(s), {commands} command(s)"
    ))
}

fn aur_summary(details: &Value) -> Option<String> {
    let aur = details.get("aur")?;
    if aur.is_null() {
        return None;
    }
    let depends = array_len(aur, "depends");
    let makedepends = array_len(aur, "makedepends");
    let optdepends = array_len(aur, "optdepends");
    let functions = array_len(aur, "functions");
    let arch_sources = array_len(aur, "arch_sources");
    let vcs_sources = array_len(aur, "vcs_sources");
    let pkgver = if aur
        .get("pkgver_function")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "pkgver() present"
    } else {
        "static pkgver"
    };
    let commands = phase_command_count(aur);
    Some(format!(
        "aur {depends} depend(s), {makedepends} makedepend(s), {optdepends} optdepend(s), {functions} function(s), {arch_sources} arch source set(s), {vcs_sources} VCS source(s), {pkgver}, {commands} command(s)"
    ))
}

fn xbps_summary(details: &Value) -> Option<String> {
    let xbps = details.get("xbps")?;
    if xbps.is_null() {
        return None;
    }
    let depends = array_len(xbps, "depends");
    let makedepends = array_len(xbps, "makedepends");
    let hostmakedepends = array_len(xbps, "hostmakedepends");
    let functions = array_len(xbps, "functions");
    let commands = phase_command_count(xbps);
    Some(format!(
        "xbps {depends} depend(s), {makedepends} makedepend(s), {hostmakedepends} hostmakedepend(s), {functions} function(s), {commands} command(s)"
    ))
}

fn phase_command_count(value: &Value) -> usize {
    value
        .get("phase_commands")
        .and_then(Value::as_array)
        .map(|phases| {
            phases
                .iter()
                .map(|phase| array_len(phase, "commands"))
                .sum()
        })
        .unwrap_or(0)
}

fn array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

pub(crate) fn object_summary(action: &Value) -> Option<String> {
    let object = action.get("package")?.get("object_metadata")?;
    let requires = object
        .get("shlib_requires")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let provides = object
        .get("shlib_provides")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    Some(format!(
        "{requires} shlib require(s), {provides} shlib provide(s)"
    ))
}

pub(crate) fn push_optional_line(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{key}: {value}"));
    }
}
