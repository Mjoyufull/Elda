use std::env;
use std::ffi::OsString;
use std::io::{IsTerminal, stdout};

use super::data::PURE_WHITE;

pub(super) fn center_text(
    text: &str,
    width: usize,
    color: bool,
    rgb: (u8, u8, u8),
    bold: bool,
) -> String {
    let padding = width.saturating_sub(text.chars().count()) / 2;
    format!("{}{}", " ".repeat(padding), paint(text, rgb, bold, color))
}

pub(super) fn section_title(title: &str, color: bool) -> String {
    paint(title, PURE_WHITE, true, color)
}

pub(super) fn paint(text: &str, rgb: (u8, u8, u8), bold: bool, color: bool) -> String {
    if !color || text.is_empty() {
        return text.to_owned();
    }

    let mut prefix = format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2);
    if bold {
        prefix.push_str("\x1b[1m");
    }
    format!("{prefix}{text}\x1b[0m")
}

pub(super) fn color_enabled() -> bool {
    stdout().is_terminal() && env::var_os("NO_COLOR").is_none()
}

pub(super) fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 72)
        .unwrap_or(100)
}

pub(super) fn is_root_help_token(token: &OsString) -> bool {
    matches!(token.to_str(), Some("help" | "-h" | "--help"))
}
