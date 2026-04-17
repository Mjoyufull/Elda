use std::path::Path;

use crate::{LuaValue, parse_pkg_lua, validate_recipe};

#[test]
fn metadata_families_parse_and_validate_in_the_current_slice() {
    let document = parse_pkg_lua(
        Path::new("pkg.lua"),
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
  conffiles = { "/etc/example.conf" },
  sysusers = {
    { kind = "user", name = "example", system = true },
  },
  tmpfiles = {
    { type = "d", path = "/var/lib/example", mode = "0750" },
  },
  alternatives = {
    { name = "editor", link = "/usr/bin/editor", path = "/usr/bin/example", priority = 50 },
  },
  hooks = {
    post_install = { file = "hooks/post_install.lua" },
  },
  flags_default = {
    wayland = true,
  },
  flags_allowed = {
    wayland = true,
    x11 = true,
  },
  flags_implies = {
    desktop = { "wayland" },
  },
  flags_conflicts = {
    wayland = { "x11" },
  },
  subpackages = {
    { name = "example-docs" },
  },
}
"#,
    )
    .expect("pkg.lua should parse");

    assert!(validate_recipe(&document).is_empty());
    assert!(matches!(
        document.package.sysusers,
        Some(LuaValue::Array(_))
    ));
    assert!(matches!(
        document.package.tmpfiles,
        Some(LuaValue::Array(_))
    ));
    assert!(matches!(
        document.package.alternatives,
        Some(LuaValue::Array(_))
    ));
    assert!(matches!(document.package.hooks, Some(LuaValue::Table(_))));
    assert!(matches!(
        document.package.flags_default,
        Some(LuaValue::Table(_))
    ));
    assert!(matches!(
        document.package.subpackages,
        Some(LuaValue::Array(_))
    ));
}

#[test]
fn invalid_metadata_shapes_are_reported() {
    let document = parse_pkg_lua(
        Path::new("pkg.lua"),
        r#"
pkg = {
  name = "broken",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",
  source = {
    kind = "git",
    url = "https://example.invalid/broken.git",
    branch = "main",
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
  sysusers = "metadata/sysusers.conf",
  alternatives = {
    { name = "editor", link = "/usr/bin/editor", path = "/usr/bin/example" },
  },
  hooks = {
    post_install = "hooks/post_install.lua",
  },
  flags_default = { wayland = "yes" },
  flags_implies = {
    desktop = "wayland",
  },
}
"#,
    )
    .expect("pkg.lua should parse");
    let issues = validate_recipe(&document);

    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("sysusers must be either an inline array or { file = \"...\" }")
    }));
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("alternatives entries require an integer `priority` field")
    }));
    assert!(
        issues
            .iter()
            .any(|issue| { issue.message.contains("hooks.post_install must be a table") })
    );
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("flags_default.wayland must be a boolean")
    }));
    assert!(issues.iter().any(|issue| {
        issue
            .message
            .contains("flags_implies.desktop must be an array of non-empty flag names")
    }));
}
