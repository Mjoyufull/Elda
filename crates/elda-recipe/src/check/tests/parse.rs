use super::*;

#[test]
fn empty_recipe_root_reports_no_issues() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");

    assert!(report.recipes.is_empty());
    assert!(report.issues.is_empty());
}

#[test]
fn valid_pkg_lua_parses_and_validates() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("example");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "example",
  epoch = 0,
  version = "1.2.3",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "git",
    url = "https://example.invalid/example.git",
    branch = "main",
  },
  depends = { "openssl>=3", { any = { "mesa", "nvidia-utils" } } },
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");

    assert_eq!(report.issues.len(), 0);
    assert!(report.recipes[0].parsed);
}

#[test]
fn invalid_pkg_lua_is_reported() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("broken");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "broken",
  version = "1.0.0",
  rel = 1,
  arch = { "x86_64" },
  kind = "normal",
  source = {
    kind = "git",
    url = "https://example.invalid/broken.git",
  },
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    );
}

#[test]
fn multi_lane_pkg_lua_parses_and_validates() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("dual");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "dual",
  epoch = 0,
  version = "1.2.3",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    default_lane = "binary",
    lanes = {
      source = {
        kind = "git",
        url = "https://example.invalid/dual.git",
        branch = "main",
      },
      binary = {
        kind = "url_archive",
        url = "https://example.invalid/dual.tar.gz",
        sha256 = "abc123",
        binary = "dual",
      },
    },
  },
  depends = {},
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");

    assert!(report.issues.is_empty());
    let document = report.recipes[0]
        .document
        .as_ref()
        .expect("recipe should parse");
    assert!(document.package.source.is_multi_lane());
    assert_eq!(
        document.package.source.default_lane.as_deref(),
        Some("binary")
    );
    assert!(document.package.source.lanes.contains_key("source"));
    assert!(document.package.source.lanes.contains_key("binary"));
}

#[test]
fn conditional_dependency_when_predicate_is_parsed() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("flagged");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "flagged",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = { kind = "git", url = "https://example.invalid/flagged.git", branch = "main" },
  depends = {
    "openssl>=3",
    { name = "wayland-protocols", when = "+wayland" },
    { any = { "gtk3", "gtk4" }, when = "+gtk,-headless" },
  },
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
  flags_default = { wayland = false, gtk = false, headless = false },
  flags_allowed = { wayland = true, gtk = true, headless = true },
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");
    assert!(
        report.issues.is_empty(),
        "expected no issues, got: {:?}",
        report.issues
    );
    let document = report.recipes[0]
        .document
        .as_ref()
        .expect("recipe should parse");
    let depends = &document.package.depends;
    assert_eq!(depends.len(), 3);
    assert!(depends[0].when.is_none());
    assert_eq!(
        depends[1]
            .when
            .as_ref()
            .map(|predicate| predicate.atoms.len()),
        Some(1)
    );
    assert_eq!(
        depends[2]
            .when
            .as_ref()
            .map(|predicate| predicate.atoms.len()),
        Some(2)
    );
}

#[test]
fn when_predicate_referencing_undeclared_flag_is_reported() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("missing-flag");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "missing-flag",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = { kind = "git", url = "https://example.invalid/missing-flag.git", branch = "main" },
  depends = { { name = "wayland-protocols", when = "+wayland" } },
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
  flags_allowed = { gtk = true },
  flags_default = { gtk = false },
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");
    assert!(report.issues.iter().any(|issue| {
        issue.severity == IssueSeverity::Error
            && issue.message.contains("references undeclared flag")
    }));
}

#[test]
fn flag_descriptions_and_cardinality_tables_parse() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("flag-meta");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "flag-meta",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = { kind = "git", url = "https://example.invalid/flag-meta.git", branch = "main" },
  depends = {},
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
  flags_default = { wayland = true, x11 = false, intel = false, nvidia = false, radeon = false },
  flags_allowed = { wayland = true, x11 = true, intel = true, nvidia = true, radeon = true },
  flags_descriptions = {
    wayland = "Build the Wayland surface backend",
    x11 = "Build the legacy X11 surface backend",
  },
  flags_required_one_of = { gpu = { "intel", "nvidia", "radeon" } },
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");
    assert!(
        report.issues.is_empty(),
        "expected clean parse, got: {:?}",
        report.issues
    );
    let document = report.recipes[0]
        .document
        .as_ref()
        .expect("recipe should parse");
    assert!(document.package.flags_descriptions.is_some());
    assert!(document.package.flags_required_one_of.is_some());
}

#[test]
fn cardinality_table_with_single_member_is_reported() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("bad-cardinality");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "bad-cardinality",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = { kind = "git", url = "https://example.invalid/bad-cardinality.git", branch = "main" },
  depends = {},
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
  flags_allowed = { wayland = true },
  flags_required_one_of = { surface = { "wayland" } },
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");
    assert!(report.issues.iter().any(|issue| {
        issue.severity == IssueSeverity::Error
            && issue
                .message
                .contains("must contain at least two flag names")
    }));
}

#[test]
fn invalid_provide_constraint_is_reported() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("broken-provide");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "broken-provide",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "git",
    url = "https://example.invalid/broken-provide.git",
    branch = "main",
  },
  depends = {},
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = { "libgl>=1" },
  conflicts = {},
  replaces = {},
  conffiles = {},
}
"#,
    )
    .expect("pkg.lua should be written");

    let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");

    assert!(report.issues.iter().any(|issue| {
        issue.severity == IssueSeverity::Error
            && issue.message.contains("provides contains invalid provide")
    }));
}
