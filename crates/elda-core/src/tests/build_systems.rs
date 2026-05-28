use super::support::*;
use super::*;

#[test]
fn direct_git_install_supports_regular_build_systems() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let cases = vec![
        (
            "make-tool",
            create_git_make_repo(tempdir.path(), "make-tool"),
            vec!["git", "make"],
            "make tool",
        ),
        (
            "cmake-tool",
            create_git_cmake_repo(tempdir.path(), "cmake-tool"),
            vec!["git", "cmake", "cc"],
            "cmake tool",
        ),
        (
            "mesonpkg-tool",
            create_git_meson_repo(tempdir.path(), "mesonpkg-tool"),
            vec!["git", "meson", "ninja", "cc"],
            "meson tool",
        ),
        (
            "go-tool",
            create_git_go_repo(tempdir.path(), "go-tool"),
            vec!["git", "go"],
            "go tool",
        ),
        (
            "zig-tool",
            create_git_zig_repo(tempdir.path(), "zig-tool"),
            vec!["git", "zig", "zig-build-self-test"],
            "zig tool",
        ),
        (
            "py-tool",
            create_git_python_repo(tempdir.path(), "py-tool"),
            vec!["git", "python3", "pip3"],
            "python tool",
        ),
        (
            "nimtool",
            create_git_nimble_repo(tempdir.path(), "nimtool"),
            vec!["git", "nimble", "nim", "nimble-build-self-test"],
            "nim tool",
        ),
    ];

    for (name, repo_dir, tools, expected_output) in cases {
        if !all_tools_available(&tools) {
            continue;
        }

        let repo_url = format!("file://{}", repo_dir.display());
        let report = run_from_root(
            tempdir.path(),
            CommandRequest::new(
                vec!["i".to_owned()],
                vec![repo_url],
                OutputMode::Json,
                false,
            ),
        )
        .expect("install should succeed");

        assert_eq!(report.area, "install");
        assert_eq!(
            run_installed_binary(tempdir.path(), &format!("/opt/elda/bin/{name}")),
            expected_output
        );

        run_from_root(
            tempdir.path(),
            CommandRequest::new(
                vec!["rm".to_owned()],
                vec![name.to_owned()],
                OutputMode::Json,
                false,
            ),
        )
        .expect("remove should succeed");
    }
}

#[test]
fn nix_flake_interbuild_installs_without_nix_cli() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_nix_flake_make_repo(tempdir.path(), "flake-tool");
    write_interbuild_recipe(tempdir.path(), "flake-tool", "nix_flake", &repo_dir, "");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["flake-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("nix_flake interbuild install should succeed");

    assert_eq!(report.area, "install");
    let details = report.details.as_ref().expect("details should exist");
    let interbuild = &details["installs"][0]["interbuild"];
    assert_eq!(interbuild["parser"], "nix_flake");
    assert_eq!(interbuild["lockfile"]["present"], true);
    assert_eq!(interbuild["lockfile"]["locked_inputs"], 1);
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/flake-tool"),
        "make tool"
    );
}

#[test]
fn gentoo_overlay_interbuild_installs_selected_ebuild_package() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_gentoo_make_overlay(tempdir.path(), "gentoo-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "gentoo-tool",
        "gentoo_overlay",
        &repo_dir,
        "    package = \"app-misc/gentoo-tool\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["gentoo-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("gentoo_overlay interbuild install should succeed");
    let details = report.details.as_ref().expect("details should exist");
    let gentoo = &details["installs"][0]["interbuild"]["gentoo"];
    assert_eq!(gentoo["eapi"], "8");
    assert_eq!(gentoo["description"], "sample tool");
    assert_eq!(gentoo["homepage"], "https://example.invalid");
    assert_eq!(gentoo["license"][0], "MIT");
    assert_eq!(
        gentoo["src_uri"][0],
        "https://example.invalid/source.tar.gz"
    );
    assert_eq!(gentoo["slot"], "0");
    assert_eq!(gentoo["depend"][0], "dev-libs/libfoo");
    assert_eq!(gentoo["rdepend"][0], "dev-libs/librun");
    assert_eq!(gentoo["iuse"][0], "wayland");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/gentoo-tool"),
        "make tool"
    );
}

#[test]
fn gentoo_overlay_interbuild_uses_git_r3_upstream_source() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let upstream_dir = create_git_make_repo(tempdir.path(), "gentoo-git-tool");
    let overlay_dir = tempdir.path().join("gentoo-git-tool-overlay");
    let package_dir = overlay_dir.join("app-misc").join("gentoo-git-tool");
    fs::create_dir_all(&package_dir).expect("overlay package dir should exist");
    fs::write(
        package_dir.join("gentoo-git-tool-9999.ebuild"),
        format!(
            r#"EAPI="8"

inherit git-r3

DESCRIPTION="sample git-r3 tool"
HOMEPAGE="https://example.invalid/gentoo-git-tool"
EGIT_REPO_URI="file://{}"
LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"
"#,
            upstream_dir.display()
        ),
    )
    .expect("ebuild should be written");
    run_git(&overlay_dir, &["init", "-b", "main"]);
    run_git(&overlay_dir, &["add", "."]);
    run_git(&overlay_dir, &["commit", "-m", "add git-r3 ebuild"]);
    write_interbuild_recipe(
        tempdir.path(),
        "gentoo-git-tool",
        "gentoo_overlay",
        &overlay_dir,
        "    package = \"app-misc/gentoo-git-tool\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["gentoo-git-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("gentoo_overlay git-r3 interbuild install should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let gentoo = &details["installs"][0]["interbuild"]["gentoo"];
    assert_eq!(gentoo["inherited_eclasses"][0], "git-r3");
    assert_eq!(
        gentoo["src_uri"][0],
        format!("file://{}", upstream_dir.display())
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/gentoo-git-tool"),
        "make tool"
    );
}

#[test]
fn gentoo_overlay_interbuild_fails_closed_for_unsupported_eapi() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_gentoo_make_overlay(tempdir.path(), "future-tool");
    std::fs::write(
        repo_dir
            .join("app-misc")
            .join("future-tool")
            .join("future-tool-0.1.0.ebuild"),
        "EAPI=\"9\"\nDESCRIPTION=\"future\"\nHOMEPAGE=\"https://example.invalid\"\nSRC_URI=\"https://example.invalid/source.tar.gz\"\n",
    )
    .expect("ebuild should be updated");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "unsupported eapi"]);
    write_interbuild_recipe(
        tempdir.path(),
        "future-tool",
        "gentoo_overlay",
        &repo_dir,
        "    package = \"app-misc/future-tool\",\n",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["future-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("unsupported EAPI should fail closed");

    assert!(error.to_string().contains("supported EAPI is `8`"));
}

#[test]
fn xbps_template_interbuild_uses_distfile_archive_source() {
    if !all_tools_available(&["git", "make", "tar"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let source_tree = tempdir.path().join("xbps-archive-tool-0.1.0");
    fs::create_dir_all(&source_tree).expect("source tree should exist");
    fs::write(
        source_tree.join("Makefile"),
        "all:\n\tchmod +x xbps-archive-tool\n\ninstall:\n\tinstall -d $(DESTDIR)$(PREFIX)/bin\n\tinstall -m 0755 xbps-archive-tool $(DESTDIR)$(PREFIX)/bin/xbps-archive-tool\n",
    )
    .expect("makefile should be written");
    fs::write(
        source_tree.join("xbps-archive-tool"),
        "#!/bin/sh\necho xbps archive tool\n",
    )
    .expect("binary should be written");
    let archive_path = tempdir.path().join("xbps-archive-tool.tar.gz");
    let status = std::process::Command::new("tar")
        .current_dir(tempdir.path())
        .args([
            "-czf",
            archive_path
                .to_str()
                .expect("test archive path should be UTF-8"),
            "xbps-archive-tool-0.1.0",
        ])
        .status()
        .expect("tar should launch");
    assert!(status.success(), "tar should create test archive");

    let repo_dir = tempdir.path().join("void-packages");
    let package_dir = repo_dir.join("srcpkgs").join("xbps-archive-tool");
    fs::create_dir_all(&package_dir).expect("template dir should exist");
    fs::write(
        package_dir.join("template"),
        format!(
            r#"pkgname=xbps-archive-tool
version=0.1.0
revision=1
build_style=gnu-makefile
short_desc="archive-backed xbps tool"
homepage="https://example.invalid/xbps-archive-tool"
license="MIT"
distfiles="file://{}"
checksum=SKIP
"#,
            archive_path.display()
        ),
    )
    .expect("template should be written");
    run_git(&repo_dir, &["init", "-b", "main"]);
    run_git(&repo_dir, &["config", "user.email", "elda@example.invalid"]);
    run_git(&repo_dir, &["config", "user.name", "Elda Tests"]);
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add xbps template"]);
    write_interbuild_recipe(
        tempdir.path(),
        "xbps-archive-tool",
        "xbps_template",
        &repo_dir,
        "    package = \"srcpkgs/xbps-archive-tool\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["xbps-archive-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("xbps_template archive interbuild install should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let xbps = &details["installs"][0]["interbuild"]["xbps"];
    assert_eq!(xbps["build_style"], "gnu-makefile");
    assert_eq!(
        xbps["distfiles"][0],
        format!("file://{}", archive_path.display())
    );
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/xbps-archive-tool"),
        "xbps archive tool"
    );
}

#[test]
fn metadata_add_generates_reviewable_metadata_without_installing() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "add-only-tool");
    let repo_url = format!("file://{}", repo_dir.display());

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["a".to_owned()],
            vec![repo_url],
            OutputMode::Json,
            false,
        ),
    )
    .expect("metadata add should succeed");

    assert_eq!(report.area, "metadata");
    let details = report.details.as_ref().expect("details should exist");
    let target = &details["metadata_add"]["targets"][0];
    assert_eq!(target["recipe_name"], "add-only-tool");
    assert_eq!(target["selected_source_kind"], "git");
    assert_eq!(target["generated"], true);
    assert_eq!(target["publish_ready"], false);
    assert!(
        target["missing_fields"]
            .as_array()
            .expect("missing fields should be an array")
            .iter()
            .any(|field| field == "description")
    );
    assert!(
        tempdir
            .path()
            .join("etc/elda/recipes/add-only-tool/pkg.lua")
            .is_file()
    );
    assert!(!tempdir.path().join("opt/elda/bin/add-only-tool").exists());
}

#[test]
fn metadata_add_uses_detected_nix_flake_strategy_for_local_source() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_nix_flake_make_repo(tempdir.path(), "add-flake-tool");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["add".to_owned()],
            vec![repo_dir.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("metadata add should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let target = &details["metadata_add"]["targets"][0];
    assert_eq!(target["selected_source_kind"], "nix_flake");
    assert_eq!(target["persisted_source_kind"], "interbuild");
    assert_eq!(target["source_options"][0]["strategy"], "nix_flake");
    assert_eq!(target["selected_source_option"]["strategy"], "nix_flake");
    assert!(!tempdir.path().join("opt/elda/bin/add-flake-tool").exists());
}

#[test]
fn metadata_add_git_ref_flag_renders_requested_tag() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "add-tag-tool");
    run_git(&repo_dir, &["tag", "v1.2.3"]);
    let repo_url = format!("file://{}", repo_dir.display());

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["add".to_owned()],
            vec![repo_url, "--to-tag=v1.2.3".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("metadata add should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let target = &details["metadata_add"]["targets"][0];
    assert_eq!(target["selected_source_kind"], "git");
    let pkg_lua = fs::read_to_string(tempdir.path().join("etc/elda/recipes/add-tag-tool/pkg.lua"))
        .expect("pkg.lua should exist");
    assert!(pkg_lua.contains(r#"tag = "v1.2.3""#));
    assert!(!pkg_lua.contains("branch = \"main\""));
}

#[test]
fn metadata_add_source_option_selects_ranked_strategy() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_nix_flake_make_repo(tempdir.path(), "add-select-tool");
    fs::write(
        repo_dir.join("PKGBUILD"),
        "pkgname=add-select-tool\npkgver=0.1.0\npkgrel=1\npkgdesc=\"selectable\"\n",
    )
    .expect("PKGBUILD should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add pkgbuild"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["add".to_owned()],
            vec![
                repo_dir.display().to_string(),
                "--source-option=2".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("metadata add should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let target = &details["metadata_add"]["targets"][0];
    assert_eq!(target["selected_source_kind"], "aur_pkgbuild");
    assert_eq!(target["selected_source_option"]["index"], 2);
    assert_eq!(target["selected_source_option"]["strategy"], "aur_pkgbuild");
    let pkg_lua = fs::read_to_string(
        tempdir
            .path()
            .join("etc/elda/recipes/add-select-tool/pkg.lua"),
    )
    .expect("pkg.lua should exist");
    assert!(pkg_lua.contains("kind = \"aur_pkgbuild\""));
}

#[test]
fn aur_pkgbuild_interbuild_installs_make_project() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-tool");
    write_interbuild_recipe(tempdir.path(), "aur-tool", "aur_pkgbuild", &repo_dir, "");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("aur_pkgbuild interbuild install should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let aur = &details["installs"][0]["interbuild"]["aur"];
    assert_eq!(aur["pkgdesc"], "sample aur tool");
    assert_eq!(aur["license"][0], "MIT");
    assert_eq!(aur["makedepends"][0], "make");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/aur-tool"),
        "make tool"
    );
}

#[test]
fn xbps_template_interbuild_installs_make_project() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_xbps_make_repo(tempdir.path(), "xbps-tool");
    write_interbuild_recipe(tempdir.path(), "xbps-tool", "xbps_template", &repo_dir, "");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["xbps-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("xbps_template interbuild install should succeed");

    let details = report.details.as_ref().expect("details should exist");
    let xbps = &details["installs"][0]["interbuild"]["xbps"];
    assert_eq!(xbps["short_desc"], "sample xbps tool");
    assert_eq!(xbps["license"][0], "MIT");
    assert_eq!(xbps["hostmakedepends"][0], "pkg-config");
    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/xbps-tool"),
        "make tool"
    );
}

#[test]
fn aur_pkgbuild_report_surfaces_bounded_build_commands() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-report-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "aur-report-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-report-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("AUR interbuild install should succeed");

    let aur =
        &report.details.as_ref().expect("details should exist")["installs"][0]["interbuild"]["aur"];
    assert_eq!(aur["optdepends"][0], "docs: documentation");
    assert_eq!(aur["phase_commands"][0]["phase"], "build");
    assert_eq!(aur["phase_commands"][0]["commands"][0], "make");
    assert_eq!(
        aur["phase_commands"][1]["commands"][0],
        "make DESTDIR=\"$pkgdir\" PREFIX=/usr install"
    );
}

#[test]
fn aur_pkgbuild_report_surfaces_vcs_metadata_without_fetching_vcs_source() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_vcs_make_repo(tempdir.path(), "aur-vcs-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "aur-vcs-tool",
        "aur_pkgbuild",
        &repo_dir,
        "    pkgname = \"aur-vcs-tool-git\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-vcs-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("AUR VCS interbuild install should succeed");

    let aur =
        &report.details.as_ref().expect("details should exist")["installs"][0]["interbuild"]["aur"];
    assert_eq!(aur["pkgname"], "aur-vcs-tool-git");
    assert_eq!(aur["pkgver_function"], true);
    assert_eq!(
        aur["vcs_sources"][0],
        "aur-vcs-tool::git+https://example.invalid/aur-vcs-tool.git#branch=main"
    );
}

#[test]
fn xbps_template_report_surfaces_bounded_build_commands() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_xbps_make_repo(tempdir.path(), "xbps-report-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "xbps-report-tool",
        "xbps_template",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["xbps-report-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("XBPS interbuild install should succeed");

    let xbps = &report.details.as_ref().expect("details should exist")["installs"][0]["interbuild"]
        ["xbps"];
    assert_eq!(xbps["hostmakedepends"][0], "pkg-config");
    assert_eq!(xbps["phase_commands"][0]["phase"], "do_build");
    assert_eq!(xbps["phase_commands"][0]["commands"][0], "make");
    assert_eq!(
        xbps["phase_commands"][1]["commands"][0],
        "make DESTDIR=\"$DESTDIR\" PREFIX=/usr install"
    );
}

#[test]
fn gentoo_overlay_report_surfaces_bounded_build_commands() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_gentoo_make_overlay(tempdir.path(), "gentoo-report-tool");
    write_interbuild_recipe(
        tempdir.path(),
        "gentoo-report-tool",
        "gentoo_overlay",
        &repo_dir,
        "    package = \"app-misc/gentoo-report-tool\",\n",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["gentoo-report-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("Gentoo interbuild install should succeed");

    let gentoo = &report.details.as_ref().expect("details should exist")["installs"][0]["interbuild"]
        ["gentoo"];
    assert_eq!(gentoo["phase_commands"][0]["phase"], "src_compile");
    assert_eq!(gentoo["phase_commands"][0]["commands"][0], "emake");
    assert_eq!(
        gentoo["phase_commands"][1]["commands"][0],
        "emake DESTDIR=\"${D}\" PREFIX=/usr install"
    );
}

#[test]
fn aur_pkgbuild_fails_closed_on_unsupported_shell_expansion() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-shell-tool");
    std::fs::write(
        repo_dir.join("PKGBUILD"),
        r#"pkgname=aur-shell-tool
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur tool"
url="https://example.invalid/aur-shell-tool"
license=('MIT')
source=('https://example.invalid/aur-shell-tool.tar.gz')
sha256sums=('SKIP')

build() {
    make $(nproc)
}
"#,
    )
    .expect("PKGBUILD should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "unsupported shell expansion"]);
    write_interbuild_recipe(
        tempdir.path(),
        "aur-shell-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-shell-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("unsupported PKGBUILD shell expansion should fail closed");

    assert!(error.to_string().contains("unsupported shell in"));
}

#[test]
fn aur_pkgbuild_fails_closed_when_source_checksum_counts_do_not_match() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-checksum-tool");
    std::fs::write(
        repo_dir.join("PKGBUILD"),
        r#"pkgname=aur-checksum-tool
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur tool"
url="https://example.invalid/aur-checksum-tool"
license=('MIT')
source=('https://example.invalid/one.tar.gz' 'https://example.invalid/two.tar.gz')
sha256sums=('SKIP')

build() {
    make
}
"#,
    )
    .expect("PKGBUILD should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "mismatched checksum count"]);
    write_interbuild_recipe(
        tempdir.path(),
        "aur-checksum-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-checksum-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("mismatched AUR checksum count should fail closed");

    assert!(error.to_string().contains("source entry(s)"));
    assert!(error.to_string().contains("checksum entry(s)"));
}

#[test]
fn aur_pkgbuild_accepts_matching_arch_specific_source_checksums() {
    if !all_tools_available(&["git", "make"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-arch-source-tool");
    std::fs::write(
        repo_dir.join("PKGBUILD"),
        r#"pkgname=aur-arch-source-tool
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur tool"
url="https://example.invalid/aur-arch-source-tool"
license=('MIT')
source=('https://example.invalid/common.tar.gz')
source_x86_64=('https://example.invalid/x86_64.tar.gz')
sha256sums=('SKIP')
sha256sums_x86_64=('SKIP')

build() {
    make
}

package() {
    make DESTDIR="$pkgdir" PREFIX=/usr install
}
"#,
    )
    .expect("PKGBUILD should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "arch source checksums"]);
    write_interbuild_recipe(
        tempdir.path(),
        "aur-arch-source-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-arch-source-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("matching arch-specific source checksums should be accepted");

    let aur =
        &report.details.as_ref().expect("details should exist")["installs"][0]["interbuild"]["aur"];
    assert_eq!(aur["arch_sources"][0]["arch"], "x86_64");
    assert_eq!(
        aur["arch_sources"][0]["source"][0],
        "https://example.invalid/x86_64.tar.gz"
    );
}

#[test]
fn aur_pkgbuild_fails_closed_when_arch_source_checksum_counts_do_not_match() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_aur_make_repo(tempdir.path(), "aur-arch-checksum-tool");
    std::fs::write(
        repo_dir.join("PKGBUILD"),
        r#"pkgname=aur-arch-checksum-tool
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur tool"
url="https://example.invalid/aur-arch-checksum-tool"
license=('MIT')
source=('https://example.invalid/common.tar.gz')
source_x86_64=('https://example.invalid/one.tar.gz' 'https://example.invalid/two.tar.gz')
sha256sums=('SKIP')
sha256sums_x86_64=('SKIP')

build() {
    make
}
"#,
    )
    .expect("PKGBUILD should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(
        &repo_dir,
        &["commit", "-m", "mismatched arch checksum count"],
    );
    write_interbuild_recipe(
        tempdir.path(),
        "aur-arch-checksum-tool",
        "aur_pkgbuild",
        &repo_dir,
        "",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["aur-arch-checksum-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("mismatched arch-specific checksum count should fail closed");

    assert!(error.to_string().contains("source_x86_64"));
    assert!(error.to_string().contains("checksum entry(s)"));
}

#[test]
fn xbps_template_fails_closed_on_unsupported_shell_expansion() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_xbps_make_repo(tempdir.path(), "xbps-shell-tool");
    std::fs::write(
        repo_dir.join("template"),
        r#"pkgname=xbps-shell-tool
version=0.1.0
revision=1
short_desc="sample xbps tool"
homepage="https://example.invalid/xbps-shell-tool"
license="MIT"
distfiles="https://example.invalid/xbps-shell-tool.tar.gz"
checksum="SKIP"

do_build() {
    make $(nproc)
}
"#,
    )
    .expect("template should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "unsupported shell expansion"]);
    write_interbuild_recipe(
        tempdir.path(),
        "xbps-shell-tool",
        "xbps_template",
        &repo_dir,
        "",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["xbps-shell-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("unsupported XBPS shell expansion should fail closed");

    assert!(error.to_string().contains("unsupported shell in"));
}

#[test]
fn xbps_template_fails_closed_when_distfile_checksum_counts_do_not_match() {
    if !all_tools_available(&["git"]) {
        return;
    }

    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_xbps_make_repo(tempdir.path(), "xbps-checksum-tool");
    std::fs::write(
        repo_dir.join("template"),
        r#"pkgname=xbps-checksum-tool
version=0.1.0
revision=1
short_desc="sample xbps tool"
homepage="https://example.invalid/xbps-checksum-tool"
license="MIT"
distfiles="https://example.invalid/one.tar.gz https://example.invalid/two.tar.gz"
checksum="SKIP"

do_build() {
    make
}
"#,
    )
    .expect("template should be rewritten");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "mismatched checksum count"]);
    write_interbuild_recipe(
        tempdir.path(),
        "xbps-checksum-tool",
        "xbps_template",
        &repo_dir,
        "",
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ig".to_owned()],
            vec!["xbps-checksum-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("mismatched XBPS checksum count should fail closed");

    assert!(error.to_string().contains("distfile entry(s)"));
    assert!(error.to_string().contains("checksum entry(s)"));
}
