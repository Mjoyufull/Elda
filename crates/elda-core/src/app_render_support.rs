use serde_json::Value;

#[must_use]
pub(crate) fn render_json_block(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_owned())
}

#[must_use]
pub(crate) fn render_header(area: &str, status: &str) -> String {
    format!("{area}: {status}")
}
