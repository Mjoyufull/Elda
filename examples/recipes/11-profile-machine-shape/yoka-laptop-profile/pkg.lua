-- 11-profile-machine-shape / yoka-laptop-profile
--
-- First-class profile package (SPEC §5.2 + §7). `kind = "profile"` is the
-- machine-shape anchor: it declares native arch, foreign archs, and the init
-- backend the host runs on. `elda pf apply yoka-laptop` selects this profile;
-- `elda pf show` reports it.
pkg = {
  name = "yoka-laptop",
  description = "Yoka laptop profile (amd64 native, dinit init, no foreign arches).",
  licenses = { "Apache-2.0" },
  upstream = "https://yoka.invalid/profiles/yoka-laptop",
  epoch = 0,
  version = "0.2.0",
  rel = 1,
  arch = { "amd64" },
  kind = "profile",

  source = {
    kind = "git",
    url = "https://example.invalid/yoka-laptop-profile.git",
    branch = "main",
  },

  depends = {
    "base-files",
    "coreutils",
    "yoka-core",
  },
  makedepends = {},
  checkdepends = {},
  recommends = {
    "tlp",
    "powertop",
    "intel-ucode",
  },
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},

  conffiles = {},
  -- `sysusers`, `tmpfiles`, `alternatives`, `hooks`, and `provider_assets`
  -- are intentionally omitted: an empty `{}` is treated as a wrong-shape
  -- value by the validator. Leave them off when you have nothing to declare.

  flags_default = {
    laptop_power = true,
  },
  flags_allowed = {
    laptop_power = true,
  },

  -- Machine-shape policy attached to a `package_kind = profile` anchor.
  profile = {
    native_arch = "amd64",
    foreign_arches = {},
    init = "dinit",
  },
}
