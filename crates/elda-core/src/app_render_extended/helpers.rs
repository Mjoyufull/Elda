use serde_json::Value;

use crate::app_render_support::{json_string, json_u64};

pub(super) fn push_kv(lines: &mut Vec<String>, key: &str, val: Option<&str>) {
    if let Some(val) = val
        && !val.is_empty()
    {
        lines.push(format!("{key}: {val}"));
    }
}

pub(super) fn recipe_package_lines(out: &mut Vec<String>, p: &Value) {
    push_kv(out, "name", json_string(p, &["name"]));
    push_kv(out, "version", json_string(p, &["version"]));
    if let (Some(e), Some(v), Some(r)) = (
        json_u64(p, &["epoch"]),
        json_string(p, &["version"]),
        json_u64(p, &["rel"]),
    ) {
        out.push(format!("epoch:rel: {e}:{v}-{r}"));
    }
    push_kv(out, "description", json_string(p, &["description"]));
    if let Some(lic) = p.get("licenses").and_then(Value::as_array) {
        let s = lic
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        if !s.is_empty() {
            out.push(format!("licenses: {s}"));
        }
    }
    push_kv(
        out,
        "upstream",
        json_string(p, &["upstream"]).map(|u| u.split('\n').next().unwrap_or(u)),
    );
}

pub(super) fn installed_record_lines(inst: &Value) -> Vec<String> {
    let mut v = Vec::new();
    push_kv(&mut v, "pkgname", json_string(inst, &["pkgname"]));
    push_kv(&mut v, "version", json_string(inst, &["version"]));
    push_kv(&mut v, "source_kind", json_string(inst, &["source_kind"]));
    push_kv(
        &mut v,
        "install_reason",
        json_string(inst, &["install_reason"]),
    );
    push_kv(&mut v, "variant_id", json_string(inst, &["variant_id"]));
    push_kv(&mut v, "remote_name", json_string(inst, &["remote_name"]));
    push_kv(&mut v, "source_ref", json_string(inst, &["source_ref"]));
    v
}

pub(super) fn synced_record_lines(s: &Value) -> Vec<String> {
    let mut v = Vec::new();
    push_kv(&mut v, "remote", json_string(s, &["remote_name"]));
    push_kv(&mut v, "channel", json_string(s, &["channel"]));
    if let (Some(e), Some(pv), Some(pr)) = (
        json_u64(s, &["epoch"]),
        json_string(s, &["pkgver"]),
        json_u64(s, &["pkgrel"]),
    ) {
        v.push(format!("version: {e}:{pv}-{pr}"));
    }
    push_kv(&mut v, "summary", json_string(s, &["summary"]));
    v
}

pub(super) fn files_summary(files: &Value) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(n) = json_u64(files, &["total_paths"]) {
        v.push(format!("total paths: {n}"));
    }
    if let Some(n) = json_u64(files, &["regular_files"]) {
        v.push(format!("files: {n}"));
    }
    if let Some(n) = json_u64(files, &["directories"]) {
        v.push(format!("directories: {n}"));
    }
    if let Some(n) = json_u64(files, &["symlinks"]) {
        v.push(format!("symlinks: {n}"));
    }
    if let Some(n) = json_u64(files, &["conffiles"]) {
        v.push(format!("conffiles: {n}"));
    }
    v
}
