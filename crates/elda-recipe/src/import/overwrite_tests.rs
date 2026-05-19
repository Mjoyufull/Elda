use std::fs;

use tempfile::TempDir;

fn expect_single(result: super::ImportResult) -> super::ImportReport {
    match result {
        super::ImportResult::Single(report) => report,
        super::ImportResult::Bulk(_) => panic!("expected a single recipe import report"),
    }
}

fn path_str(path: &std::path::Path) -> &str {
    path.to_str().expect("test path should be valid UTF-8")
}

#[test]
fn add_recipe_preserves_existing_generated_metadata_without_replace() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes = tempdir.path().join("recipes");
    let recipe_dir = recipes.join("stable-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        "pkg = { name = \"stable-tool\" }\n",
    )
    .expect("existing metadata should be written");

    let report = expect_single(
        super::add_recipe_with_options(
            &recipes,
            "stable-tool",
            None,
            &super::ImportOptions::default(),
        )
        .expect("existing metadata should be preserved"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should be readable");

    assert_eq!(pkg_lua, "pkg = { name = \"stable-tool\" }\n");
    assert!(!report.generated_pkg_lua);
}

#[test]
fn add_recipe_replace_overwrites_existing_generated_metadata() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes = tempdir.path().join("recipes");
    let recipe_dir = recipes.join("replace-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(recipe_dir.join("pkg.lua"), "pkg = { name = \"old\" }\n")
        .expect("existing metadata should be written");

    let report = expect_single(
        super::add_recipe_with_options(
            &recipes,
            "replace-tool",
            None,
            &super::ImportOptions {
                replace: true,
                ..super::ImportOptions::default()
            },
        )
        .expect("replace should regenerate metadata"),
    );
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should be readable");

    assert!(pkg_lua.contains(r#"name = "replace-tool""#));
    assert!(!pkg_lua.contains(r#"name = "old""#));
    assert!(report.generated_pkg_lua);
}

#[test]
fn local_import_preserves_existing_metadata_without_replace() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes = tempdir.path().join("recipes");
    let recipe_dir = recipes.join("local-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(recipe_dir.join("pkg.lua"), "pkg = { name = \"kept\" }\n")
        .expect("existing pkg.lua should be written");
    fs::write(recipe_dir.join("build.lua"), "-- kept build\n")
        .expect("existing build.lua should be written");

    let source = tempdir.path().join("sources/local-tool");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("pkg.lua"), "pkg = { name = \"incoming\" }\n")
        .expect("incoming pkg.lua should be written");
    fs::write(source.join("build.lua"), "-- incoming build\n")
        .expect("incoming build.lua should be written");

    let report = expect_single(
        super::add_recipe_with_options(
            &recipes,
            path_str(&source),
            None,
            &super::ImportOptions::default(),
        )
        .expect("local import should preserve existing metadata"),
    );

    assert_eq!(
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should read"),
        "pkg = { name = \"kept\" }\n"
    );
    assert_eq!(
        fs::read_to_string(report.recipe_dir.join("build.lua")).expect("build.lua should read"),
        "-- kept build\n"
    );
    assert!(!report.imported_pkg_lua);
    assert!(!report.imported_build_lua);
}

#[test]
fn local_import_replace_overwrites_existing_metadata() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipes = tempdir.path().join("recipes");
    let recipe_dir = recipes.join("local-replace-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(recipe_dir.join("pkg.lua"), "pkg = { name = \"kept\" }\n")
        .expect("existing pkg.lua should be written");

    let source = tempdir.path().join("sources/local-replace-tool");
    fs::create_dir_all(&source).expect("source dir should exist");
    fs::write(source.join("pkg.lua"), "pkg = { name = \"incoming\" }\n")
        .expect("incoming pkg.lua should be written");

    let report = expect_single(
        super::add_recipe_with_options(
            &recipes,
            path_str(&source),
            None,
            &super::ImportOptions {
                replace: true,
                ..super::ImportOptions::default()
            },
        )
        .expect("local import replace should overwrite metadata"),
    );

    assert_eq!(
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should read"),
        "pkg = { name = \"incoming\" }\n"
    );
    assert!(report.imported_pkg_lua);
}
