-- 15-build-systems-meson / example-meson-app
--
-- Declarative meson build (no build.lua). Meson `features` map directly to
-- `-D<key>=<value>` flags. Place the recipe at:
--   /etc/elda/recipes/example-meson-app/pkg.lua
pkg = {
  name = "example-meson-app",
  description = "Example app built with the declarative meson backend.",
  licenses = { "MIT" },
  upstream = "https://example.invalid/example-meson-app",
  epoch = 0,
  version = "1.2.3",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/example-meson-app.git",
    tag = "v1.2.3",
  },

  build = {
    system = "meson",
    bins = { "example-meson-app" },
    features = {
      "default-library=shared",
      "tests=true",
      "manpage=true",
    },
    tests = true,
  },

  depends = {
    "glib",
  },
  makedepends = {
    "meson>=1.4",
    "ninja",
    "pkgconf",
  },
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
