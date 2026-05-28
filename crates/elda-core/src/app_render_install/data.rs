use serde_json::Value;

use crate::app_render_support::{json_string, json_u64};

pub(crate) fn primary_action(actions: &[Value]) -> Option<&Value> {
    actions
        .iter()
        .find(|action| json_string(action, &["install_reason"]) == Some("explicit"))
        .or_else(|| actions.first())
}

pub(crate) fn provenance_badge(action: &Value) -> &'static str {
    if let Some(kind) = json_string(action, &["selected_source_kind"])
        && matches!(
            kind,
            "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template"
        )
    {
        return "[I]";
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
