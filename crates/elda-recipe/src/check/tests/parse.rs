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

#[test]
fn github_release_assets_parse_and_validate_for_matching_arches() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("fsel");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "fsel",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",
  source = {
    kind = "github_release",
    repo = "Mjoyufull/fsel",
    tag = "v1.0.0",
    assets = {
      amd64 = {
        asset = "fsel-x86_64-unknown-linux-gnu.tar.xz",
        sha256 = "abc123",
        binary = "fsel",
      },
      arm64 = {
        asset = "fsel-aarch64-unknown-linux-gnu.tar.xz",
        sha256 = "def456",
        binary = "fsel",
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
    assert_eq!(
        document
            .package
            .source
            .github_release_assets
            .get("amd64")
            .map(|asset| asset.asset.as_str()),
        Some("fsel-x86_64-unknown-linux-gnu.tar.xz")
    );
    assert_eq!(
        document
            .package
            .source
            .github_release_assets
            .get("arm64")
            .map(|asset| asset.sha256.as_str()),
        Some("def456")
    );
}

#[test]
fn github_release_assets_require_entries_for_each_package_arch() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("broken-release");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "broken-release",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",
  source = {
    kind = "github_release",
    repo = "Mjoyufull/fsel",
    tag = "v1.0.0",
    assets = {
      amd64 = {
        asset = "fsel-x86_64-unknown-linux-gnu.tar.xz",
        sha256 = "abc123",
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

    assert!(report.issues.iter().any(|issue| {
        issue.severity == IssueSeverity::Error
            && issue
                .message
                .contains("missing an `assets.arm64` entry for package arch `arm64`")
    }));
}
