use super::data::{ASH_WHITE, PEACH_HAZARD, SYNTAX_GREEN};
use super::style::{paint, section_title};

pub(super) fn render_framed_command_help(command: &str, clap_help: &str) -> String {
    let mut output = String::new();
    output.push_str(&section_title("Command Help", true));
    output.push('\n');
    output.push_str(&format!(
        "{} {}\n\n",
        paint("└─", PEACH_HAZARD, true, true),
        paint(command, SYNTAX_GREEN, true, true)
    ));
    for line in clap_help.lines() {
        if line.trim().is_empty() {
            output.push('\n');
            continue;
        }
        output.push_str(&paint("│  ", PEACH_HAZARD, true, true));
        output.push_str(&paint(line, ASH_WHITE, false, true));
        output.push('\n');
    }
    output
}
