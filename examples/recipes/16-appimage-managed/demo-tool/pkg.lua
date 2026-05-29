-- 16-appimage-managed / demo-tool
--
-- Managed AppImage lane: the `.AppImage` file is stored under
-- `usr/lib/elda/appimages/<name>/<epoch>:<version>-<rel>/payload/` and a symlink
-- launcher is placed in `usr/bin/`.
--
-- Use `integration = "desktop"` only when you want a minimal `.desktop` stub;
-- icons and AppStream still require recipe-side assets today.
--
-- Drives:
--   elda rc check ./demo-tool
pkg = {
  name = "demo-tool",
  description = "Example managed AppImage payload (replace URLs and checksums).",
  licenses = { "MIT" },
  upstream = "https://example.invalid/demo-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "appimage",
    repo = "example/demo-tool",
    tag = "v1.0.0",
    asset = "demo-tool-1.0.0-x86_64-unknown-linux-gnu.AppImage",
    sha256 = "0000000000000000000000000000000000000000000000000000000000000000",
    binary = "demo-tool",
    integration = "none",
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
