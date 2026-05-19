-- 07-system-service-providers / example-daemon
--
-- Long-running system service that publishes init-system service files for
-- every supported provider through `provider_assets`. Provider files live as
-- companion files under the recipe tree and are referenced explicitly; Elda
-- does not guess by filename.
--
-- Layout on disk:
--   /etc/elda/recipes/example-daemon/
--     pkg.lua
--     providers/init/dinit/example-daemon         (single file)
--     providers/init/runit/example-daemon/        (run/finish/log directory)
--
-- Activation runs the configured init backend (see `[profile].init`) and
-- writes only the matching provider's assets into the system root.
pkg = {
  name = "example-daemon",
  description = "Reference long-running daemon used to demonstrate provider_assets.",
  licenses = { "Apache-2.0" },
  upstream = "https://example.invalid/example-daemon",
  epoch = 0,
  version = "1.4.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "github_release",
    repo = "example-org/example-daemon",
    tag = "v1.4.0",
    asset = "example-daemon-v1.4.0-linux-amd64.tar.zst",
    sha256 = "5555555555555555555555555555555555555555555555555555555555555555",
    binary = "example-daemon",
  },

  depends = {
    "glibc>=2.38",
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

  conffiles = {
    "/etc/example-daemon/example-daemon.conf",
  },

  sysusers = {
    { kind = "user", name = "example-daemon", group = "example-daemon", system = true },
  },

  tmpfiles = {
    {
      type = "d",
      path = "/var/lib/example-daemon",
      mode = "0750",
      user = "example-daemon",
      group = "example-daemon",
    },
    {
      type = "d",
      path = "/var/log/example-daemon",
      mode = "0750",
      user = "example-daemon",
      group = "example-daemon",
    },
  },

  -- `alternatives` intentionally omitted: an empty `{}` is treated as a
  -- wrong-shape value by the validator. Leave the field off when you have
  -- nothing to declare; populate it as in 08-conffiles-and-state when you do.

  hooks = {
    -- Lifecycle hooks are exceptional. Prefer declarative metadata above.
    post_install = { file = "hooks/post_install.lua" },
  },

  provider_assets = {
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/example-daemon",
          file = "providers/init/dinit/example-daemon",
        },
      },
      runit = {
        {
          kind = "tree",
          target = "/etc/sv/example-daemon",
          dir = "providers/init/runit/example-daemon",
        },
      },
    },
  },
}
