use elda_types::{
    ArtifactAcquisition, ArtifactEntry, ArtifactEntryKind, ArtifactFormat, ArtifactSurvey,
};
use tempfile::TempDir;

use super::{render_pkg_lua, strip_leading, survey_summary, write_local_artifact_recipe};
use crate::import::model::ImportOptions;

fn options() -> ImportOptions {
    ImportOptions {
        strategy_priority: Vec::new(),
        release_binary_format_priority: Vec::new(),
        selected_source_option: None,
        git_ref: None,
        replace: false,
        exclude: Vec::new(),
        acquisition_url: None,
    }
}

fn survey() -> ArtifactSurvey {
    ArtifactSurvey {
        source_path: "/tmp/delta-linux-x86_64.tar.gz".to_owned(),
        file_name: "delta-linux-x86_64.tar.gz".to_owned(),
        format: ArtifactFormat::TarGz,
        sha256: "a".repeat(64),
        size: 1024,
        strip_components: 1,
        name: Some("delta".to_owned()),
        version: Some("0.18.2".to_owned()),
        architecture: "amd64".to_owned(),
        appimage_payload: None,
        entries: vec![
            ArtifactEntry {
                path: "delta-0.18.2-linux/delta".to_owned(),
                kind: ArtifactEntryKind::Executable,
                size: 512,
                executable: true,
            },
            ArtifactEntry {
                path: "delta-0.18.2-linux/delta.1".to_owned(),
                kind: ArtifactEntryKind::ManPage,
                size: 32,
                executable: false,
            },
            ArtifactEntry {
                path: "delta-0.18.2-linux/delta.exe".to_owned(),
                kind: ArtifactEntryKind::ForeignPlatform,
                size: 8,
                executable: true,
            },
        ],
    }
}

#[test]
fn url_acquisition_pins_source_digest_and_strips_the_root() {
    let rendered = render_pkg_lua(
        "delta",
        &survey(),
        &ArtifactAcquisition::Url("https://example.test/delta.tar.gz".to_owned()),
    );

    assert!(rendered.contains("pkg = {"));
    assert!(rendered.contains("name = \"delta\""));
    assert!(rendered.contains("version = \"0.18.2\""));
    assert!(rendered.contains("kind = \"url_archive\""));
    assert!(rendered.contains("url = \"https://example.test/delta.tar.gz\""));
    assert!(rendered.contains(&format!("sha256 = \"{}\"", "a".repeat(64))));
    assert!(rendered.contains("strip_components = 1"));
    // The archive root is stripped at stage time, so the recorded binary must not carry it.
    assert!(rendered.contains("binary = \"delta\""), "got: {rendered}");
}

#[test]
fn local_only_acquisition_says_it_cannot_upgrade() {
    let rendered = render_pkg_lua("delta", &survey(), &ArtifactAcquisition::LocalOnly);
    assert!(rendered.contains("cannot be re-fetched or upgraded"));
    assert!(rendered.contains(r#"url = "file:///tmp/delta-linux-x86_64.tar.gz""#));
}

#[test]
fn raw_elf_and_dwarfs_appimage_use_safe_launcher_metadata() {
    let mut raw = survey();
    raw.format = ArtifactFormat::Elf;
    let rendered = render_pkg_lua("delta", &raw, &ArtifactAcquisition::LocalOnly);
    assert!(rendered.contains("rename = \"delta\""));
    assert!(!rendered.contains("binary = \"delta-linux-x86_64.tar.gz\""));

    let mut appimage = survey();
    appimage.format = ArtifactFormat::AppImage;
    appimage.strip_components = 0;
    appimage.appimage_payload = Some("dwarfs".to_owned());
    let rendered = render_pkg_lua("delta", &appimage, &ArtifactAcquisition::LocalOnly);
    assert!(rendered.contains("binary = \"delta\""));
    assert!(rendered.contains("integration = \"none\""));
    assert!(!rendered.contains("strip_components"));
}

#[test]
fn writing_twice_requires_replace() {
    let dir = TempDir::new().expect("tempdir");
    let acquisition = ArtifactAcquisition::Url("https://example.test/d.tar.gz".to_owned());

    let report = write_local_artifact_recipe(dir.path(), &survey(), &acquisition, &options())
        .expect("first write should succeed");
    assert_eq!(report.recipe_name, "delta");
    assert!(report.generated_pkg_lua);
    assert!(report.recipe_dir.join("pkg.lua").is_file());
    assert!(report.recipe_dir.join("artifact-survey.json").is_file());

    let again = write_local_artifact_recipe(dir.path(), &survey(), &acquisition, &options());
    assert!(again.is_err(), "second write must not clobber silently");

    let mut replacing = options();
    replacing.replace = true;
    write_local_artifact_recipe(dir.path(), &survey(), &acquisition, &replacing)
        .expect("--replace should overwrite");
}

#[test]
fn a_survey_without_a_name_is_refused() {
    let mut nameless = survey();
    nameless.name = None;
    let err = write_local_artifact_recipe(
        TempDir::new().expect("tempdir").path(),
        &nameless,
        &ArtifactAcquisition::LocalOnly,
        &options(),
    )
    .expect_err("no inferred name must fail closed");
    assert!(format!("{err}").contains("could not infer a package name"));
}

#[test]
fn strip_leading_removes_only_the_requested_depth() {
    assert_eq!(strip_leading("root/bin/tool", 1), "bin/tool");
    assert_eq!(strip_leading("root/bin/tool", 2), "tool");
    assert_eq!(strip_leading("tool", 1), "tool");
    assert_eq!(strip_leading("root/tool", 0), "root/tool");
}

#[test]
fn summary_counts_what_the_operator_cares_about() {
    let text = survey_summary(&survey());
    assert!(text.contains("3 member(s)"));
    assert!(text.contains("1 executable"));
    assert!(text.contains("1 man"));
    assert!(text.contains("1 dropped"));
}
