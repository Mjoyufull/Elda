use super::*;

#[test]
fn rc_add_scaffolds_new_recipe() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let report = add_recipe(tempdir.path(), "hello", None).expect("add should succeed");

    assert!(report.generated_pkg_lua);
    assert!(tempdir.path().join("hello/pkg.lua").exists());
}

#[test]
fn rc_add_imports_pkgit_files() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("legacy-src");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("pkgdeps"), "https://example.invalid/dep.git\n")
        .expect("pkgdeps should be written");
    fs::write(source.join("bldit"), "bldit() { cargo build --release; }\n")
        .expect("bldit should be written");

    let target_root = tempdir.path().join("recipes");
    let report =
        add_recipe(&target_root, &source.to_string_lossy(), None).expect("import should succeed");

    assert!(report.imported_legacy_pkgdeps);
    assert!(report.imported_legacy_bldit);
    assert!(report.generated_build_lua);
    assert!(report.wrote_legacy_summary);
    assert!(target_root.join("legacy-src/legacy/pkgdeps").exists());
    assert!(target_root.join("legacy-src/legacy/pkgit.bldit").exists());
    assert!(
        target_root
            .join("legacy-src/legacy/pkgit-import.json")
            .exists()
    );
    let pkg_lua = fs::read_to_string(target_root.join("legacy-src/pkg.lua"))
        .expect("generated pkg.lua should be readable");
    assert!(pkg_lua.contains("depends = { \"dep\" }"));
}

#[test]
fn local_import_uses_file_source_when_no_git_remote_exists() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let source = tempdir.path().join("local-src");
    fs::create_dir_all(&source).expect("source dir should exist");

    let target_root = tempdir.path().join("recipes");
    let report =
        add_recipe(&target_root, &source.to_string_lossy(), None).expect("import should succeed");
    let pkg_lua = fs::read_to_string(report.recipe_dir.join("pkg.lua"))
        .expect("generated pkg.lua should be readable");

    assert!(pkg_lua.contains(&format!("file://{}", source.display())));
}
