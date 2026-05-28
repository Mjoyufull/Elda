use std::collections::BTreeMap;

use crate::model::{LuaValue, PackageDefinition, ValidationIssue};

use super::error;

pub(super) fn validate_metadata(package: &PackageDefinition, issues: &mut Vec<ValidationIssue>) {
    validate_inline_or_file("sysusers", package.sysusers.as_ref(), issues);
    validate_inline_or_file("tmpfiles", package.tmpfiles.as_ref(), issues);
    validate_alternatives(package.alternatives.as_ref(), issues);
    validate_hooks(package.hooks.as_ref(), issues);
    validate_provider_assets(package.provider_assets.as_ref(), issues);
    validate_flag_table("flags_default", package.flags_default.as_ref(), issues);
    validate_flag_table("flags_allowed", package.flags_allowed.as_ref(), issues);
    validate_flag_table("flags_implies", package.flags_implies.as_ref(), issues);
    validate_flag_table("flags_conflicts", package.flags_conflicts.as_ref(), issues);
    validate_flag_descriptions(package.flags_descriptions.as_ref(), issues);
    validate_flag_cardinality(
        "flags_required_one_of",
        package.flags_required_one_of.as_ref(),
        issues,
    );
    validate_flag_cardinality(
        "flags_required_at_most_one",
        package.flags_required_at_most_one.as_ref(),
        issues,
    );
    validate_flag_cardinality(
        "flags_required_any_of",
        package.flags_required_any_of.as_ref(),
        issues,
    );
    validate_subpackages(package.subpackages.as_ref(), issues);
}

fn validate_flag_descriptions(value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let Some(value) = value else {
        return;
    };
    let table = match value {
        LuaValue::Table(table) => table,
        LuaValue::Array(arr) if arr.is_empty() => return,
        _ => {
            issues.push(error(
                "flags_descriptions must be a table keyed by flag name",
            ));
            return;
        }
    };
    for (flag, entry) in table {
        if !matches!(entry, LuaValue::String(text) if !text.trim().is_empty()) {
            issues.push(error(format!(
                "flags_descriptions.{flag} must be a non-empty string description"
            )));
        }
    }
}

fn validate_flag_cardinality(
    field: &str,
    value: Option<&LuaValue>,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(value) = value else {
        return;
    };
    let table = match value {
        LuaValue::Table(table) => table,
        LuaValue::Array(arr) if arr.is_empty() => return,
        _ => {
            issues.push(error(format!(
                "{field} must be a table keyed by group name with arrays of flag names"
            )));
            return;
        }
    };
    for (group, entry) in table {
        let LuaValue::Array(entries) = entry else {
            issues.push(error(format!(
                "{field}.{group} must be an array of non-empty flag names"
            )));
            continue;
        };
        if entries.len() < 2 {
            issues.push(error(format!(
                "{field}.{group} must contain at least two flag names"
            )));
        }
        for member in entries {
            if !matches!(member, LuaValue::String(value) if !value.trim().is_empty()) {
                issues.push(error(format!(
                    "{field}.{group} must contain only non-empty string flag names"
                )));
            }
        }
    }
}

fn validate_inline_or_file(
    field: &str,
    value: Option<&LuaValue>,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(value) = value else {
        return;
    };

    match value {
        LuaValue::Array(_) => {}
        LuaValue::Table(table) if table.len() == 1 => {
            if matches!(table.get("file"), Some(LuaValue::String(path)) if !path.trim().is_empty())
            {
                return;
            }
            issues.push(error(format!(
                "{field} file-backed metadata must use {{ file = \"relative/path\" }}"
            )));
        }
        _ => issues.push(error(format!(
            "{field} must be either an inline array or {{ file = \"...\" }}"
        ))),
    }
}

fn validate_alternatives(value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let Some(LuaValue::Array(entries)) = value else {
        if value.is_some() {
            issues.push(error(
                "alternatives must be an array of { name, link, path, priority } tables",
            ));
        }
        return;
    };

    for entry in entries {
        let LuaValue::Table(table) = entry else {
            issues.push(error(
                "alternatives entries must be { name, link, path, priority } tables",
            ));
            continue;
        };
        validate_string_key("alternatives", table, "name", issues);
        validate_string_key("alternatives", table, "link", issues);
        validate_string_key("alternatives", table, "path", issues);
        match table.get("priority") {
            Some(LuaValue::Integer(_)) => {}
            _ => issues.push(error(
                "alternatives entries require an integer `priority` field",
            )),
        }
    }
}

fn validate_hooks(value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let hooks = match value {
        Some(LuaValue::Table(hooks)) => hooks,
        Some(LuaValue::Array(arr)) if arr.is_empty() => return,
        Some(_) => {
            issues.push(error("hooks must be a table keyed by lifecycle point"));
            return;
        }
        None => return,
    };

    for (hook, spec) in hooks {
        let LuaValue::Table(table) = spec else {
            issues.push(error(format!(
                "hooks.{hook} must be a table such as {{ file = \"...\" }}"
            )));
            continue;
        };
        let has_file =
            matches!(table.get("file"), Some(LuaValue::String(path)) if !path.trim().is_empty());
        let has_lua =
            matches!(table.get("lua"), Some(LuaValue::String(chunk)) if !chunk.trim().is_empty());
        if !has_file && !has_lua {
            issues.push(error(format!(
                "hooks.{hook} must define either a non-empty `file` or `lua` entry"
            )));
        }
    }
}

fn validate_provider_assets(value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let families = match value {
        Some(LuaValue::Table(families)) => families,
        Some(LuaValue::Array(arr)) if arr.is_empty() => return,
        Some(_) => {
            issues.push(error(
                "provider_assets must be a table keyed by provider family, then provider name",
            ));
            return;
        }
        None => return,
    };

    for (family, providers) in families {
        let LuaValue::Table(providers) = providers else {
            issues.push(error(format!(
                "provider_assets.{family} must be a table keyed by provider name"
            )));
            continue;
        };
        for (provider, assets) in providers {
            let LuaValue::Array(entries) = assets else {
                issues.push(error(format!(
                    "provider_assets.{family}.{provider} must be an array of asset tables"
                )));
                continue;
            };
            for entry in entries {
                validate_provider_asset_entry(family, provider, entry, issues);
            }
        }
    }
}

fn validate_provider_asset_entry(
    family: &str,
    provider: &str,
    value: &LuaValue,
    issues: &mut Vec<ValidationIssue>,
) {
    let field = format!("provider_assets.{family}.{provider}");
    let LuaValue::Table(table) = value else {
        issues.push(error(format!(
            "{field} entries must be tables such as {{ kind = \"file\", target = \"/...\", file = \"...\" }}"
        )));
        return;
    };

    let kind = match table.get("kind").and_then(as_non_empty_string) {
        Some(kind) => kind,
        None => {
            issues.push(error(format!(
                "{field} entries require a non-empty `kind` field"
            )));
            return;
        }
    };
    match table.get("target").and_then(as_non_empty_string) {
        Some(target) if target.starts_with('/') => {}
        Some(_) => issues.push(error(format!(
            "{field} entries require an absolute `target` path"
        ))),
        None => issues.push(error(format!(
            "{field} entries require a non-empty `target` field"
        ))),
    }

    match kind {
        "file" => validate_provider_file_asset(&field, table, issues),
        "tree" => validate_provider_tree_asset(&field, table, issues),
        _ => issues.push(error(format!(
            "{field} entries must use supported kinds `file` or `tree`"
        ))),
    }
}

fn validate_provider_file_asset(
    field: &str,
    table: &BTreeMap<String, LuaValue>,
    issues: &mut Vec<ValidationIssue>,
) {
    let has_file = table.get("file").and_then(as_non_empty_string).is_some();
    let has_text = table.get("text").and_then(as_non_empty_string).is_some();
    if has_file == has_text {
        issues.push(error(format!(
            "{field} file assets must define exactly one of `file` or `text`"
        )));
    }
    if table.contains_key("dir") {
        issues.push(error(format!("{field} file assets must not define `dir`")));
    }
    if matches!(table.get("mode"), Some(value) if as_non_empty_string(value).is_none()) {
        issues.push(error(format!(
            "{field} file assets must use a non-empty string `mode` when present"
        )));
    }
}

fn validate_provider_tree_asset(
    field: &str,
    table: &BTreeMap<String, LuaValue>,
    issues: &mut Vec<ValidationIssue>,
) {
    if table.get("dir").and_then(as_non_empty_string).is_none() {
        issues.push(error(format!(
            "{field} tree assets require a non-empty `dir` field"
        )));
    }
    if table.contains_key("file") || table.contains_key("text") {
        issues.push(error(format!(
            "{field} tree assets must not define `file` or `text`"
        )));
    }
}

fn validate_flag_table(field: &str, value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let table = match value {
        Some(LuaValue::Table(table)) => table,
        Some(LuaValue::Array(arr)) if arr.is_empty() => return,
        Some(_) => {
            issues.push(error(format!("{field} must be a table")));
            return;
        }
        None => return,
    };

    for (flag, entry) in table {
        match field {
            "flags_default" | "flags_allowed" if !matches!(entry, LuaValue::Boolean(_)) => {
                issues.push(error(format!("{field}.{flag} must be a boolean")));
            }
            "flags_default" | "flags_allowed" => {}
            "flags_implies" | "flags_conflicts" => {
                let LuaValue::Array(entries) = entry else {
                    issues.push(error(format!(
                        "{field}.{flag} must be an array of non-empty flag names"
                    )));
                    continue;
                };
                for implied in entries {
                    if !matches!(implied, LuaValue::String(value) if !value.trim().is_empty()) {
                        issues.push(error(format!(
                            "{field}.{flag} must contain only non-empty string flag names"
                        )));
                    }
                }
            }
            _ => {}
        }
    }
}

fn validate_subpackages(value: Option<&LuaValue>, issues: &mut Vec<ValidationIssue>) {
    let Some(value) = value else {
        return;
    };

    if !matches!(value, LuaValue::Array(_) | LuaValue::Table(_)) {
        issues.push(error(
            "subpackages must be a table or array in the current declarative slice",
        ));
    }
}

fn validate_string_key(
    field: &str,
    table: &BTreeMap<String, LuaValue>,
    key: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if !matches!(table.get(key), Some(LuaValue::String(value)) if !value.trim().is_empty()) {
        issues.push(error(format!(
            "{field} entries require a non-empty `{key}` field"
        )));
    }
}

fn as_non_empty_string(value: &LuaValue) -> Option<&str> {
    match value {
        LuaValue::String(value) if !value.trim().is_empty() => Some(value.as_str()),
        _ => None,
    }
}
