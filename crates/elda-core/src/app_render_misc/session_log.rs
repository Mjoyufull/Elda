use serde_json::Value;

use crate::app_render_support::render_section;
use crate::run_log::session_log_path;

pub(crate) fn render_session_log_section(details: &Value) -> Option<String> {
    let path = session_log_path(details)?;
    Some(render_section("Log", &[format!("path: {path}")]))
}
