use std::collections::BTreeMap;

use crate::error::RecipeError;
use crate::model::{DependencyEntry, LuaValue, ScalarValue};

pub(super) fn get_required_string(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<String, RecipeError> {
    match table.get(key) {
        Some(LuaValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(RecipeError::Parse(format!(
            "field `{key}` must be a string"
        ))),
        None => Err(RecipeError::Parse(format!("field `{key}` is required"))),
    }
}

pub(super) fn get_required_integer(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<i64, RecipeError> {
    match table.get(key) {
        Some(LuaValue::Integer(value)) => Ok(*value),
        Some(_) => Err(RecipeError::Parse(format!(
            "field `{key}` must be an integer"
        ))),
        None => Err(RecipeError::Parse(format!("field `{key}` is required"))),
    }
}

pub(super) fn get_optional_integer(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Option<i64>, RecipeError> {
    match table.get(key) {
        Some(LuaValue::Integer(value)) => Ok(Some(*value)),
        Some(_) => Err(RecipeError::Parse(format!(
            "field `{key}` must be an integer"
        ))),
        None => Ok(None),
    }
}

pub(super) fn get_optional_string(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Option<String>, RecipeError> {
    match table.get(key) {
        Some(LuaValue::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(RecipeError::Parse(format!(
            "field `{key}` must be a string"
        ))),
        None => Ok(None),
    }
}

pub(super) fn get_required_table(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<BTreeMap<String, LuaValue>, RecipeError> {
    match table.get(key) {
        Some(LuaValue::Table(value)) => Ok(value.clone()),
        Some(_) => Err(RecipeError::Parse(format!("field `{key}` must be a table"))),
        None => Err(RecipeError::Parse(format!("field `{key}` is required"))),
    }
}

pub(super) fn get_optional_table(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Option<BTreeMap<String, LuaValue>>, RecipeError> {
    match table.get(key) {
        Some(LuaValue::Table(value)) => Ok(Some(value.clone())),
        Some(_) => Err(RecipeError::Parse(format!("field `{key}` must be a table"))),
        None => Ok(None),
    }
}

pub(super) fn get_optional_boolean(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Option<bool>, RecipeError> {
    match table.get(key) {
        Some(LuaValue::Boolean(value)) => Ok(Some(*value)),
        Some(_) => Err(RecipeError::Parse(format!(
            "field `{key}` must be a boolean"
        ))),
        None => Ok(None),
    }
}

pub(super) fn get_optional_value(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Option<LuaValue> {
    table.get(key).cloned()
}

pub(super) fn get_required_string_array(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Vec<String>, RecipeError> {
    get_optional_string_array(table, key)?
        .filter(|values| !values.is_empty())
        .ok_or_else(|| {
            RecipeError::Parse(format!(
                "field `{key}` must be a non-empty array of strings"
            ))
        })
}

pub(super) fn get_optional_string_array(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Option<Vec<String>>, RecipeError> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    let LuaValue::Array(items) = value else {
        return Err(RecipeError::Parse(format!(
            "field `{key}` must be an array"
        )));
    };

    let mut strings = Vec::with_capacity(items.len());
    for item in items {
        let LuaValue::String(string) = item else {
            return Err(RecipeError::Parse(format!(
                "field `{key}` must contain only strings"
            )));
        };
        strings.push(string.clone());
    }

    Ok(Some(strings))
}

pub(super) fn get_dependency_entries(
    table: &BTreeMap<String, LuaValue>,
    key: &str,
) -> Result<Vec<DependencyEntry>, RecipeError> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let LuaValue::Array(items) = value else {
        return Err(RecipeError::Parse(format!(
            "field `{key}` must be an array"
        )));
    };

    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        match item {
            LuaValue::String(value) => entries.push(DependencyEntry::Constraint(value.clone())),
            LuaValue::Table(table) => entries.push(parse_any_of_dependency(key, table)?),
            _ => {
                return Err(RecipeError::Parse(format!(
                    "field `{key}` supports only string constraints or any-of tables"
                )));
            }
        }
    }

    Ok(entries)
}

pub(super) fn integer_to_u64(value: i64, field: &str) -> Result<u64, RecipeError> {
    u64::try_from(value)
        .map_err(|_| RecipeError::Parse(format!("field `{field}` cannot be negative")))
}

pub(super) fn scalar_from_lua(key: &str, value: LuaValue) -> Result<ScalarValue, RecipeError> {
    match value {
        LuaValue::String(value) => Ok(ScalarValue::String(value)),
        LuaValue::Integer(value) => Ok(ScalarValue::Integer(value)),
        LuaValue::Boolean(value) => Ok(ScalarValue::Boolean(value)),
        _ => Err(RecipeError::Parse(format!(
            "source field `{key}` must be a scalar value in the current declarative subset"
        ))),
    }
}

fn parse_any_of_dependency(
    key: &str,
    table: &BTreeMap<String, LuaValue>,
) -> Result<DependencyEntry, RecipeError> {
    let Some(LuaValue::Array(values)) = table.get("any") else {
        return Err(RecipeError::Parse(format!(
            "field `{key}` supports only string constraints or {{ any = {{ ... }} }} tables"
        )));
    };

    let mut providers = Vec::with_capacity(values.len());
    for value in values {
        let LuaValue::String(value) = value else {
            return Err(RecipeError::Parse(format!(
                "field `{key}` any-of providers must be strings"
            )));
        };
        providers.push(value.clone());
    }

    Ok(DependencyEntry::AnyOf(providers))
}
