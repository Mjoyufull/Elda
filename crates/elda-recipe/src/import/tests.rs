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
fn add_recipe_records_cargo_build_intent() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("cargo-tool");
    fs::create_dir_all(source.join("src/bin")).expect("source dirs should exist");
    fs::write(
        source.join("Cargo.toml"),
        "[package]\nname = \"cargo-tool\"\nversion = \"0.1.0\"\n[[bin]]\nname = \"admin\"\n",
    )
    .expect("Cargo.toml should exist");
    fs::write(source.join("src/main.rs"), "fn main() {}\n").expect("main should exist");
    fs::write(source.join("src/bin/side.rs"), "fn main() {}\n").expect("side should exist");

    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("cargo scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains(r#"build = {"#));
    assert!(pkg_lua.contains(r#"system = "cargo""#));
    assert!(pkg_lua.contains(r#"bins = { "admin", "cargo-tool", "side" }"#));
}

#[test]
fn add_recipe_records_python_script_build_intent() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("python-tool");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(
        source.join("pyproject.toml"),
        "[project]\nname = \"python-tool\"\nversion = \"0.1.0\"\n[project.scripts]\npytool = \"tool:main\"\n",
    )
    .expect("pyproject should exist");

    let report = expect_single(
        add_recipe(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
            None,
        )
        .expect("python scaffold should succeed"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains(r#"build = {"#));
    assert!(pkg_lua.contains(r#"system = "python""#));
    assert!(pkg_lua.contains(r#"bins = { "pytool" }"#));
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
fn add_recipe_prefers_expanded_srcinfo_metadata_for_pkgbuilds() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("aur-srcinfo-fields");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(
        source.join("PKGBUILD"),
        r#"_owner=example
pkgname=aur-srcinfo-fields
pkgver=1.2.3
pkgrel=4
pkgdesc="AUR field sample (prebuilt binary)"
url="https://example.invalid/${_owner}/${pkgname}"
license=('MIT')
depends=('glibc' 'libgcc')
source=('https://example.invalid/aur-srcinfo-fields.tar.gz')
"#,
    )
    .expect("PKGBUILD should exist");
    fs::write(
        source.join(".SRCINFO"),
        "pkgbase = aur-srcinfo-fields\n\tpkgdesc = AUR field sample (prebuilt binary)\n\tpkgver = 1.2.3\n\tpkgrel = 4\n\turl = https://example.invalid/example/aur-srcinfo-fields\n\tlicense = MIT\n\tdepends = glibc\n\tdepends = libgcc\n",
    )
    .expect(".SRCINFO should exist");

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

    assert!(pkg_lua.contains(r#"description = "AUR field sample (prebuilt binary)""#));
    assert!(pkg_lua.contains(r#"upstream = "https://example.invalid/example/aur-srcinfo-fields""#));
    assert!(pkg_lua.contains(r#"depends = { "glibc", "libgcc" }"#));
}

#[test]
fn add_recipe_emits_binary_lane_for_srcinfo_archive() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("aur-fields-bin");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("PKGBUILD"), "pkgname=aur-fields-bin\n").expect("PKGBUILD should exist");
    fs::write(
        source.join(".SRCINFO"),
        "pkgbase = aur-fields-bin\n\tpkgname = aur-fields-bin\n\tpkgver = 1.2.3\n\tpkgrel = 1\n\tpkgdesc = Demo binary\n\turl = https://example.invalid/demo\n\tlicense = MIT\n\tprovides = demo=1.2.3\n\tsource_x86_64 = demo.tar.xz::https://example.invalid/demo-x86_64.tar.xz\n\tsha256sums_x86_64 = aaaa\n",
    )
    .expect(".SRCINFO should exist");

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

    assert!(pkg_lua.contains(r#"default_lane = "binary""#));
    assert!(pkg_lua.contains(r#"kind = "aur_pkgbuild""#));
    assert!(pkg_lua.contains(r#"kind = "url_archive""#));
    assert!(pkg_lua.contains(r#"url = "https://example.invalid/demo-x86_64.tar.xz""#));
    assert!(pkg_lua.contains(r#"binary = "demo""#));
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
    let source = tempdir.path().join("tagged-tool");
    fs::create_dir_all(&source).expect("source dir should exist");

    let report = expect_single(
        super::add_recipe_with_options(
            tempdir.path().join("recipes").as_path(),
            path_str(&source),
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

#[test]
fn arch_package_page_maps_to_official_packaging_git_source() {
    assert_eq!(
        super::detect::arch_package_source_url(
            "https://archlinux.org/packages/extra-testing/x86_64/anki/"
        )
        .as_deref(),
        Some("https://gitlab.archlinux.org/archlinux/packaging/packages/anki.git")
    );
    assert!(
        super::detect::arch_package_source_url("https://archlinux.org/packages/extra/x86_64/")
            .is_none()
    );
}
