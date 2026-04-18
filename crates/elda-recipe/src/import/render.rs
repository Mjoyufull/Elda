use super::detect::detect_default_branch;
use super::model::LegacyPkgdep;

pub(super) fn render_pkg_lua(
    recipe_name: &str,
    source_url: Option<&str>,
    legacy_pkgdeps: &[LegacyPkgdep],
    recipe_kind: &str,
) -> String {
    let source_line = match source_url {
        Some(source_url) => format!("    url = \"{}\",\n", escape_lua_string(source_url)),
        None => "    url = \"https://example.invalid/replace-me.git\",\n".to_owned(),
    };
    let depends_block = render_depends_block(legacy_pkgdeps);
    let branch = source_url
        .and_then(detect_default_branch)
        .unwrap_or_else(|| "main".to_owned());

    format!(
        "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"{recipe_kind}\",\n\n  source = {{\n    kind = \"git\",\n{source_line}    branch = \"{branch}\",\n  }},\n\n{depends_block}  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n\n  conffiles = {{}},\n  sysusers = {{}},\n  tmpfiles = {{}},\n  alternatives = {{}},\n  hooks = {{}},\n\n  flags_default = {{}},\n  flags_allowed = {{}},\n  flags_implies = {{}},\n  flags_conflicts = {{}},\n\n  subpackages = {{}},\n{profile_block}}}\n",
        name = escape_lua_string(recipe_name),
        recipe_kind = escape_lua_string(recipe_kind),
        branch = escape_lua_string(&branch),
        source_line = source_line,
        depends_block = depends_block,
        profile_block = profile_block(recipe_kind),
    )
}

fn profile_block(recipe_kind: &str) -> &'static str {
    if recipe_kind == "profile" {
        "  profile = {},\n"
    } else {
        ""
    }
}

fn render_depends_block(legacy_pkgdeps: &[LegacyPkgdep]) -> String {
    if legacy_pkgdeps.is_empty() {
        return "  depends = {},\n".to_owned();
    }

    let entries = legacy_pkgdeps
        .iter()
        .map(|pkgdep| format!("\"{}\"", escape_lua_string(&pkgdep.package_name)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("  depends = {{ {} }},\n", entries)
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
