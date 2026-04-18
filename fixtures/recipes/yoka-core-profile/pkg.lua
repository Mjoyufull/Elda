pkg = {
  name = "yoka-core",
  epoch = 0,
  version = "0.1.0",
  rel = 1,
  arch = { "amd64" },
  kind = "profile",

  source = {
    kind = "git",
    url = "https://example.invalid/yoka-core.git",
    branch = "main",
  },

  depends = {
    "base-files",
    "coreutils",
  },
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
  sysusers = {},
  tmpfiles = {},
  alternatives = {},
  hooks = {},

  flags_default = {},
  flags_allowed = {},
  flags_implies = {},
  flags_conflicts = {},

  subpackages = {},

  profile = {
    native_arch = "amd64",
    foreign_arches = { "i386" },
    init = "dinit",
  },
}
