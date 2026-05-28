use super::detect::detect_default_branch;
use super::metadata::GeneratedMetadata;
use super::model::{GitRefKind, GitRefRequest, LegacyPkgdep};
use super::strategy::SourceStrategy;

pub(super) fn render_pkg_lua(
    recipe_name: &str,
    source_url: Option<&str>,
    legacy_pkgdeps: &[LegacyPkgdep],
    recipe_kind: &str,
    strategy: &SourceStrategy,
    metadata: &GeneratedMetadata,
    git_ref: Option<&GitRefRequest>,
) -> String {
    render_pkg_lua_with_binary_lane(PkgLuaRender {
        recipe_name,
        source_url,
        legacy_pkgdeps,
        recipe_kind,
        source_strategy: strategy,
        binary_strategy: None,
        default_lane: "source",
        metadata,
        git_ref,
    })
}

pub(super) struct PkgLuaRender<'a> {
    pub(super) recipe_name: &'a str,
    pub(super) source_url: Option<&'a str>,
    pub(super) legacy_pkgdeps: &'a [LegacyPkgdep],
    pub(super) recipe_kind: &'a str,
    pub(super) source_strategy: &'a SourceStrategy,
    pub(super) binary_strategy: Option<&'a SourceStrategy>,
    pub(super) default_lane: &'a str,
    pub(super) metadata: &'a GeneratedMetadata,
    pub(super) git_ref: Option<&'a GitRefRequest>,
}

pub(super) fn render_pkg_lua_with_binary_lane(input: PkgLuaRender<'_>) -> String {
    let source_block = render_source_block(
        input.source_url,
        input.source_strategy,
        input.binary_strategy,
        input.default_lane,
        input.git_ref,
    );
    let metadata = input.metadata;
    let depends_block = render_depends_block(input.legacy_pkgdeps, &metadata.depends);
    let makedepends_block = render_string_array("makedepends", &metadata.makedepends);
    let checkdepends_block = render_string_array("checkdepends", &metadata.checkdepends);
    let provides_block = render_string_array("provides", &metadata.provides);
    let conflicts_block = render_string_array("conflicts", &metadata.conflicts);
    let replaces_block = render_string_array("replaces", &metadata.replaces);

    format!(
        "pkg = {{\n  name = \"{name}\",\n  description = \"{description}\",\n  licenses = {licenses},\n  upstream = \"{upstream}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = {rel},\n  arch = {{ \"amd64\" }},\n  kind = \"{recipe_kind}\",\n\n{source_block}\n{depends_block}{makedepends_block}{checkdepends_block}  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n{provides_block}{conflicts_block}{replaces_block}\n  conffiles = {{}},\n  sysusers = {{}},\n  tmpfiles = {{}},\n  alternatives = {{}},\n  hooks = {{}},\n  provider_assets = {{}},\n\n  flags_default = {{}},\n  flags_allowed = {{}},\n  flags_implies = {{}},\n  flags_conflicts = {{}},\n\n  subpackages = {{}},\n{profile_block}}}\n",
        name = escape_lua_string(input.recipe_name),
        description = escape_lua_string(metadata.description.as_deref().unwrap_or_default()),
        licenses = render_array_values(&metadata.licenses),
        upstream = escape_lua_string(metadata.upstream.as_deref().unwrap_or_default()),
        version = escape_lua_string(metadata.version.as_deref().unwrap_or("0.1.0")),
        rel = metadata.rel.unwrap_or(1),
        recipe_kind = escape_lua_string(input.recipe_kind),
        source_block = source_block,
        depends_block = depends_block,
        makedepends_block = makedepends_block,
        checkdepends_block = checkdepends_block,
        provides_block = provides_block,
        conflicts_block = conflicts_block,
        replaces_block = replaces_block,
        profile_block = profile_block(input.recipe_kind),
    )
}

fn render_source_block(
    source_url: Option<&str>,
    strategy: &SourceStrategy,
    binary_strategy: Option<&SourceStrategy>,
    default_lane: &str,
    git_ref: Option<&GitRefRequest>,
) -> String {
    let Some(binary_strategy) = binary_strategy else {
        return format!(
            "  source = {{\n    kind = \"{}\",\n{}  }},\n\n",
            strategy.kind(),
            source_body(source_url, strategy, git_ref),
        );
    };

    format!(
        "  source = {{\n    default_lane = \"{}\",\n    lanes = {{\n      source = {{\n        kind = \"{}\",\n{}      }},\n      binary = {{\n        kind = \"{}\",\n{}      }},\n    }},\n  }},\n\n",
        escape_lua_string(default_lane),
        strategy.kind(),
        indent_lane_body(&source_body(source_url, strategy, git_ref)),
        binary_strategy.kind(),
        indent_lane_body(&source_body(source_url, binary_strategy, None)),
    )
}

fn source_body(
    source_url: Option<&str>,
    strategy: &SourceStrategy,
    git_ref: Option<&GitRefRequest>,
) -> String {
    format!(
        "{}{}{}",
        source_line(source_url, strategy),
        git_ref_line(source_url, strategy, git_ref),
        strategy.extra_fields(),
    )
}

fn indent_lane_body(body: &str) -> String {
    body.lines()
        .map(|line| format!("    {line}\n"))
        .collect::<String>()
}

fn source_line(source_url: Option<&str>, strategy: &SourceStrategy) -> String {
    if matches!(
        strategy.kind(),
        "github_release" | "release_asset" | "appimage"
    ) {
        return String::new();
    }
    match source_url {
        Some(source_url) => format!(
            "    url = \"{}\",
",
            escape_lua_string(source_url)
        ),
        None => "    url = \"https://example.invalid/replace-me.git\",
"
        .to_owned(),
    }
}

fn git_ref_line(
    source_url: Option<&str>,
    strategy: &SourceStrategy,
    git_ref: Option<&GitRefRequest>,
) -> String {
    if strategy.kind() != "git" {
        return String::new();
    }

    if let Some(git_ref) = git_ref {
        return explicit_git_ref_line(git_ref);
    }

    let branch = source_url
        .and_then(detect_default_branch)
        .unwrap_or_else(|| "main".to_owned());
    format!("    branch = \"{}\",\n", escape_lua_string(&branch))
}

fn explicit_git_ref_line(git_ref: &GitRefRequest) -> String {
    let key = match git_ref.kind {
        GitRefKind::Branch => "branch",
        GitRefKind::Tag => "tag",
        GitRefKind::Rev => "rev",
    };
    format!("    {key} = \"{}\",\n", escape_lua_string(&git_ref.value))
}

fn profile_block(recipe_kind: &str) -> &'static str {
    if recipe_kind == "profile" {
        "  profile = {},\n"
    } else {
        ""
    }
}

fn render_depends_block(legacy_pkgdeps: &[LegacyPkgdep], metadata_depends: &[String]) -> String {
    let mut entries = legacy_pkgdeps
        .iter()
        .map(|pkgdep| pkgdep.package_name.clone())
        .collect::<Vec<_>>();
    entries.extend(metadata_depends.iter().cloned());
    entries.sort();
    entries.dedup();

    render_string_array("depends", &entries)
}

fn render_string_array(key: &str, values: &[String]) -> String {
    format!("  {key} = {},\n", render_array_values(values))
}

fn render_array_values(values: &[String]) -> String {
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

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
