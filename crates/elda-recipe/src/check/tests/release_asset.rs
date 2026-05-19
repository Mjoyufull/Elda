use super::*;

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
    let assets = &document.package.source.github_release_assets;
    assert_eq!(
        assets.get("amd64").map(|asset| asset.asset.as_str()),
        Some("fsel-x86_64-unknown-linux-gnu.tar.xz")
    );
    assert_eq!(
        assets.get("arm64").map(|asset| asset.sha256.as_str()),
        Some("def456")
    );
}

#[test]
fn release_asset_source_accepts_gitlab_provider() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("gitlab-release-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "gitlab-release-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "release_asset",
    provider = "gitlab",
    repo = "owner/release-tool",
    tag = "v1.0.0",
    asset = "release-tool-linux-amd64.tar.gz",
    sha256 = "abc123",
    binary = "release-tool",
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
    assert_eq!(
        report.recipes[0]
            .document
            .as_ref()
            .expect("recipe should parse")
            .package
            .source
            .fields
            .get("provider"),
        Some(&crate::ScalarValue::String("gitlab".to_owned()))
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

#[test]
fn release_asset_source_parses_and_validates_for_github_provider() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("release-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "release-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "release_asset",
    provider = "github",
    repo = "owner/release-tool",
    tag = "v1.0.0",
    assets = {
      amd64 = {
        asset = "release-tool-linux-amd64.tar.gz",
        sha256 = "abc123",
        signature = "release-tool-linux-amd64.tar.gz.minisig",
        binary = "release-tool",
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
    assert_eq!(document.package.source.kind, "release_asset");
    let asset = document
        .package
        .source
        .github_release_assets
        .get("amd64")
        .expect("amd64 release asset should parse");
    assert_eq!(asset.asset, "release-tool-linux-amd64.tar.gz");
    assert_eq!(
        asset.signature.as_deref(),
        Some("release-tool-linux-amd64.tar.gz.minisig")
    );
}

#[test]
fn release_asset_host_must_be_bare_host() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("bad-host-release-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "bad-host-release-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "release_asset",
    provider = "gitlab",
    host = "https://gitlab.example.invalid",
    repo = "owner/release-tool",
    tag = "v1.0.0",
    asset = "release-tool-linux-amd64.tar.gz",
    sha256 = "abc123",
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

    let report = check_local_recipes(tempdir.path(), None).expect("check should finish");

    assert!(report.issues.iter().any(|issue| {
        issue
            .message
            .contains("release_asset host must be a bare forge host")
    }));
}

#[test]
fn release_asset_source_accepts_sourcehut_and_direct_providers() {
    for (name, provider, repo) in [
        ("sourcehut-tool", "sourcehut", "~chris/tool"),
        (
            "direct-tool",
            "direct",
            "https://example.invalid/tool.elda-releases.json",
        ),
    ] {
        let tempdir = TempDir::new().expect("tempdir should exist");
        let recipe_dir = tempdir.path().join(name);
        fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
        fs::write(
            recipe_dir.join("pkg.lua"),
            format!(
                r#"
pkg = {{
  name = "{name}",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = {{ "amd64" }},
  kind = "normal",
  source = {{
    kind = "release_asset",
    provider = "{provider}",
    repo = "{repo}",
    tag = "v1.0.0",
    asset = "tool.tar.gz",
    sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  }},
  depends = {{}},
  makedepends = {{}},
  checkdepends = {{}},
  recommends = {{}},
  suggests = {{}},
  supplements = {{}},
  enhances = {{}},
  provides = {{}},
  conflicts = {{}},
  replaces = {{}},
  conffiles = {{}},
}}
"#
            ),
        )
        .expect("pkg.lua should be written");

        let report = check_local_recipes(tempdir.path(), None).expect("check should succeed");
        assert!(report.issues.is_empty());
    }
}
