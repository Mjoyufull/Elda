use std::fs;
use std::path::{Path, PathBuf};

use super::super::support::{make_executable, make_git_repo, run_git};

pub(super) fn create_system_make_repo(root: &Path, name: &str, output: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(repo_dir.join(name), format!("#!/bin/sh\necho '{output}'\n"))
        .expect("script binary should be written");
    make_executable(&repo_dir.join(name));
    fs::write(
        repo_dir.join(format!("lib{name}.so")),
        format!("library for {name}\n"),
    )
    .expect("shared library stub should be written");
    fs::write(
        repo_dir.join(format!("{name}.desktop")),
        format!("[Desktop Entry]\nType=Application\nName={name}\nExec=/usr/bin/{name}\n"),
    )
    .expect("desktop file should be written");
    fs::write(
        repo_dir.join("Makefile"),
        format!(
            "all:\n\ninstall:\n\tinstall -d $(DESTDIR)$(PREFIX)/bin\n\tinstall -d $(DESTDIR)$(PREFIX)/lib\n\tinstall -d $(DESTDIR)$(PREFIX)/share/applications\n\tinstall -m 0755 {name} $(DESTDIR)$(PREFIX)/bin/{name}\n\tinstall -m 0644 lib{name}.so $(DESTDIR)$(PREFIX)/lib/lib{name}.so\n\tinstall -m 0644 {name}.desktop $(DESTDIR)$(PREFIX)/share/applications/{name}.desktop\n"
        ),
    )
    .expect("makefile should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(super) fn update_system_make_repo(repo_dir: &Path, output: &str) {
    let name = repo_dir
        .file_name()
        .and_then(|value| value.to_str())
        .expect("repo dir should have a package name");
    fs::write(repo_dir.join(name), format!("#!/bin/sh\necho '{output}'\n"))
        .expect("script binary should be updated");
    make_executable(&repo_dir.join(name));
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "update system backend fixture"]);
}

pub(super) fn write_system_recipe(
    root: &Path,
    name: &str,
    repo_dir: &Path,
    version: &str,
    sysusers_entry: &str,
    tmpfiles_entry: &str,
) {
    write_system_recipe_with_provider_assets(
        root,
        name,
        repo_dir,
        version,
        sysusers_entry,
        tmpfiles_entry,
        "{}",
    );
}

pub(super) fn write_system_recipe_with_provider_assets(
    root: &Path,
    name: &str,
    repo_dir: &Path,
    version: &str,
    sysusers_entry: &str,
    tmpfiles_entry: &str,
    provider_assets: &str,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{repo}\",\n    branch = \"main\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  sysusers = {{ \"{sysusers_entry}\" }},\n  tmpfiles = {{ \"{tmpfiles_entry}\" }},\n  alternatives = {{\n    {{\n      name = \"toolctl\",\n      link = \"/usr/bin/toolctl\",\n      path = \"/usr/bin/{name}\",\n      priority = 50,\n    }},\n  }},\n  provider_assets = {provider_assets},\n}}\n",
            repo = repo_dir.display(),
        ),
    )
    .expect("system recipe should be written");
}
