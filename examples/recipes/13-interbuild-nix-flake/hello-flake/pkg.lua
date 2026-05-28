-- 13-interbuild-nix-flake / hello-flake
--
-- Interbuild source: a maintained recipe whose source contract delegates to
-- an upstream Nix flake. The author-facing `kind = "nix_flake"` is normalized
-- into persisted `source_kind = interbuild` once Elda actually builds it
-- (SPEC §5.2 + §12.1).
--
-- Required:  url
-- Optional:  rev, lockfile, installable
pkg = {
  name = "hello-flake",
  description = "GNU hello, built through the upstream Nix flake adapter.",
  licenses = { "GPL-3.0-or-later" },
  upstream = "https://example.invalid/hello-flake",
  epoch = 0,
  version = "2.12.1",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "nix_flake",
    url = "github:example-org/hello-flake",
    rev = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    installable = ".#hello",
    lockfile = "flake.lock",
  },

  depends = {
    "glibc>=2.38",
  },
  makedepends = {
    "nix>=2.18",
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
