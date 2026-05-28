-- 05-multi-arch-binary / elda-cli
--
-- Multi-architecture github_release recipe. Each canonical Elda arch
-- (`amd64`, `arm64`, `i386`, `armhf`, `riscv64`, `ppc64le`) gets its own
-- entry under `assets`. Top-level `binary`/`strip_components` apply as
-- defaults; per-arch entries may override them.
--
-- Published metadata is always pinned per arch: each entry must carry
-- `asset` plus `sha256`, never runtime asset guessing.
pkg = {
  name = "elda-cli",
  description = "Elda command-line entrypoint published via GitHub releases.",
  licenses = { "Apache-2.0" },
  upstream = "https://github.com/Yoka-OS/elda",
  epoch = 0,
  version = "0.1.42",
  rel = 1,
  arch = { "amd64", "arm64", "riscv64" },
  kind = "normal",

  source = {
    kind = "github_release",
    repo = "Yoka-OS/elda",
    tag = "v0.1.42",
    binary = "elda",
    strip_components = 1,
    assets = {
      amd64 = {
        asset = "elda-0.1.42-x86_64-unknown-linux-gnu.tar.zst",
        sha256 = "1111111111111111111111111111111111111111111111111111111111111111",
      },
      arm64 = {
        asset = "elda-0.1.42-aarch64-unknown-linux-gnu.tar.zst",
        sha256 = "2222222222222222222222222222222222222222222222222222222222222222",
      },
      riscv64 = {
        asset = "elda-0.1.42-riscv64-unknown-linux-gnu.tar.zst",
        sha256 = "3333333333333333333333333333333333333333333333333333333333333333",
        binary = "elda",
      },
    },
  },

  depends = {
    "glibc>=2.38",
  },
  makedepends = {},
  checkdepends = {},
  recommends = {
    "git",
  },
  suggests = {},
  supplements = {},
  enhances = {},
  provides = { "elda" },
  conflicts = {},
  replaces = {},
  conffiles = {},
}
