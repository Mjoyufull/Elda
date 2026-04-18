use std::fs;

use tempfile::TempDir;

use super::add_recipe;

#[test]
fn add_recipe_can_scaffold_a_profile_recipe() {
    let tempdir = TempDir::new().expect("tempdir should exist");

    let report = add_recipe(tempdir.path(), "yoka-core", Some("profile"))
        .expect("profile scaffold should succeed");
    let pkg_lua =
        fs::read_to_string(report.recipe_dir.join("pkg.lua")).expect("pkg.lua should exist");

    assert!(pkg_lua.contains("kind = \"profile\""));
    assert!(pkg_lua.contains("profile = {}"));
}
