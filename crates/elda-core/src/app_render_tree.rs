//! Shared tree-style frame primitives for human-mode renderers.
//!
//! These helpers produce the connector-and-glyph output that the operator
//! sees in `┌─ ├─ │ └─` blocks. They are pure formatters; the live
//! progression sink uses the same glyphs but emits lines incrementally.
//!
//! All static renderers must format frames through this module so that the
//! Unicode/ASCII fallback decision lives in one place. A few row variants
//! and the `Bullet` glyph are part of the documented frame surface from
//! `eldainstallaztionuxandcliimprovements.md` §13.4 but not yet consumed by
//! every handler. They stay on the API so the pattern matches in renderers
//! stay exhaustive as the static-frame sweep lands.
#![allow(dead_code)]

use std::{cell::Cell, env};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeStyle {
    Unicode,
    Ascii,
}

thread_local! {
    static CONFIGURED_TREE_STYLE: Cell<Option<TreeStyle>> = const { Cell::new(None) };
}

pub(crate) fn set_configured_tree_style(style: Option<TreeStyle>) {
    CONFIGURED_TREE_STYLE.with(|cell| cell.set(style));
}

impl TreeStyle {
    pub(crate) fn detect() -> Self {
        if let Some(style) = configured_tree_style() {
            return style;
        }
        if env::var_os("ELDA_TREE_ASCII").is_some() || env::var_os("NO_UNICODE").is_some() {
            return Self::Ascii;
        }
        match env::var("LANG") {
            Ok(value) if value.to_ascii_lowercase().contains("utf") => Self::Unicode,
            Ok(_) => match env::var("LC_ALL") {
                Ok(other) if other.to_ascii_lowercase().contains("utf") => Self::Unicode,
                _ => Self::Ascii,
            },
            Err(_) => match env::var("LC_ALL") {
                Ok(other) if other.to_ascii_lowercase().contains("utf") => Self::Unicode,
                _ => Self::Unicode,
            },
        }
    }
}

fn configured_tree_style() -> Option<TreeStyle> {
    CONFIGURED_TREE_STYLE.with(Cell::get)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Glyph {
    Done,
    Blocked,
    Warn,
    Running,
    Skipped,
    Bullet,
}

impl Glyph {
    pub(crate) fn render(self, style: TreeStyle) -> &'static str {
        match (self, style) {
            (Self::Done, TreeStyle::Unicode) => "✔",
            (Self::Done, TreeStyle::Ascii) => "[ok]",
            (Self::Blocked, TreeStyle::Unicode) => "✘",
            (Self::Blocked, TreeStyle::Ascii) => "[!!]",
            (Self::Warn, TreeStyle::Unicode) => "⚠",
            (Self::Warn, TreeStyle::Ascii) => "[w]",
            (Self::Running, TreeStyle::Unicode) => "◢",
            (Self::Running, TreeStyle::Ascii) => "[..]",
            (Self::Skipped, TreeStyle::Unicode) => "·",
            (Self::Skipped, TreeStyle::Ascii) => "[--]",
            (Self::Bullet, TreeStyle::Unicode) => "•",
            (Self::Bullet, TreeStyle::Ascii) => "*",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TreeChars {
    top: &'static str,
    mid: &'static str,
    vert: &'static str,
    bot: &'static str,
}

impl TreeChars {
    fn for_style(style: TreeStyle) -> Self {
        match style {
            TreeStyle::Unicode => Self {
                top: "┌─",
                mid: "├─",
                vert: "│",
                bot: "└─",
            },
            TreeStyle::Ascii => Self {
                top: "+-",
                mid: "+-",
                vert: "|",
                bot: "+-",
            },
        }
    }
}

/// A row inside a frame.
#[derive(Debug, Clone)]
pub(crate) enum Row {
    /// A new sub-section header (`├─ Title`).
    Section(String),
    /// A blank vertical-only spacer line (`│`).
    Spacer,
    /// A simple labeled line (`│  label`).
    Line(String),
    /// A glyph + label line (`│  ✔ label`).
    Glyph { glyph: Glyph, label: String },
    /// A two-column key/value line (`│  key:: value`).
    KeyValue { key: String, value: String },
}

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    title: String,
    rows: Vec<Row>,
    footer: Option<FrameFooter>,
}

#[derive(Debug, Clone)]
pub(crate) struct FrameFooter {
    pub(crate) glyph: Option<Glyph>,
    pub(crate) text: String,
}

impl Frame {
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
            footer: None,
        }
    }

    pub(crate) fn section(&mut self, title: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Section(title.into()));
        self
    }

    pub(crate) fn spacer(&mut self) -> &mut Self {
        self.rows.push(Row::Spacer);
        self
    }

    pub(crate) fn line(&mut self, label: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Line(label.into()));
        self
    }

    pub(crate) fn glyph_line(&mut self, glyph: Glyph, label: impl Into<String>) -> &mut Self {
        self.rows.push(Row::Glyph {
            glyph,
            label: label.into(),
        });
        self
    }

    pub(crate) fn kv(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.rows.push(Row::KeyValue {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    pub(crate) fn footer(&mut self, footer: FrameFooter) -> &mut Self {
        self.footer = Some(footer);
        self
    }

    pub(crate) fn render(&self, style: TreeStyle) -> String {
        let chars = TreeChars::for_style(style);
        let mut buffer = String::new();
        buffer.push_str(chars.top);
        buffer.push(' ');
        buffer.push_str(&self.title);

        for row in &self.rows {
            buffer.push('\n');
            match row {
                Row::Section(title) => {
                    buffer.push_str(chars.mid);
                    buffer.push(' ');
                    buffer.push_str(title);
                    buffer.push(':');
                }
                Row::Spacer => {
                    buffer.push_str(chars.vert);
                }
                Row::Line(label) => {
                    buffer.push_str(chars.vert);
                    buffer.push_str("  ");
                    buffer.push_str(label);
                }
                Row::Glyph { glyph, label } => {
                    buffer.push_str(chars.vert);
                    buffer.push_str("  ");
                    buffer.push_str(glyph.render(style));
                    buffer.push(' ');
                    buffer.push_str(label);
                }
                Row::KeyValue { key, value } => {
                    buffer.push_str(chars.vert);
                    buffer.push_str("  ");
                    buffer.push_str(key);
                    buffer.push_str(":: ");
                    buffer.push_str(value);
                }
            }
        }

        buffer.push('\n');
        buffer.push_str(chars.bot);
        if let Some(footer) = &self.footer {
            buffer.push(' ');
            if let Some(glyph) = footer.glyph {
                buffer.push_str(glyph.render(style));
                buffer.push(' ');
            }
            buffer.push_str(&footer.text);
        }
        buffer
    }
}

/// Convenience: render a one-shot frame with the default style.
pub(crate) fn render_frame(frame: &Frame) -> String {
    frame.render(TreeStyle::detect())
}

/// Build a frame from `(section_title, body_lines)` pairs and an optional
/// footer line. Each section becomes an `├─ Title` row followed by one
/// `│  line` per entry. This is the bridge that converts every existing
/// `render_section`-style renderer to the spec's framed format without
/// rewriting per-section line composition.
pub(crate) fn frame_from_sections(
    title: impl Into<String>,
    sections: &[(String, Vec<String>)],
    footer: Option<FrameFooter>,
) -> Frame {
    let mut frame = Frame::new(title);
    for (idx, (section_title, lines)) in sections.iter().enumerate() {
        if idx > 0 {
            frame.spacer();
        }
        frame.section(section_title.clone());
        for line in lines {
            frame.line(line.clone());
        }
    }
    if let Some(footer) = footer {
        frame.footer(footer);
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::{Frame, FrameFooter, Glyph, TreeStyle};

    #[test]
    fn unicode_frame_renders_connectors_and_glyphs() {
        let mut frame = Frame::new("System Health");
        frame
            .section("DB Integrity")
            .glyph_line(Glyph::Done, "Pass")
            .section("Orphan Packages")
            .glyph_line(Glyph::Warn, "2 found (`elda autoremove`)")
            .footer(FrameFooter {
                glyph: None,
                text: "Overall Status: Warning".to_owned(),
            });

        let rendered = frame.render(TreeStyle::Unicode);
        let expected = "┌─ System Health\n├─ DB Integrity:\n│  ✔ Pass\n├─ Orphan Packages:\n│  ⚠ 2 found (`elda autoremove`)\n└─ Overall Status: Warning";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn ascii_fallback_emits_plain_connectors() {
        let mut frame = Frame::new("Sync");
        frame
            .section("yoka-core")
            .glyph_line(Glyph::Done, "fetched")
            .footer(FrameFooter {
                glyph: Some(Glyph::Done),
                text: "Sync complete".to_owned(),
            });

        let rendered = frame.render(TreeStyle::Ascii);
        let expected = "+- Sync\n+- yoka-core:\n|  [ok] fetched\n+- [ok] Sync complete";
        assert_eq!(rendered, expected);
    }

    #[test]
    fn key_value_rows_align_to_padding() {
        let mut frame = Frame::new("Target");
        frame.kv("requested", "hyprland");
        frame.kv("mode", "system");

        let rendered = frame.render(TreeStyle::Unicode);
        assert!(
            rendered.contains("│  requested:: hyprland"),
            "missing requested row: {rendered}"
        );
        assert!(
            rendered.contains("│  mode:: system"),
            "missing mode row: {rendered}"
        );
    }
}
