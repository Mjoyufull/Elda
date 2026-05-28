use crate::model::RemoteDocument;

use super::{Candidate, InteremoteKind, PackageMetadata};

pub(super) fn render_pkg_lua(
    remote: &RemoteDocument,
    candidate: &Candidate,
    metadata: &PackageMetadata,
    commit: Option<&str>,
) -> String {
    let source_kind = source_kind_label(candidate.kind);
    let package = candidate.rel_path.to_string_lossy();
    let rev_line = commit
        .map(|commit| format!("    rev = \"{}\",\n", escape_lua_string(commit)))
        .unwrap_or_default();

    format!(
        "pkg = {{\n  name = \"{name}\",\n  description = \"{description}\",\n  licenses = {licenses},\n  upstream = \"{upstream}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = {rel},\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n\n  source = {{\n    kind = \"{source_kind}\",\n    url = \"{url}\",\n{rev_line}    package = \"{package}\",\n  }},\n\n  depends = {depends},\n  makedepends = {makedepends},\n  checkdepends = {checkdepends},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  sysusers = {{}},\n  tmpfiles = {{}},\n  alternatives = {{}},\n  hooks = {{}},\n  provider_assets = {{}},\n\n  flags_default = {flags_default},\n  flags_allowed = {flags_allowed},\n  flags_implies = {{}},\n  flags_conflicts = {{}},\n\n  subpackages = {{}},\n}}\n",
        name = escape_lua_string(&candidate.name),
        description = escape_lua_string(metadata.description.as_deref().unwrap_or_default()),
        licenses = render_array(&metadata.license),
        upstream = escape_lua_string(metadata.homepage.as_deref().unwrap_or_default()),
        version = escape_lua_string(metadata.version.as_deref().unwrap_or("0.1.0")),
        rel = metadata.rel,
        source_kind = source_kind,
        url = escape_lua_string(&remote.index_url),
        rev_line = rev_line,
        package = escape_lua_string(&package),
        depends = render_array(&metadata.depends),
        makedepends = render_array(&metadata.makedepends),
        checkdepends = render_array(&metadata.checkdepends),
        flags_default = render_bool_table(&metadata.flags_default, true),
        flags_allowed = render_bool_table(&metadata.flags_allowed, true),
    )
}

fn render_array(values: &[String]) -> String {
    if values.is_empty() {
        return "{}".to_owned();
    }
    let entries = values
        .iter()
        .map(|value| format!("\"{}\"", escape_lua_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {entries} }}")
}

fn render_bool_table(enabled: &[String], value: bool) -> String {
    if enabled.is_empty() {
        return "{}".to_owned();
    }
    let entries = enabled
        .iter()
        .map(|flag| format!("[\"{}\"] = {value}", escape_lua_string(flag)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {entries} }}")
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn source_kind_label(kind: InteremoteKind) -> &'static str {
    match kind {
        InteremoteKind::GentooOverlay => "gentoo_overlay",
        InteremoteKind::XbpsSrc => "xbps_template",
    }
}
