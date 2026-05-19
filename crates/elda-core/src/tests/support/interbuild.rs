use std::fs;
use std::path::Path;

pub(in crate::tests) fn write_interbuild_recipe(
    root: &Path,
    name: &str,
    kind: &str,
    repo_dir: &Path,
    extra_fields: &str,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"{kind}\",\n    url = \"file://{repo}\",\n{extra_fields}  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
            repo = repo_dir.display(),
        ),
    )
    .expect("interbuild recipe should be written");
}
