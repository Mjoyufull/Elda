mod cases;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::app_render_tree::{TreeStyle, scoped_tree_style_for_tests};
use crate::render_human;
use crate::render_style::{scoped_force_color_for_tests, scoped_no_color_for_tests};

use cases::{SNAPSHOT_REQUIRED, snapshot_cases};

fn allows_scan_header(name: &str) -> bool {
    matches!(name, "state-ls" | "state-list" | "recipe-catalog")
}

#[test]
fn surface_snapshots_are_square_and_plain_without_ansi() {
    scoped_no_color_for_tests(true, || {
        scoped_tree_style_for_tests(Some(TreeStyle::Ascii), || {
            for (name, report) in snapshot_cases() {
                let rendered = render_human(&report);
                assert!(
                    !rendered.contains("\x1b["),
                    "{name} leaked ansi despite NO_COLOR:\n{rendered}"
                );
                assert!(
                    !rendered.contains("├─ ") && !rendered.contains("┌─ "),
                    "{name} ignored ASCII tree mode:\n{rendered}"
                );
                assert!(
                    !starts_with_scan_header(&rendered) || allows_scan_header(name),
                    "{name} leaked legacy area header:\n{rendered}"
                );
                assert!(
                    !rendered.contains("{\n") && !rendered.contains("\"details\""),
                    "{name} fell back to JSON dump:\n{rendered}"
                );
            }
        });
    });
}

fn starts_with_scan_header(rendered: &str) -> bool {
    rendered
        .lines()
        .next()
        .is_some_and(|line| line.ends_with(": ok"))
}

#[test]
fn surface_snapshots_color_lane_applies_semantic_ansi() {
    scoped_no_color_for_tests(false, || {
        scoped_force_color_for_tests(true, || {
            use crate::render_style::{paint, palette};
            let colored = paint("fsel", palette::IDENTITY, true);
            assert!(
                colored.contains("\x1b["),
                "forced color lane should emit ANSI escapes: {colored}"
            );
        });
    });
}

#[test]
fn surface_snapshots_write_review_transcripts() {
    scoped_no_color_for_tests(true, || {
        scoped_tree_style_for_tests(Some(TreeStyle::Ascii), || {
            let output_dir = snapshot_output_dir();
            fs::create_dir_all(&output_dir).expect("snapshot output dir should be created");
            for (name, report) in snapshot_cases() {
                let rendered = render_human(&report);
                let path = output_dir.join(format!("{name}.txt"));
                fs::write(path, rendered).expect("snapshot transcript should be written");
            }
        });
    });
}

fn snapshot_output_dir() -> PathBuf {
    std::env::var_os("ELDA_SURFACE_SNAPSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/elda-surface-snapshots"))
}

#[test]
fn surface_snapshots_cover_every_audit_family() {
    let cases = snapshot_cases();
    let names: HashSet<_> = cases.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names.len(),
        cases.len(),
        "duplicate snapshot case names detected"
    );
    assert_eq!(
        names.len(),
        SNAPSHOT_REQUIRED.len(),
        "snapshot case count drifted from SNAPSHOT_REQUIRED"
    );

    for required in SNAPSHOT_REQUIRED {
        assert!(names.contains(required), "missing {required} snapshot");
    }
}
