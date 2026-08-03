//! Assertion helpers for framed human output.
//!
//! Frame key/value rows are column-aligned, so the gap after `::` varies with
//! the widest key in the frame. Tests assert on the *content* of a row, not on
//! how far the value column happened to land.

/// True when the rendered frame contains a `key:: value` row, ignoring the
/// column padding between the separator and the value.
pub(crate) fn has_row(rendered: &str, key: &str, value: &str) -> bool {
    find_row(rendered, key).is_some_and(|actual| actual.starts_with(value))
}

/// The value side of the first `key::` row, with column padding stripped.
///
/// The key must fill the whole key column, so asking for `parser` does not
/// match an `interbuild parser::` row.
pub(crate) fn find_row<'a>(rendered: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}::");
    rendered.lines().find_map(|line| {
        let body = line.trim_start_matches(['\u{2502}', '|', ' ']);
        Some(body.strip_prefix(&needle)?.trim_start())
    })
}

#[cfg(test)]
mod tests {
    use super::{find_row, has_row};

    const FRAME: &str = "\
┌─ install foot
│  target::   foot
│  version::  0:9999-1
│  interbuild parser:: nix_flake
└─ Proceed? [Y/n/e]";

    #[test]
    fn rows_match_regardless_of_column_padding() {
        assert!(has_row(FRAME, "target", "foot"));
        assert!(has_row(FRAME, "version", "0:9999-1"));
        assert_eq!(find_row(FRAME, "target"), Some("foot"));
    }

    #[test]
    fn a_short_key_does_not_match_a_longer_key_ending_in_it() {
        assert_eq!(find_row(FRAME, "parser"), None);
        assert_eq!(find_row(FRAME, "interbuild parser"), Some("nix_flake"));
    }

    #[test]
    fn missing_rows_report_absent() {
        assert!(!has_row(FRAME, "target", "other"));
        assert_eq!(find_row(FRAME, "space"), None);
    }
}
