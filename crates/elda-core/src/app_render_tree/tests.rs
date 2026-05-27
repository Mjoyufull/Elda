use super::{Frame, FrameFooter, Glyph, HighlightKind, TreeStyle};

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

#[test]
fn highlighted_rows_keep_frame_structure_plain_without_color() {
    let mut frame = Frame::new("Result");
    frame.highlighted("installed", "foot", HighlightKind::Identity);

    let rendered = frame.render(TreeStyle::Unicode);
    assert!(rendered.contains("│  installed foot"));
}
