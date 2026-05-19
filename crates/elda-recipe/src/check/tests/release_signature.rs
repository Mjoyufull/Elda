use super::*;

#[test]
fn release_asset_signature_fields_must_be_safe_non_empty_values() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("bad-signature-release-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "bad-signature-release-tool",
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
    asset = "release-tool-linux-amd64.tar.gz",
    sha256 = "abc123",
    signature = "../release-tool-linux-amd64.tar.gz.minisig",
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
            .contains("source `signature` must not contain parent-directory traversal")
    }));
}

#[test]
fn arch_release_asset_signature_must_not_be_empty() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let recipe_dir = tempdir.path().join("empty-signature-release-tool");
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        r#"
pkg = {
  name = "empty-signature-release-tool",
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
        signature = "",
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

    let report = check_local_recipes(tempdir.path(), None).expect("check should finish");

    assert!(report.issues.iter().any(|issue| {
        issue
            .message
            .contains("source `assets.amd64.signature` must not be empty")
    }));
}
