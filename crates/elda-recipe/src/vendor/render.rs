use super::model::{ResolvedVendorSource, VendorLockEntry};

pub(super) fn render_vendor_manifest_line(entry: &VendorLockEntry) -> String {
    let base = match entry.source_kind.as_str() {
        "github_release" => format!(
            "{} {repo}@{tag}",
            entry.package_name,
            repo = entry.repo.as_deref().unwrap_or("missing/repo"),
            tag = entry.tag.as_deref().unwrap_or("missing-tag")
        ),
        _ => format!(
            "{} {}",
            entry.package_name,
            entry.url.as_deref().unwrap_or("missing-url")
        ),
    };
    let mut suffix = Vec::new();
    if let Some(asset) = &entry.asset {
        suffix.push(format!("--asset {asset}"));
    }
    if let Some(binary) = &entry.binary {
        suffix.push(format!("--binary {binary}"));
    }

    if suffix.is_empty() {
        base
    } else {
        format!("{base} {}", suffix.join(" "))
    }
}

pub(super) fn render_vendor_pkg_lua(package_name: &str, resolved: &ResolvedVendorSource) -> String {
    let source_body = match resolved {
        ResolvedVendorSource::UrlArchive {
            url,
            sha256,
            binary,
            rename,
        } => render_source_fields(
            "url_archive",
            &[
                ("url", Some(url.as_str())),
                ("sha256", Some(sha256.as_str())),
                ("binary", binary.as_deref()),
                ("rename", rename.as_deref()),
            ],
        ),
        ResolvedVendorSource::GitHubRelease {
            repo,
            tag,
            asset,
            sha256,
            binary,
            rename,
        } => render_source_fields(
            "github_release",
            &[
                ("repo", Some(repo.as_str())),
                ("tag", Some(tag.as_str())),
                ("asset", Some(asset.as_str())),
                ("sha256", Some(sha256.as_str())),
                ("binary", binary.as_deref()),
                ("rename", rename.as_deref()),
            ],
        ),
    };

    format!(
        "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n\n  source = {{\n{source_body}  }},\n\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n\n  conffiles = {{}},\n  sysusers = {{}},\n  tmpfiles = {{}},\n  alternatives = {{}},\n  hooks = {{}},\n\n  flags_default = {{}},\n  flags_allowed = {{}},\n  flags_implies = {{}},\n  flags_conflicts = {{}},\n\n  subpackages = {{}},\n}}\n",
        name = escape_lua_string(package_name),
        source_body = source_body,
    )
}

fn render_source_fields(kind: &str, fields: &[(&str, Option<&str>)]) -> String {
    let mut rendered = format!("    kind = \"{}\",\n", escape_lua_string(kind));
    for (key, value) in fields {
        if let Some(value) = value {
            rendered.push_str(&format!("    {key} = \"{}\",\n", escape_lua_string(value)));
        }
    }

    rendered
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
