use serde_json::Value;

use crate::app_render_support::{json_string, json_u64};

pub(crate) fn primary_action(actions: &[Value]) -> Option<&Value> {
    actions
        .iter()
        .find(|action| json_string(action, &["install_reason"]) == Some("explicit"))
        .or_else(|| actions.first())
}

pub(crate) fn provenance_badge(action: &Value) -> &'static str {
    if let Some(kind) = json_string(action, &["selected_source_kind"]) {
        if matches!(
            kind,
            "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template"
        ) {
            return "[I]";
        }
    }

    match json_string(action, &["persisted_source_kind"])
        .or_else(|| json_string(action, &["package", "source_kind"]))
        .or_else(|| json_string(action, &["selected_source_kind"]))
        .unwrap_or("unknown")
    {
        "repo_binary" | "local_recipe" => "[E]",
        "interbuild" => "[I]",
        "adopted" => "[A]",
        "git" if ad_hoc_vendor_source(action) => "[V]",
        "git" => "[E]",
        _ => "[?]",
    }
}

fn ad_hoc_vendor_source(action: &Value) -> bool {
    json_string(action, &["generated_metadata_path"]).is_some()
        || action.get("ad_hoc_git_moving").is_some()
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

/// Semantic version tone for plan gates: green upgrade/install, blue keep/reinstall, red downgrade.
pub(crate) fn version_change_rgb(action: &Value) -> Option<(u8, u8, u8)> {
    use crate::render_style::palette;
    match json_string(action, &["action"]).unwrap_or("install") {
        "downgrade-explicit" | "downgrade-dependency" | "source-ref-downgrade" => {
            Some(palette::WARNING) // yellow-orange for downgrade per contract
        }
        "keep-installed" | "keep" => Some(palette::VERSION),
        "upgrade-explicit" | "upgrade-dependency" | "install-replacing" => Some(palette::SUCCESS),
        "install-explicit" | "install-dependency" | "install-recommended" => Some(palette::SUCCESS),
        _ if action
            .get("needs_change")
            .and_then(Value::as_bool)
            .is_some_and(|needs| !needs) =>
        {
            Some(palette::VERSION)
        }
        _ => None,
    }
}

pub(crate) fn compact_plan_interbuild_summary(action: &Value) -> Option<String> {
    let details = action.get("interbuild")?;
    if details.is_null() {
        return None;
    }
    let parser = json_string(details, &["parser"]).unwrap_or("unknown");
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
    Some(format!("parser {parser}, {external_cli}"))
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
