use std::ffi::OsString;

use super::render::render_root_help;
use super::should_print_root_help;

#[test]
fn detects_root_help_invocations() {
    assert!(should_print_root_help(&[OsString::from("elda")]));
    assert!(should_print_root_help(&[
        OsString::from("elda"),
        OsString::from("help"),
    ]));
    assert!(should_print_root_help(&[
        OsString::from("elda"),
        OsString::from("--help"),
    ]));
    assert!(!should_print_root_help(&[
        OsString::from("elda"),
        OsString::from("search"),
        OsString::from("--help"),
    ]));
}

#[test]
fn root_help_contains_branded_sections_and_examples() {
    let rendered = render_root_help(false, 100);

    assert!(rendered.contains("replacement-grade Unix-first package manager"));
    assert!(rendered.contains("Core Commands"));
    assert!(rendered.contains("i <target...>"));
    assert!(rendered.contains("├─ i <target...>"));
    assert!(rendered.contains("# install package names, recipes, or git targets"));
    assert!(rendered.contains("vendor add/import/export"));
    assert!(rendered.contains("└─ elda help <command>"));
}
