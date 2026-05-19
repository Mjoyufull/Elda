<div align="center">

<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaasciidark.png#gh-dark-mode-only">
<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaascii.png#gh-light-mode-only">

<p><i>Elda universality in your choices</i></p>

[![language: Rust](https://img.shields.io/badge/language-Rust-red.svg?style=flat-square)](https://www.rust-lang.org/)
[![repository](https://img.shields.io/badge/repo-Mjoyufull%2FElda-blue?style=flat-square)](https://github.com/Mjoyufull/Elda)

</div>

Elda is a Unix-first, Linux-first package manager by Rikona
([@Mjoyufull](https://github.com/Mjoyufull)). It is built to replace the split
between a system package manager, direct git installer, vendor binary downloader,
foreign repository bridge, and migration tool with one coherent package-manager
state model.

Elda uses explicit package metadata, signed remotes, staged payloads, recorded
ownership, deterministic verification, recoverable transactions, and source or
binary lanes under one package identity.

## Status

Elda is active development software. The current runtime has a real local and
disposable-root package-manager slice, including source builds, binary lanes,
remotes, cache lookup, verification, rollback, profiles, bounded interbuilds,
local CI/forge tooling, and migration/adoption groundwork. It is not yet a final
live-system replacement release.

For exact behavior and implementation status, read:

- [SPEC.md](./SPEC.md) - behavior contract
- [USAGE.md](./USAGE.md) - operator workflows and examples
- [phase.md](./phase.md) - implementation ledger and phase status
- [checklist.md](./checklist.md) - development tracker
- [eldaforgehosting/](./eldaforgehosting/README.md) - native forge, remote, cache, and publish hosting

## Building Elda

### What you need

| Requirement | Notes |
| --- | --- |
| **Rust** | **1.94 or newer** (`rust-version` in the workspace `Cargo.toml`). Install with [rustup](https://rustup.rs/). |
| **Cargo** | Ships with rustup. |
| **C toolchain** | `gcc` or `clang`, plus `make`, for native Rust dependency builds (`liblzma`, `zstd`, etc.). |
| **pkg-config** | Used when linking system libraries for compression crates. |
| **libzstd** | Development headers for the `zstd` crate (`libzstd-dev`, `zstd` on Arch). |
| **liblzma** | Development headers for `liblzma` in `elda-build` (`liblzma-dev`, `xz` on Arch). |
| **git** | Required to clone the repo; also used heavily by the test suite. |

Elda does **not** need CMake, Meson, Go, Zig, Nimble, or other build tools on the **host that compiles Elda**. Those are only needed on machines where you **build packages through Elda** (source-lane recipes).

Example distro packages:

```sh
# Arch Linux
sudo pacman -S --needed base-devel git rust pkgconf zstd xz

# Debian / Ubuntu
sudo apt install build-essential git pkg-config libzstd-dev liblzma-dev
# Then install Rust 1.94+ with rustup if the distro rustc is too old.
```

### Build and install the binary

```sh
git clone https://github.com/Mjoyufull/Elda
cd Elda
cargo build --release
```

Run from the workspace while developing:

```sh
cargo build
./target/debug/elda --help
```

Install the binary somewhere on your PATH when you are ready to use it outside
the checkout:

```sh
install -Dm0755 target/release/elda ~/.local/bin/elda
```

## Quick Usage

```sh
# Inspect the CLI (see also examples/ and USAGE.md)
elda --help
elda i --help
elda rmt add --help

# Register a native remote and sync (illustrative URLs and keys—not a real remote)
elda rmt add yoka-main=https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --packages-url https://github.com/Mjoyufull/Elda.git
elda sync

# Search and install
elda search fsel
elda i fsel
elda ig fsel      # force source lane
elda ib fsel      # force binary lane

# Add local metadata without overwriting existing metadata
elda a https://github.com/Mjoyufull/fsel
elda a https://github.com/Mjoyufull/fsel --replace

# Dynamic interemote: --exclude must come at the END of the flag list; it consumes
# all trailing package names (spaces or comma-separated tokens).
elda rmt add heather-overlay=https://github.com/heather7283/heather7283-overlay --exclude firefox vlc
elda rmt add other-overlay=https://example.invalid/overlay.git --exclude firefox, vlc
elda rmt preview heather-overlay
elda sync heather-overlay

# Inspect state and bootstrap health
elda doctor
elda ls
elda info fsel
elda files fsel
elda files owner /usr/bin/fsel
elda verify fsel
```

For the full operator guide, use [USAGE.md](./USAGE.md). For hosting native indexes, forges, and caches end to end, see [eldaforgehosting/](./eldaforgehosting/README.md).

> [!WARNING]
> **Documentation examples:** URLs, remote names, signing keys, and third-party repository names used in this repository’s docs are **strictly illustrative** (e.g., example remotes and keys are not real) unless you recognize them as your own infrastructure. Replace them with your real index URLs, `packages_url`, trust material, and cache bases.

> [!TIP]
> **`examples/`:** The [examples/](./examples/) tree is the **primary** and most important place to learn real `pkg.lua` layouts, profile snippets, import inputs, and annotated `config.toml` fragments. Use it as your primary reference for available features.

## Dependencies

### Workspace crates

The Cargo workspace (`Cargo.toml`) builds these members:

| Crate | Role |
| --- | --- |
| `elda-cli` | `elda` binary, CLI parsing, help rendering, privilege re-exec |
| `elda-core` | Command dispatch, install/upgrade solver, human output, config |
| `elda-build` | Source/binary fetch, build systems, payload staging |
| `elda-install` | Transactions, activation, rollback, system backend |
| `elda-db` | SQLite installed-state database |
| `elda-recipe` | `pkg.lua` parse/validate/import/format |
| `elda-repo` | Remote indexes, sync, interemotes, trust |
| `elda-git` | Tag/release inspection (GitHub, GitLab, Gitea, …) |
| `elda-appimage` | AppImage inspect and staging helpers |
| `elda-populate` | Cache seeding / maintained-remote mirroring |
| `elda-linux` | Linux backend selection and trigger metadata |
| `elda-types` | Shared types and boundaries |
| `elda-fetch`, `elda-ext`, `elda-unix` | Reserved boundaries (minimal today) |
| `xtask` | Internal maintenance tasks |

### Rust libraries (Cargo)

Shared workspace dependencies:

| Crate | Used for |
| --- | --- |
| `anyhow` | CLI error context (`elda-cli`) |
| `clap` | CLI parsing |
| `serde` / `serde_json` | Reports, manifests, metadata |
| `toml` | `config.toml`, remote documents |
| `rusqlite` (bundled SQLite) | Installed-state DB |
| `pubgrub` | Install/upgrade dependency solver |
| `rustix` | Filesystem and process helpers |
| `sha2`, `base64`, `ed25519-dalek` | Checksums and release/index trust |
| `ureq` | HTTP(S) fetch for remotes and release assets |
| `zstd`, `tar`, `flate2` | Payload archives and compression |
| `regex` | Remote/index parsing |
| `tempfile`, `fs4` | Temp dirs and file locking |
| `thiserror` | Typed errors |
| `anstyle`, `spinners` | Terminal styling and live progress |

Additional dependencies outside the workspace table:

| Crate | Crate(s) | Used for |
| --- | --- | --- |
| `liblzma` | `elda-build` | `.xz` archive extraction |
| `goblin` | `elda-build`, `elda-appimage` | ELF / object inspection |
| `rnix`, `rowan` | `elda-recipe`, `elda-build` | Nix flake parsing (interbuild) |
| `backhand` | `elda-appimage` | AppImage payload access |

### Host programs (runtime, by feature)

These are **not** required to compile Elda. Install them when you use the matching recipe lane or command.

| Program | When Elda needs it |
| --- | --- |
| `git` | Git sources, `packages_url` remotes, interemotes, `elda git *`, CI publish, metadata import |
| `cargo`, `rustc` | Recipes with `build.system = "cargo"` |
| `cmake`, `ctest` | CMake recipes |
| `meson` (and usually **Ninja**, via Meson) | Meson recipes |
| `make` | Makefile recipes |
| `go` | Go recipes |
| `python3` | Python `setup.py` / wheel staging recipes |
| `zig` | Zig recipes |
| `nimble` | Nim/Nimble recipes |
| `bash` | Bounded AUR/XBPS interbuild shell parsing |
| `gh` | `elda forge fork` (GitHub CLI) |
| `diff`, `less` (or `$PAGER` / `$ELDA_PAGER`) | Review diffs and `rc edit` |
| `doas`, `sudo`, `run0`, or `su` | Live host `/usr` mode privilege escalation (one on `PATH`, or configure `[privilege].provider`) |
| `snapper` | Optional Btrfs snapshot hooks when configured |

Network access is required for `elda sync`, remote binary lanes, and release-asset fetch unless you use `--offline` with a verified local cache/snapshot.

## What Works Today

- `pkg.lua` recipes with source lanes, binary lanes, flags, weak deps, providers,
  conflicts, replaces, conffiles, hooks, `sysusers`, `tmpfiles`, alternatives,
  provider assets, meta packages, profile packages, and split package metadata.
- Source installs from git and local recipes, with build support for Cargo,
  CMake, Meson, Make, Go, Python, Zig, and Nimble in the current slice.
- Binary installs from URL archives, GitHub release assets, provider-neutral
  release assets, GPKG, AppImage, and vendor-generated recipes.
- Signed native remotes, explicit TOFU/pinned trust, channel-aware sync,
  source-capable `packages_url` remotes, cache-first payload lookup, offline
  verified snapshot use, and dynamic Gentoo/XBPS interemotes.
- Solver-backed install, remove, upgrade, downgrade, pin, hold, weak dependency,
  provider, conflict, and replacement handling.
- SQLite installed-state DB, file ownership, manifests, verification, recovery,
  conffile handling, prefix rollback, and the first disposable `/usr` backend.
- Profiles and machine shape through `pf`, state export/import, trigger
  inspection, config queue resolution, and system-provider asset visibility.
- Local native CI/forge workflows through `ci`, `forge`, `qa`, and local publish
  artifacts.
- Bounded interbuilds for Nix flakes, Gentoo overlays, AUR PKGBUILDs, and XBPS
  templates without invoking foreign package-manager CLIs in the parser path.
- Adoption/migration groundwork for pacman, apt/dpkg, apk, xbps, and portage
  installed-state import.

## Configuration

The sample config is [config.toml](./config.toml). Annotated examples live under
[examples/config](./examples/config) (including
[host.d/yoka.toml.example](./examples/config/host.d/yoka.toml.example) for
maintainers), and lean fixtures live under [fixtures/config](./fixtures/config).

Runtime paths:

```text
/etc/elda/config.toml
/etc/elda/remotes.d/*.toml
/etc/elda/caches.d/*.toml
/etc/elda/extensions.d/*.toml
/etc/elda/host.d/*.toml
/etc/elda/recipes/<pkgname>/pkg.lua
```

The [su/config.toml](./su/config.toml) file is a copyable `/etc/elda/config.toml`
example for hosts that use `su` as the privilege provider.

## Package Definition Example

```lua
pkg = {
  name = "fd",
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
        sha256 = "...",
        binary = "fd",
      },
    },
  },

  depends = {},
  recommends = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
  sysusers = {},
  tmpfiles = {},
  alternatives = {},
  hooks = {},
  provider_assets = {},
  flags_default = {},
  flags_allowed = {},
  flags_implies = {},
  flags_conflicts = {},
  subpackages = {},
}
```

More complete package examples are in [examples/recipes](./examples/recipes). Annotated configuration and fixture-style samples live under [examples/config](./examples/config).

## Documentation

- [SPEC.md](./SPEC.md) - product/runtime behavior contract
- [USAGE.md](./USAGE.md) - operator workflows and CLI examples
- [eldaforgehosting/](./eldaforgehosting/README.md) - native forge, remote, index, cache, and publish hosting
- [phase.md](./phase.md) - implementation order and current status
- [checklist.md](./checklist.md) - development tracker
- [man/elda.1](./man/elda.1) - man page source
- [examples/](./examples/) - recipes, config samples, and import fixtures

Report the running build with `elda -V` or `elda version` (release **0.1.49-Sumomo**).

## Development

```sh
cargo fmt --check
cargo test --workspace
cargo build
```

The full workspace test suite expects **git** on `PATH` and may invoke **zig**, **nimble**, **make**, and **sh** for build-system integration tests. You do not need every host build tool unless you run tests or installs that exercise that lane.

Follow [CODE_STANDARDS.md](./CODE_STANDARDS.md) when changing Rust code.
