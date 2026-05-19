-- 01-binary-github-release / fd
--
-- Smallest realistic maintained recipe: a single GitHub-release binary lane.
-- Place this tree at:  /etc/elda/recipes/fd/pkg.lua
--
-- Drives:
--   elda i fd                 (uses default lane = binary)
--   elda info fd              (renders identity, source, dependencies)
--   elda rc check ./fd        (validates the recipe in place)
--
-- The single-asset shorthand below pins one tarball plus its sha256. For
-- multi-architecture recipes use the per-arch `assets = { amd64 = {...} }`
-- shape demonstrated under examples/recipes/05-multi-arch-binary/.
pkg = {
  name = "fd",
  description = "Simple, fast, user-friendly alternative to find.",
  licenses = { "MIT", "Apache-2.0" },
  upstream = "https://github.com/sharkdp/fd",
  epoch = 0,
  version = "10.2.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "github_release",
    repo = "sharkdp/fd",
    tag = "v10.2.0",
    asset = "fd-v10.2.0-x86_64-unknown-linux-gnu.tar.gz",
    sha256 = "0000000000000000000000000000000000000000000000000000000000000000",
    binary = "fd",
    strip_components = 1,
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
