-- 04-dual-lane-source-and-binary / fd
--
-- Single package identity that exposes both an upstream source-build path and
-- an upstream release-binary path. Lane selection follows SPEC §5.2:
--
--   1. explicit command (`elda ig` / `elda ib`)
--   2. explicit preference flag (`--prefer-source` / `--prefer-binary`)
--   3. `source.default_lane`
--   4. config `defaults.install_preference`
--
-- Drives:
--   elda i fd                 -> binary (default_lane)
--   elda ig fd                -> source (forced)
--   elda ib fd                -> binary (forced)
--   elda i fd --prefer-source -> source (override)
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
    default_lane = "binary",
    lanes = {
      source = {
        kind = "git",
        url = "https://github.com/sharkdp/fd",
        tag = "v10.2.0",
      },
      binary = {
        kind = "github_release",
        repo = "sharkdp/fd",
        tag = "v10.2.0",
        asset = "fd-v10.2.0-x86_64-unknown-linux-gnu.tar.gz",
        sha256 = "0000000000000000000000000000000000000000000000000000000000000000",
        binary = "fd",
        strip_components = 1,
      },
    },
  },

  build = {
    system = "cargo",
    bins = { "fd" },
    tests = false,
  },

  depends = {},
  makedepends = {
    "rust>=1.75",
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
