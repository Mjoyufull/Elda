-- 02-source-cargo / ripgrep
--
-- Pure source-lane recipe driven by the declarative `build` table. No
-- `build.lua` is needed because the cargo build path is expressible directly
-- in pkg.lua. Place this tree at:  /etc/elda/recipes/ripgrep/pkg.lua
--
-- Drives:
--   elda ig ripgrep           (forces the source lane explicitly)
--   elda i ripgrep            (uses source lane because no binary lane is declared)
--
-- Note: when `build.system` covers the case (`cargo`, `cmake`, `meson`, `make`,
-- `go`, `python`, `zig`, `nimble`), `build.lua` should *not* exist.
pkg = {
  name = "ripgrep",
  description = "Recursively searches directories for a regex pattern.",
  licenses = { "MIT", "Unlicense" },
  upstream = "https://github.com/BurntSushi/ripgrep",
  epoch = 0,
  version = "14.1.1",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://github.com/BurntSushi/ripgrep",
    tag = "14.1.1",
  },

  build = {
    system = "cargo",
    bins = { "rg" },
    features = { "pcre2" },
    tests = true,
  },

  depends = {
    "pcre2",
  },
  makedepends = {
    "rust>=1.75",
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
