mod data;
mod render;
mod style;
#[cfg(test)]
mod tests;

use std::ffi::OsString;

use render::render_root_help;
use style::{color_enabled, is_root_help_token, terminal_width};

pub fn should_print_root_help(args: &[OsString]) -> bool {
    match args {
        [_] => true,
        [_, token] => is_root_help_token(token),
        _ => false,
    }
}

pub fn print_root_help() {
    println!("{}", render_root_help(color_enabled(), terminal_width()));
}
