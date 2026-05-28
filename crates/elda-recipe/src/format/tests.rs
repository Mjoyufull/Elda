use std::fs;

use tempfile::TempDir;

use super::{format_recipe_file, normalize_recipe_file};

#[test]
fn format_recipe_rewrites_pkg_lua_with_canonical_spacing() {
    let tempdir = TempDir::new().expect("tempdir");
    let recipe_dir = tempdir.path().join("demo");
    fs::create_dir_all(&recipe_dir).expect("recipe dir");
    fs::write(
        recipe_dir.join("pkg.lua"),
        "pkg={name=\"demo\",version=\"1\",rel=0,arch={\"any\"},kind=\"normal\",source={kind=\"git\",url=\"https://example.invalid/repo.git\",rev=\"main\"}}",
    )
    .expect("pkg.lua");

    let formatted = format_recipe_file(&recipe_dir.join("pkg.lua")).expect("format should succeed");
    assert!(formatted.contains("pkg = {"));
    assert!(formatted.contains("name = \"demo\""));
}

#[test]
fn normalize_recipe_rejects_invalid_recipes() {
    let tempdir = TempDir::new().expect("tempdir");
    let recipe_dir = tempdir.path().join("broken");
    fs::create_dir_all(&recipe_dir).expect("recipe dir");
    fs::write(recipe_dir.join("pkg.lua"), "pkg = { }\n").expect("pkg.lua");

    normalize_recipe_file(&recipe_dir.join("pkg.lua")).expect_err("invalid recipe should fail");
}
