use std::fs;

use tempfile::TempDir;

use super::{add_recipe, add_recipe_with_priority};

fn expect_single(result: crate::import::ImportResult) -> crate::import::ImportReport {
    match result {
        crate::import::ImportResult::Single(report) => report,
        crate::import::ImportResult::Bulk(_) => panic!("expected a single recipe import report"),
    }
}

fn path_str(path: &std::path::Path) -> &str {
    path.to_str().expect("test path should be valid UTF-8")
}

#[test]
fn add_recipe_can_scaffold_a_profile_recipe() {
    let tempdir = TempDir::new().expect("tempdir should exist");

    let report = expect_single(
        add_recipe(tempdir.path(), "yoka-core", Some("profile"))
            .expect("profile scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"profile\""));
    assert!(pkg_lua.contains("profile = {}"));
}

#[test]
fn add_recipe_detects_local_nix_flake_strategy() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("flake-source");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");

    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("flake scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"nix_flake\""));
    assert!(!pkg_lua.contains("branch = \"main\""));
}

#[test]
fn add_recipe_detects_local_gentoo_overlay_strategy() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let package_dir = tempdir.path().join("overlay/app-misc/sample");
    fs::create_dir_all(&package_dir).expect("package dir should exist");
    fs::write(package_dir.join("sample-1.0.ebuild"), "EAPI=8\n").expect("ebuild should exist");

    let source = tempdir.path().join("overlay");
    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("overlay scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"gentoo_overlay\""));
    assert!(pkg_lua.contains("package = \"app-misc/sample\""));
}

#[test]
fn add_recipe_priority_can_prefer_native_build_over_flake() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("mixed-source");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");
    fs::write(
        source.join("Makefile"),
        "all:
	true
",
    )
    .expect("makefile should exist");

    let priority = vec!["make".to_owned(), "nix_flake".to_owned()];
    let report = expect_single(
        add_recipe_with_priority(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
            &priority,
            &[],
        )
        .expect("mixed scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"git\""));
    assert!(pkg_lua.contains("branch = \"main\""));
}

#[test]
fn add_recipe_priority_can_prefer_pkgbuild_over_flake() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("aur-mixed-source");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");
    fs::write(
        source.join("PKGBUILD"),
        "pkgname=aur-mixed-source
pkgver=0.1.0
",
    )
    .expect("PKGBUILD should exist");

    let priority = vec!["aur_pkgbuild".to_owned(), "nix_flake".to_owned()];
    let report = expect_single(
        add_recipe_with_priority(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
            &priority,
            &[],
        )
        .expect("mixed scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"aur_pkgbuild\""));
    assert!(!pkg_lua.contains("branch = \"main\""));
}

#[test]
fn add_recipe_populates_pkgbuild_metadata_fields() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("aur-fields");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(
        source.join("PKGBUILD"),
        r#"pkgname=aur-fields
pkgver=1.2.3
pkgrel=4
pkgdesc="AUR field sample"
url="https://example.invalid/aur-fields"
license=('MIT' 'Apache-2.0')
depends=('glibc')
makedepends=('make')
checkdepends=('check')
provides=('aur-fields')
conflicts=('old-aur-fields')
replaces=('older-aur-fields')
source=('https://example.invalid/aur-fields.tar.gz')
"#,
    )
    .expect("PKGBUILD should exist");

    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("PKGBUILD scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains(r#"description = "AUR field sample""#));
    assert!(pkg_lua.contains(r#"licenses = { "MIT", "Apache-2.0" }"#));
    assert!(pkg_lua.contains(r#"upstream = "https://example.invalid/aur-fields""#));
    assert!(pkg_lua.contains(r#"version = "1.2.3""#));
    assert!(pkg_lua.contains("rel = 4"));
    assert!(pkg_lua.contains(r#"depends = { "glibc" }"#));
    assert!(pkg_lua.contains(r#"makedepends = { "make" }"#));
    assert!(pkg_lua.contains(r#"checkdepends = { "check" }"#));
    assert!(pkg_lua.contains(r#"provides = { "aur-fields" }"#));
    assert!(pkg_lua.contains(r#"conflicts = { "old-aur-fields" }"#));
    assert!(pkg_lua.contains(r#"replaces = { "older-aur-fields" }"#));
}

#[test]
fn add_recipe_populates_xbps_metadata_fields() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("xbps-fields");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(
        source.join("template"),
        r#"pkgname=xbps-fields
version=2.3.4
revision=5
short_desc="XBPS field sample"
homepage="https://example.invalid/xbps-fields"
license="MIT Apache-2.0"
depends="glibc"
makedepends="make"
checkdepends="check"
provides="xbps-fields"
conflicts="old-xbps-fields"
"#,
    )
    .expect("template should exist");

    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("XBPS scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains(r#"description = "XBPS field sample""#));
    assert!(pkg_lua.contains(r#"licenses = { "MIT", "Apache-2.0" }"#));
    assert!(pkg_lua.contains(r#"upstream = "https://example.invalid/xbps-fields""#));
    assert!(pkg_lua.contains(r#"version = "2.3.4""#));
    assert!(pkg_lua.contains("rel = 5"));
    assert!(pkg_lua.contains(r#"depends = { "glibc" }"#));
    assert!(pkg_lua.contains(r#"makedepends = { "make" }"#));
    assert!(pkg_lua.contains(r#"checkdepends = { "check" }"#));
    assert!(pkg_lua.contains(r#"provides = { "xbps-fields" }"#));
    assert!(pkg_lua.contains(r#"conflicts = { "old-xbps-fields" }"#));
}

#[test]
fn add_recipe_reports_detected_source_options_in_priority_order() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("mixed-options");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");
    fs::write(
        source.join("PKGBUILD"),
        "pkgname=mixed-options\npkgver=0.1.0\n",
    )
    .expect("PKGBUILD should exist");
    fs::write(source.join("Makefile"), "all:\n\ttrue\n").expect("makefile should exist");

    let priority = vec![
        "aur_pkgbuild".to_owned(),
        "make".to_owned(),
        "nix_flake".to_owned(),
    ];
    let report = expect_single(
        add_recipe_with_priority(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
            &priority,
            &[],
        )
        .expect("mixed scaffold should succeed"),
    );

    assert_eq!(report.source_options.len(), 3);
    assert_eq!(report.source_options[0].strategy, "aur_pkgbuild");
    assert!(report.source_options[0].selected);
    assert_eq!(report.source_options[1].strategy, "make");
    assert_eq!(report.source_options[2].strategy, "nix_flake");
    assert_eq!(
        report
            .selected_source_option
            .as_ref()
            .map(|option| option.strategy.as_str()),
        Some("aur_pkgbuild")
    );
}

#[test]
fn source_options_keep_source_selected_when_release_has_no_checksum() {
    let priority = vec!["git_release".to_owned(), "git_source".to_owned()];
    let opts = super::ImportOptions {
        strategy_priority: priority,
        release_binary_format_priority: Vec::new(),
        selected_source_option: None,
        git_ref: None,
        ..super::ImportOptions::default()
    };
    let options = super::strategy::source_options_with_priority(
        None,
        Some("https://example.invalid/not-github.git"),
        &opts,
    );

    assert_eq!(options.len(), 1);
    assert_eq!(options[0].strategy, "git_source");
    assert!(options[0].selected);
}

#[test]
fn add_recipe_source_option_can_select_lower_ranked_local_strategy() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("selectable-source");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");
    fs::write(
        source.join("PKGBUILD"),
        "pkgname=selectable-source\npkgver=0.1.0\n",
    )
    .expect("PKGBUILD should exist");

    let report = expect_single(
        super::add_recipe_with_options(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
            &super::ImportOptions {
                strategy_priority: vec!["nix_flake".to_owned(), "aur_pkgbuild".to_owned()],
                release_binary_format_priority: Vec::new(),
                selected_source_option: Some(2),
                git_ref: None,
                ..super::ImportOptions::default()
            },
        )
        .expect("selected source option should scaffold"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert_eq!(
        report
            .selected_source_option
            .as_ref()
            .expect("selected source option should be recorded")
            .index,
        2
    );
    assert_eq!(
        report
            .selected_source_option
            .as_ref()
            .expect("selected source option should be recorded")
            .strategy,
        "aur_pkgbuild"
    );
    assert!(pkg_lua.contains("kind = \"aur_pkgbuild\""));
}

#[test]
fn add_recipe_source_option_rejects_missing_option_index() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("flake-only");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("flake.nix"), "{ outputs = { self }: {}; }").expect("flake should exist");

    let error = super::add_recipe_with_options(
        tempdir.path().join("recipes").as_path(),
        path_str(&source),
        None,
        &super::ImportOptions {
            strategy_priority: vec!["nix_flake".to_owned()],
            release_binary_format_priority: Vec::new(),
            selected_source_option: Some(2),
            git_ref: None,
            ..super::ImportOptions::default()
        },
    )
    .expect_err("missing source option should fail");

    assert!(
        error
            .to_string()
            .contains("source option `2` is not available")
    );
}

#[test]
fn add_recipe_git_ref_option_renders_tag_instead_of_default_branch() {
    let tempdir = TempDir::new().expect("tempdir should exist");

    let report = expect_single(
        super::add_recipe_with_options(
            tempdir.path().join("recipes").as_path(),
            "https://example.invalid/tagged-tool.git",
            None,
            &super::ImportOptions {
                strategy_priority: vec!["git_source".to_owned()],
                release_binary_format_priority: Vec::new(),
                selected_source_option: None,
                git_ref: Some(super::GitRefRequest {
                    kind: super::GitRefKind::Tag,
                    value: "v1.2.3".to_owned(),
                }),
                ..super::ImportOptions::default()
            },
        )
        .expect("tagged scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains(r#"tag = "v1.2.3""#));
    assert!(!pkg_lua.contains("branch = \"main\""));
}
