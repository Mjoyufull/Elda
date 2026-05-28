-- 03-source-with-build-lua / zig-toy
--
-- Source recipe whose build steps cannot be fully expressed by the declarative
-- `build` table, so we drop into `build.lua`. The accompanying `build.lua`
-- runs inside Elda's embedded Lua sandbox and may use the staging helpers,
-- archive/process helpers, structured logging, and metadata inspection
-- documented in SPEC §5.3.
--
-- Place this tree at:  /etc/elda/recipes/zig-toy/
--   pkg.lua
--   build.lua
pkg = {
  name = "zig-toy",
  description = "Toy zig project that needs custom staging and asset rewrites.",
  licenses = { "MIT" },
  upstream = "https://example.invalid/zig-toy",
  epoch = 0,
  version = "0.3.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/zig-toy.git",
    tag = "v0.3.0",
  },

  depends = {},
  makedepends = {
    "zig>=0.13",
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
