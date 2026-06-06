-- 18-binary-raw-release-asset / demo-tool
--
-- Plain release asset that is already the executable file. There is no archive
-- member to name with `binary`; `rename` controls the command installed under
-- `usr/bin/`.
pkg = {
  name = "demo-tool",
  description = "Example raw release executable installed through rename.",
  licenses = { "MIT" },
  upstream = "https://example.invalid/demo-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "github_release",
    repo = "example/demo-tool",
    tag = "v1.0.0",
    asset = "demo-tool-linux-x86_64",
    sha256 = "8888888888888888888888888888888888888888888888888888888888888888",
    rename = "demo-tool",
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
