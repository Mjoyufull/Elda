use super::data::{
    ACID_LIME, BURNT_RUST, CORE_ROWS, ELECTRIC_MAGENTA, EXAMPLES, FLAG_ROWS, HAZARD_ORANGE,
    HelpRow, LOGO, NAMESPACE_ROWS, PURE_WHITE, SIGNAL_YELLOW, STATE_ROWS,
};
use super::style::{center_text, paint, section_title};

pub(super) fn render_root_help(color: bool, width: usize) -> String {
    let mut output = String::new();
    output.push_str(&render_logo(color, width));
    output.push('\n');
    output.push_str(&center_text(
        "replacement-grade Unix-first package manager",
        width,
        color,
        PURE_WHITE,
        false,
    ));
    output.push('\n');
    output.push_str(&center_text(
        env!("CARGO_PKG_VERSION"),
        width,
        color,
        ELECTRIC_MAGENTA,
        true,
    ));
    output.push_str("\n\n");
    output.push_str(&section_title("Usage", color));
    output.push('\n');
    output.push_str(&format!(
        "{}\n\n",
        paint(
            "elda <command> [options] [operands]",
            SIGNAL_YELLOW,
            true,
            color
        )
    ));
    output.push_str(&format_section("Core Commands", CORE_ROWS, color));
    output.push('\n');
    output.push_str(&format_section("State And Repair", STATE_ROWS, color));
    output.push('\n');
    output.push_str(&format_section("Namespaces", NAMESPACE_ROWS, color));
    output.push('\n');
    output.push_str(&format_section("Global Flags", FLAG_ROWS, color));
    output.push('\n');
    output.push_str(&format_examples(color));
    output
}

fn format_section(title: &str, rows: &[HelpRow], color: bool) -> String {
    let width = rows
        .iter()
        .map(HelpRow::label_width)
        .max()
        .unwrap_or_default();
    let mut output = String::new();
    output.push_str(&section_title(title, color));
    output.push('\n');
    for (index, row) in rows.iter().enumerate() {
        let branch = if index + 1 == rows.len() {
            "└─"
        } else {
            "├─"
        };
        output.push_str(&paint(branch, HAZARD_ORANGE, true, color));
        output.push(' ');
        output.push_str(&paint(row.command, ACID_LIME, true, color));
        if !row.args.is_empty() {
            output.push(' ');
            output.push_str(&paint(row.args, PURE_WHITE, false, color));
        }
        let padding = width.saturating_sub(row.label_width()) + 2;
        output.push_str(&" ".repeat(padding));
        output.push(' ');
        output.push_str(&paint("#", PURE_WHITE, true, color));
        output.push(' ');
        output.push_str(&paint(row.description, PURE_WHITE, false, color));
        output.push('\n');
    }
    output
}

fn format_examples(color: bool) -> String {
    let width = EXAMPLES
        .iter()
        .map(|row| row.command.chars().count())
        .max()
        .unwrap_or_default();
    let mut output = String::new();
    output.push_str(&section_title("Examples", color));
    output.push('\n');
    for (index, row) in EXAMPLES.iter().enumerate() {
        let branch = if index + 1 == EXAMPLES.len() {
            "└─"
        } else {
            "├─"
        };
        output.push_str(&paint(branch, HAZARD_ORANGE, true, color));
        output.push(' ');
        output.push_str(&paint(row.command, ELECTRIC_MAGENTA, true, color));
        let padding = width.saturating_sub(row.command.chars().count()) + 2;
        output.push_str(&" ".repeat(padding));
        output.push(' ');
        output.push_str(&paint("#", PURE_WHITE, true, color));
        output.push(' ');
        output.push_str(&paint(row.description, PURE_WHITE, false, color));
        output.push('\n');
    }
    output
}

fn render_logo(color: bool, width: usize) -> String {
    LOGO.lines()
        .map(|line| center_logo_line(line, width, color))
        .collect::<Vec<_>>()
        .join("\n")
}

fn center_logo_line(line: &str, width: usize, color: bool) -> String {
    let visible = line.chars().count();
    let padding = width.saturating_sub(visible) / 2;
    let mut output = " ".repeat(padding);
    if color {
        output.push_str(&colorize_logo_line(line));
    } else {
        output.push_str(line);
    }
    output
}

fn colorize_logo_line(line: &str) -> String {
    let mut output = String::new();
    for ch in line.chars() {
        match ch {
            'E' | 'L' | 'D' | 'A' => {
                output.push_str(&paint(&ch.to_string(), ACID_LIME, true, true))
            }
            'M' | 'm' => output.push_str(&paint(&ch.to_string(), HAZARD_ORANGE, true, true)),
            'H' | 'K' | 'I' | 'F' => {
                output.push_str(&paint(&ch.to_string(), BURNT_RUST, true, true))
            }
            '#' | '@' => output.push_str(&paint(&ch.to_string(), ELECTRIC_MAGENTA, true, true)),
            _ => output.push(ch),
        }
    }
    output
}
