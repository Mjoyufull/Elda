-- 08-conffiles-and-state / example-config-pkg
--
-- Showcase recipe focused on declarative state metadata: conffiles,
-- alternatives, plus inline sysusers/tmpfiles. No init service is published
-- (see 07- for that pattern).
pkg = {
  name = "example-config-pkg",
  description = "Reference recipe demonstrating conffiles, alternatives, and state files.",
  licenses = { "Apache-2.0" },
  upstream = "https://example.invalid/example-config-pkg",
  epoch = 0,
  version = "0.9.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "url_archive",
    url = "https://example.invalid/example-config-pkg/0.9.0/example-config-pkg-0.9.0.tar.gz",
    sha256 = "6666666666666666666666666666666666666666666666666666666666666666",
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

  -- Conffiles are protected on upgrade (`*.eldasave` semantics, SPEC §10.2).
  conffiles = {
    "/etc/example-config/example.conf",
    "/etc/example-config/profile.d/local.conf",
  },

  sysusers = {
    { kind = "user",  name = "example-config", group = "example-config", system = true },
    { kind = "group", name = "example-data",  system = true },
  },

  tmpfiles = {
    { type = "d", path = "/var/lib/example-config", mode = "0755", user = "root", group = "root" },
    { type = "d", path = "/run/example-config",     mode = "0755", user = "example-config", group = "example-config" },
  },

  -- One symlink registered with the alternatives system.
  alternatives = {
    { name = "editor", link = "/usr/bin/editor", path = "/usr/bin/example-config-edit", priority = 50 },
  },
}
