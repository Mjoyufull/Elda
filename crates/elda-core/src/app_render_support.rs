use serde_json::Value;

use crate::CommandReport;
use crate::app_render_tree::{FrameFooter, TreeStyle, frame_from_sections};

/// One tree frame for command-specific human output (UX §1.3 / §6).
#[must_use]
pub(crate) fn human_framed_report(
    _report: &CommandReport,
    frame_title: impl Into<String>,
    sections: &[(String, Vec<String>)],
    footer: Option<FrameFooter>,
) -> String {
    let frame = frame_from_sections(frame_title.into(), sections, footer);
    frame.render(TreeStyle::detect())
}

#[must_use]
pub(crate) fn render_header(area: &str, status: &str) -> String {
    format!("{area} {status}")
}

pub(crate) fn render_section(title: &str, lines: &[String]) -> String {
    let mut rendered = String::from(title);
    for line in lines {
        rendered.push('\n');
        rendered.push_str("  ");
        rendered.push_str(line);
    }
    rendered
}

pub(crate) fn json_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

pub(crate) fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_u64()
}

pub(crate) fn json_array<'a>(value: &'a Value, path: &[&str]) -> Option<&'a [Value]> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_array().map(Vec::as_slice)
}
