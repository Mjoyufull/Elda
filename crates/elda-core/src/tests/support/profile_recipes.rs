use std::fs;
use std::path::Path;

pub(in crate::tests) fn write_local_profile_recipe_with_policy(
    root: &Path,
    name: &str,
    repo_dir: &Path,
    depends: &[&str],
    native_arch: Option<&str>,
    foreign_arches: &[&str],
    init: Option<&str>,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"profile\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{repo}\",\n    branch = \"main\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n{profile_block}}}\n",
            name = name,
            repo = repo_dir.display(),
            depends = string_array_lua(depends),
            profile_block = render_profile_policy(native_arch, foreign_arches, init),
        ),
    )
    .expect("profile recipe should be written");
}

fn render_profile_policy(
    native_arch: Option<&str>,
    foreign_arches: &[&str],
    init: Option<&str>,
) -> String {
    if native_arch.is_none() && foreign_arches.is_empty() && init.is_none() {
        return String::new();
    }

    let mut body = String::from("  profile = {\n");
    if let Some(native_arch) = native_arch {
        body.push_str(&format!("    native_arch = \"{native_arch}\",\n"));
    }
    if !foreign_arches.is_empty() {
        body.push_str(&format!(
            "    foreign_arches = {},\n",
            string_array_lua(foreign_arches)
        ));
    }
    if let Some(init) = init {
        body.push_str(&format!("    init = \"{init}\",\n"));
    }
    body.push_str("  },\n");

    body
}

fn string_array_lua(values: &[&str]) -> String {
    if values.is_empty() {
        return "{}".to_owned();
    }

    format!(
        "{{ {} }}",
        values
            .iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
