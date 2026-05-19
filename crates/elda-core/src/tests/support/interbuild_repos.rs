use std::fs;
use std::path::{Path, PathBuf};

use super::repos::{make_git_repo, run_git};

pub(in crate::tests) fn create_git_nix_flake_make_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = create_git_make_tree(root, name);
    fs::write(
        repo_dir.join("flake.nix"),
        r#"{
  inputs.dep.url = "github:example/dep";
  outputs = { self, dep }: {
    packages.x86_64-linux.default = self;
  };
}
"#,
    )
    .expect("flake.nix should be written");
    fs::write(
        repo_dir.join("flake.lock"),
        r#"{"nodes":{"root":{"inputs":{"dep":"dep"}},"dep":{"locked":{"type":"github","owner":"example","repo":"dep","rev":"abc","narHash":"sha256-test"}}}}"#,
    )
    .expect("flake.lock should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add flake"]);

    repo_dir
}

pub(in crate::tests) fn create_git_gentoo_make_overlay(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(format!("{name}-overlay"));
    let package_dir = repo_dir.join("app-misc").join(name);
    fs::create_dir_all(&package_dir).expect("overlay package dir should exist");
    write_make_project_files(&package_dir, name, "make tool");
    fs::write(
        package_dir.join(format!("{name}-0.1.0.ebuild")),
        r#"EAPI="8"
DESCRIPTION="sample tool"
HOMEPAGE="https://example.invalid"
SRC_URI="https://example.invalid/source.tar.gz"
LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"
IUSE="wayland test"
DEPEND="dev-libs/libfoo"
RDEPEND="dev-libs/librun"
BDEPEND="virtual/pkgconfig"

src_compile() {
    emake
}

src_install() {
    emake DESTDIR="${D}" PREFIX=/usr install
}
"#,
    )
    .expect("ebuild should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

fn create_git_make_tree(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    write_make_project_files(&repo_dir, name, "make tool");
    make_git_repo(&repo_dir);

    repo_dir
}

fn write_make_project_files(repo_dir: &Path, name: &str, output: &str) {
    fs::write(
        repo_dir.join("Makefile"),
        format!(
            "all:\n\tchmod +x {name}\n\ninstall:\n\tinstall -d $(DESTDIR)$(PREFIX)/bin\n\tinstall -m 0755 {name} $(DESTDIR)$(PREFIX)/bin/{name}\n"
        ),
    )
    .expect("makefile should be written");
    fs::write(repo_dir.join(name), format!("#!/bin/sh\necho {output}\n"))
        .expect("make binary should be written");
}

pub(in crate::tests) fn create_git_aur_make_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = create_git_make_tree(root, name);
    fs::write(
        repo_dir.join("PKGBUILD"),
        format!(
            r#"pkgname={name}
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur tool"
url="https://example.invalid/{name}"
license=('MIT')
source=('https://example.invalid/{name}.tar.gz')
sha256sums=('SKIP')
depends=('glibc')
makedepends=('make')
checkdepends=('check')
optdepends=('docs: documentation')
provides=('{name}')
conflicts=()
replaces=()

build() {{
    make
}}

package() {{
    make DESTDIR="$pkgdir" PREFIX=/usr install
}}
"#
        ),
    )
    .expect("PKGBUILD should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add pkgbuild"]);

    repo_dir
}

pub(in crate::tests) fn create_git_aur_vcs_make_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = create_git_make_tree(root, name);
    fs::write(
        repo_dir.join("PKGBUILD"),
        format!(
            r#"pkgname={name}-git
pkgver=0.1.0
pkgrel=1
pkgdesc="sample aur vcs tool"
url="https://example.invalid/{name}"
license=('MIT')
source=('{name}::git+https://example.invalid/{name}.git#branch=main')
sha256sums=('SKIP')
depends=('glibc')
makedepends=('git' 'make')

pkgver() {{
    printf 0.1.0.r1.gabcdef0
}}

build() {{
    make
}}

package() {{
    make DESTDIR="$pkgdir" PREFIX=/usr install
}}
"#
        ),
    )
    .expect("PKGBUILD should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add vcs pkgbuild"]);

    repo_dir
}

pub(in crate::tests) fn create_git_xbps_make_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = create_git_make_tree(root, name);
    fs::write(
        repo_dir.join("template"),
        format!(
            r#"pkgname={name}
version=0.1.0
revision=1
short_desc="sample xbps tool"
homepage="https://example.invalid/{name}"
license="MIT"
distfiles="https://example.invalid/{name}.tar.gz"
checksum="SKIP"
depends="glibc"
makedepends="make"
hostmakedepends="pkg-config"
checkdepends="check"
provides="{name}"
archs="x86_64"

do_build() {{
    make
}}

do_install() {{
    make DESTDIR="$DESTDIR" PREFIX=/usr install
}}
"#
        ),
    )
    .expect("XBPS template should be written");
    run_git(&repo_dir, &["add", "."]);
    run_git(&repo_dir, &["commit", "-m", "add template"]);

    repo_dir
}
