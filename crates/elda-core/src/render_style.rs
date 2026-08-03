//! Minimal true-color styling for operator-facing frames in `elda-core`.

use std::cell::Cell;
use std::env;
use std::io::{IsTerminal, stdout};

const CHALK_YELLOW: (u8, u8, u8) = (229, 192, 123);
const SYNTAX_GREEN: (u8, u8, u8) = (152, 195, 121);
const CORAL_RED: (u8, u8, u8) = (224, 108, 117);
const LAVENDER_VIOLET: (u8, u8, u8) = (198, 120, 221);
const METRIC_ORANGE: (u8, u8, u8) = (209, 154, 102);

thread_local! {
    static FORCE_NO_COLOR: Cell<bool> = const { Cell::new(false) };
    static FORCE_COLOR: Cell<bool> = const { Cell::new(false) };
}

pub(crate) mod palette {
    pub(crate) const IDENTITY: (u8, u8, u8) = (220, 223, 228);
    pub(crate) const PROMPT: (u8, u8, u8) = super::SYNTAX_GREEN;
    pub(crate) const PROVENANCE: (u8, u8, u8) = super::CHALK_YELLOW;
    pub(crate) const SUCCESS: (u8, u8, u8) = super::SYNTAX_GREEN;
    pub(crate) const VERSION: (u8, u8, u8) = super::LAVENDER_VIOLET;
    pub(crate) const WARNING: (u8, u8, u8) = super::CHALK_YELLOW;
    pub(crate) const METRIC: (u8, u8, u8) = super::METRIC_ORANGE;
    pub(crate) const MUTED: (u8, u8, u8) = (127, 132, 156);
}

#[must_use]
pub(crate) fn color_enabled() -> bool {
    if FORCE_COLOR.with(Cell::get) {
        return true;
    }
    stdout().is_terminal() && env::var_os("NO_COLOR").is_none() && !FORCE_NO_COLOR.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn scoped_no_color_for_tests<T>(force: bool, f: impl FnOnce() -> T) -> T {
    let previous = FORCE_NO_COLOR.with(Cell::get);
    FORCE_NO_COLOR.with(|cell| cell.set(force));
    let result = f();
    FORCE_NO_COLOR.with(|cell| cell.set(previous));
    result
}

#[cfg(test)]
pub(crate) fn scoped_force_color_for_tests<T>(force: bool, f: impl FnOnce() -> T) -> T {
    let previous = FORCE_COLOR.with(Cell::get);
    FORCE_COLOR.with(|cell| cell.set(force));
    let result = f();
    FORCE_COLOR.with(|cell| cell.set(previous));
    result
}

#[must_use]
pub(crate) fn paint(text: &str, rgb: (u8, u8, u8), bold: bool) -> String {
    if !color_enabled() || text.is_empty() {
        return text.to_owned();
    }
    let mut prefix = format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2);
    if bold {
        prefix.push_str("\x1b[1m");
    }
    format!("{prefix}{text}\x1b[0m")
}

/// Apply semantic emphasis to framed operator output without tinting frame structure.
#[must_use]
pub fn highlight_operator_frame(text: &str) -> String {
    if !color_enabled() {
        return text.to_owned();
    }

    text.lines()
        .map(highlight_operator_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Color live progress tree lines (stderr stream, not static frames).
#[must_use]
pub(crate) fn highlight_progress_line(line: &str) -> String {
    if !color_enabled() || line.contains('\x1b') {
        return line.to_owned();
    }

    if let Some(rest) = line.strip_prefix("\u{2514}\u{2500} ") {
        return format!("\u{2514}\u{2500} {}", highlight_progress_footer(rest));
    }

    let (connector, body) = match line.split_once("  ") {
        Some((connector, body)) if connector == "\u{2502}" || connector == "\u{251c}\u{2500}" => {
            (connector, body)
        }
        _ => return line.to_owned(),
    };

    let (glyph, remainder) = match split_progress_glyph(body) {
        Some(parts) => parts,
        None => return line.to_owned(),
    };

    let (label, detail) = split_progress_label(remainder);
    let build_step = is_build_progress_label(label);
    let painted_glyph = paint_progress_glyph(glyph, build_step);
    let painted_label = if build_step {
        paint(label, palette::METRIC, false)
    } else {
        label.to_owned()
    };

    match detail {
        Some(detail) if !detail.is_empty() => {
            format!("{connector}  {painted_glyph} {painted_label}: {detail}")
        }
        _ => format!("{connector}  {painted_glyph} {painted_label}"),
    }
}

/// Dim the structural connector so content reads first. Per the redesign
/// contract, frame borders are neutral/dim and never carry semantic colour.
fn border(chars: &str) -> String {
    paint(chars, palette::MUTED, false)
}

fn highlight_operator_line(line: &str) -> String {
    if line.contains('\x1b') {
        return line.to_owned();
    }
    if let Some(body) = line.strip_prefix("\u{2502}  ") {
        if let Some((key, rest)) = body.split_once("::") {
            // Preserve the column padding the frame renderer computed; only the
            // value is repainted, and the key column stays neutral.
            let value = rest.trim_start();
            let padding = &rest[..rest.len() - value.len()];
            return format!(
                "{}  {key}::{padding}{}",
                border("\u{2502}"),
                paint_kv_value(key.trim(), value.trim_end())
            );
        }
        if let Some(rest) = body.strip_prefix('\u{2714}') {
            return format!(
                "{}  {}{}",
                border("\u{2502}"),
                paint("\u{2714}", palette::SUCCESS, true),
                rest
            );
        }
        return format!("{}  {body}", border("\u{2502}"));
    }
    if line == "\u{2502}" {
        return border("\u{2502}");
    }
    if let Some(title) = line.strip_prefix("\u{251c}\u{2500} ") {
        let trimmed = title.trim_end_matches(':').trim();
        return format!("{} {trimmed}:", border("\u{251c}\u{2500}"));
    }
    if let Some(title) = line.strip_prefix("\u{250c}\u{2500} ") {
        return format!("{} {title}", border("\u{250c}\u{2500}"));
    }
    if let Some(rest) = line.strip_prefix("\u{2514}\u{2500} ") {
        return format!(
            "{} {}",
            border("\u{2514}\u{2500}"),
            highlight_operator_footer(rest)
        );
    }
    if let Some(rest) = line.strip_prefix("advisory:") {
        return format!("{}{}", paint("advisory:", palette::WARNING, true), rest);
    }
    line.to_owned()
}

fn paint_kv_value(key: &str, value: &str) -> String {
    match key {
        "target" | "package" => bold_text(value),
        "paths" | "objects" => paint(value, palette::METRIC, false),
        "version" => paint(value, palette::VERSION, false),
        _ => paint_value(value),
    }
}

fn bold_text(text: &str) -> String {
    if !color_enabled() || text.is_empty() {
        return text.to_owned();
    }
    format!("\x1b[1m{text}\x1b[0m")
}

fn highlight_operator_footer(rest: &str) -> String {
    if rest.contains("Proceed?") || rest.contains("Accept and import") {
        return colorize_token(rest, "Proceed?", palette::PROMPT);
    }
    if rest.starts_with('\u{2714}') || rest.contains("installed") {
        let mut styled = rest.to_owned();
        styled = styled.replace('\u{2714}', &paint("\u{2714}", palette::SUCCESS, true));
        if rest.contains("installed") {
            styled = colorize_token(&styled, "installed", palette::SUCCESS);
        }
        return styled;
    }
    if rest.starts_with("dry run") {
        return paint(rest, palette::MUTED, false);
    }
    rest.to_owned()
}

fn highlight_progress_footer(rest: &str) -> String {
    let mut styled = rest.to_owned();
    if rest.contains('\u{2714}') {
        styled = styled.replace('\u{2714}', &paint("\u{2714}", palette::SUCCESS, true));
    }
    if rest.contains(" ok") {
        styled = colorize_token(&styled, "ok", palette::SUCCESS);
    }
    styled
}

fn paint_value(value: &str) -> String {
    let mut styled = value.to_owned();
    styled = colorize_token(&styled, "[I]", palette::PROVENANCE);
    styled = colorize_token(&styled, "[E]", palette::SUCCESS);
    styled = colorize_token(&styled, "[F]", palette::VERSION);
    styled = colorize_token(&styled, "[V]", palette::VERSION);
    styled = colorize_token(&styled, "[A]", CORAL_RED);
    styled
}

fn paint_progress_glyph(glyph: &str, build_step: bool) -> String {
    match glyph {
        "\u{2714}" => paint("\u{2714}", palette::SUCCESS, true),
        "\u{2718}" => paint("\u{2718}", CORAL_RED, true),
        "\u{25cf}" | "\u{00b7}" => paint(glyph, palette::MUTED, false),
        _ if triangle_spinner(glyph) => paint(glyph, palette::METRIC, true),
        _ if build_step => paint(glyph, palette::METRIC, true),
        _ => glyph.to_owned(),
    }
}

fn split_progress_glyph(body: &str) -> Option<(&str, &str)> {
    let body = body.trim_start();
    if let Some(rest) = body.strip_prefix('\u{2714}') {
        return Some(("\u{2714}", rest.trim_start()));
    }
    if let Some(rest) = body.strip_prefix('\u{2718}') {
        return Some(("\u{2718}", rest.trim_start()));
    }
    if let Some(rest) = body.strip_prefix('\u{00b7}') {
        return Some(("\u{00b7}", rest.trim_start()));
    }
    if let Some(rest) = body.strip_prefix("[ok]") {
        return Some(("[ok]", rest.trim_start()));
    }
    if let Some(rest) = body.strip_prefix("[..]") {
        return Some(("[..]", rest.trim_start()));
    }
    for glyph in ["\u{25e2}", "\u{25e3}", "\u{25e4}", "\u{25e5}"] {
        if let Some(rest) = body.strip_prefix(glyph) {
            return Some((glyph, rest.trim_start()));
        }
    }
    None
}

fn split_progress_label(remainder: &str) -> (&str, Option<&str>) {
    if let Some((label, detail)) = remainder.split_once(": ") {
        return (label.trim(), Some(detail.trim()));
    }
    (remainder.trim(), None)
}

fn is_build_progress_label(label: &str) -> bool {
    label.contains("build source")
        || label.contains("acquire source")
        || label.contains("stage payload")
        || label.contains("build source payload")
}

fn triangle_spinner(glyph: &str) -> bool {
    matches!(glyph, "\u{25e2}" | "\u{25e3}" | "\u{25e4}" | "\u{25e5}")
}

fn colorize_token(line: &str, token: &str, rgb: (u8, u8, u8)) -> String {
    line.replace(token, &paint(token, rgb, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The key *text* between the dimmed connector and `::`. The connector
    /// itself is allowed to carry the muted border colour.
    fn key_text(styled: &str) -> String {
        let key_side = styled.split("::").next().expect("key part");
        let after_connector = key_side
            .rsplit_once("\x1b[0m")
            .map_or(key_side, |(_, rest)| rest);
        after_connector.trim().to_owned()
    }

    #[test]
    fn kv_keys_stay_plain_and_target_value_is_bold_only() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{2502}  target:: foot";
            let styled = highlight_operator_line(line);
            assert_eq!(key_text(&styled), "target", "key text must be plain");
            assert!(
                !key_text(&styled).contains('\x1b'),
                "key side should stay plain: {styled}"
            );
            assert!(
                styled.contains("\x1b[1mfoot\x1b[0m"),
                "value should be bold: {styled}"
            );
        });
    }

    #[test]
    fn kv_keys_stay_neutral_and_objects_use_metric_orange() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{2502}  objects:: 11 shlib require(s), 0 shlib provide(s)";
            let styled = highlight_operator_line(line);
            assert_eq!(key_text(&styled), "objects", "key text must be plain");
            assert!(styled.contains("11 shlib"));
            assert!(styled.contains("\x1b["), "expected value color: {styled}");
        });
    }

    #[test]
    fn frame_title_stays_plain_while_the_border_dims() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{250c}\u{2500} install foot";
            let styled = highlight_operator_line(line);
            let title = styled.split(' ').skip(1).collect::<Vec<_>>().join(" ");
            assert_eq!(
                title, "install foot",
                "title text must carry no styling: {styled}"
            );
            assert!(
                styled.starts_with("\x1b["),
                "border should be dimmed: {styled}"
            );
        });
    }

    #[test]
    fn kv_padding_survives_colorization() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{2502}  mode::      system";
            let styled = highlight_operator_line(line);
            assert!(
                styled.contains("mode::      "),
                "column padding must be preserved: {styled:?}"
            );
        });
    }

    #[test]
    fn plain_lines_keep_their_text_and_only_dim_the_connector() {
        scoped_force_color_for_tests(true, || {
            let styled = highlight_operator_line("\u{2502}  some free-form detail");
            assert!(styled.ends_with("  some free-form detail"), "{styled:?}");
        });
    }

    #[test]
    fn highlight_paths_value_uses_metric_orange_when_color_forced() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{2502}  paths:: 139";
            let styled = highlight_operator_line(line);
            assert!(styled.contains("\x1b["), "expected ansi: {styled}");
            assert!(styled.contains("139"));
        });
    }

    #[test]
    fn highlight_installed_footer_marks_success_glyph_and_word() {
        scoped_force_color_for_tests(true, || {
            let line =
                "\u{2514}\u{2500} \u{2714} installed 1 target(s) into the current Elda root.";
            let styled = highlight_operator_line(line);
            assert!(styled.contains("\x1b["), "expected ansi: {styled}");
            assert!(styled.contains("installed"));
        });
    }

    #[test]
    fn progress_done_line_marks_green_check_and_orange_build_label() {
        scoped_force_color_for_tests(true, || {
            let line = "\u{2502}  \u{2714} build source: build pipeline";
            let styled = highlight_progress_line(line);
            assert!(styled.contains("\x1b["), "expected ansi: {styled}");
            assert!(styled.contains("build pipeline"));
        });
    }
}
