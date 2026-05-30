<div align="center">

<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaasciidark.png#gh-dark-mode-only">
<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaascii.png#gh-light-mode-only">

<p><i>Elda universality in your choices</i></p>

[![language: Rust](https://img.shields.io/badge/language-Rust-red.svg?style=flat-square)](https://www.rust-lang.org/)
[![repository](https://img.shields.io/badge/repo-Mjoyufull%2FElda-blue?style=flat-square)](https://github.com/Mjoyufull/Elda)

<br>

A Unix-first, Linux-first system package manager written in Rust.<br>
Binary-first delivery, git-capable sources, signed remotes - one ledger for every path under `/usr`.

</div>

## Table of Contents

- [Features](#features)
- [Quick Usage](#quick-usage)
- [Status](#status)
- [Building Elda](#building-elda)
- [Dependencies](#dependencies)
- [Configuration](#configuration)
- [Package Definition Example](#package-definition-example)
- [Documentation](#documentation)

**Elda** is a Unix-first, Linux-first system package manager written in Rust. It manages `/usr` as installed state: one package identity and PubGrub-style solver (`epoch:pkgver-pkgrel`), one SQLite ledger for ownership and rollback, and one transaction path for signed remotes (including dynamic interemotes), Lua recipes (`pkg.lua`, optional `build.lua`), git upstreams, vendor/release binaries, interbuild imports (Nix flakes, Gentoo overlays, AUR, XBPS), and interepo foreign indexes. Host backends handle activation, triggers, conffiles, and boot policy on each supported Unix target. Hard fork of [pkgit](https://git.symlinx.net/pkgit/about/); full contract in [SPEC.md](./SPEC.md).

## Features

- **Lua recipes** - `pkg.lua` / `build.lua`, flags, deps, hooks, profiles, conffiles, `provider_assets`, split packages
- **Signed remotes** - `sync` (all or named remotes), channels, TOFU/pinned trust, key rotation, cache-first payloads
- **Interemotes** - Gentoo overlay / XBPS `srcpkgs` git remotes with `rmt preview`, `--exclude`, sync deltas and parser diagnostics
- **Source and binary lanes** - same package name; `ig` / `ib`, release assets, AppImage, GPKG, vendor-generated recipes
- **Foreign packaging** - interbuild parsers (Nix, Gentoo, AUR, XBPS) and interepo adapters into one staging model
- **SQLite ledger** - ownership, manifests, `files` / `files search`, verify/recover, prefix and `/usr` rollback
- **Operator surfaces** - install preflight, live progress, review stamps, `doctor`, `config pending`/`apply`, `trigger ls/info`
- **Recipe & git ops** - `rc show` / `diff` / `publish-ready`, `vendor` import/export, `git releases` / tags, metadata `add` with `--replace`
- **Policy & introspection** - pin/hold, `why` / `rdeps` / `autoremove`, downgrade, `fl check` / `diff`, provider preferences
- **Forge publishing** - `ci` / `forge` / `qa`, hosted `ci pr`, signed indexes, populate cache mirror/push
- **Init-agnostic** - provider assets for systemd, OpenRC, runit, dinit, and similar; no mandated init
- **Staged transactions** - build -> stage -> verify -> activate with journaled mutations on system backends

## Quick Usage

```sh
# Inspect the CLI (see also examples/ and USAGE.md)
elda --help
elda i --help
elda rmt add --help

# Register a native remote and sync (illustrative URLs and keys - example remote only)
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

# Dynamic interemote: --exclude must come at the END of the flag list
elda rmt add heather-overlay=https://github.com/heather7283/heather7283-overlay --exclude firefox vlc
elda rmt preview heather-overlay
elda sync heather-overlay

# Inspect state and bootstrap health
elda doctor
elda ls
elda info fsel
```

For the full operator guide, use [USAGE.md](./USAGE.md). For hosting native indexes, forges, and caches end to end, see [eldaforgehosting/](./eldaforgehosting/README.md).

> [!WARNING]
> **Documentation examples:** URLs, remote names, signing keys, and third-party repository names used in this repository's docs are **strictly illustrative** unless you recognize them as your own infrastructure. Replace them with your index URLs, `packages_url`, trust material, and cache bases.

> [!TIP]
> **`examples/`:** The [examples/](./examples/) tree is the **primary** reference for `pkg.lua` layouts, profile snippets, import inputs, and annotated `config.toml` fragments.

## Status

**Overall (toward full [SPEC.md](./SPEC.md) scope): ~68%**

| Track | Progress | Notes |
| --- | ---: | --- |
| Core PM (recipes, solve, install, state, remotes, build, forge) | **~100%** | Includes interemotes, channels, cache policy, vendor/git release lanes, disposable roots + `/usr` backend |
| Operator UX (review, doctor, progress, inspection) | **~85%** | Preflight, live progress, review memory, `files`/`config`/`trigger`/`rc` surfaces; setup/takeover still thin |
| Interepo (foreign index -> Elda install) | **~15%** | Architecture + bounded pieces; adapters/coexistence not done |
| Migration / pkgit retirement | **~25%** | DB import groundwork; live takeover/coexistence not done |
| Host activation (merged tree, atomic switch) | **0%** | Not in runtime yet |

Active development. Native slice covers install/upgrade/remove, signed remotes and interemotes (`preview`, targeted `sync`), review gates, inspection commands, forge/CI publish, and bounded interbuild/migration import; interepo install and merged-tree host activation are not. Prefer disposable roots; treat live `/usr` as experimental. Ledger detail: [phase.md](./phase.md).

### Feature checklist (current tree)

Legend: `[x]` done in the current slice · `[~]` partial · `[ ]` not started

- [x] `pkg.lua` / `build.lua` - parse, validate, install; flags, provider assets, meta/profile packages
- [x] Source lanes - git, local recipes, synced `packages_url` trees; Cargo, CMake, Meson, Make, Go, Python, Zig, Nimble
- [x] Binary lanes - URLs, forge `release_asset`, GPKG, AppImage + `appimage inspect`, vendor add/import/export
- [x] Signed remotes - TOFU/pinned trust, channels, key rotation, release-asset signature keys, offline snapshots, cache priority and cleanup
- [x] Interemotes - dynamic Gentoo/XBPS git remotes, `rmt preview`/`trust`/`info`, `--exclude`, `sync <remote>` deltas
- [x] Solver - install/upgrade/downgrade; pins, holds, providers, weak deps, replaces; `why` / `rdeps` / `autoremove`
- [x] SQLite state - ownership, manifests, `files search`, verify/recover, conffile queue, prefix rollback
- [x] `/usr` backend - staged activation, triggers, `fix-triggers`, provider assets, archive rollback (not final host model)
- [x] Profiles - `pf` edit/apply, machine shape, init/foreign-arch policy, `state export`/`import`
- [x] Inspection - `rc show`/`diff`/`publish-ready`, `config pending`/`diff`/`apply`, `trigger ls/info`, `doctor`
- [x] Review - generated-metadata and interbuild gates, `review ls/info/forget/diff`, content-addressed stamps
- [x] Forge - `ci`/`forge`/`qa`, local publish (lock/index/sidecars), hosted `ci pr` with token/bearer auth; `elda-populate` cache mirror
- [x] Interbuild - bounded Nix flake, Gentoo overlay, AUR PKGBUILD, XBPS template parsers + review metadata
- [~] Operator bootstrap - preflight, live progress, privilege handoff; full setup/takeover still open
- [~] Migration - `mg from` / `adopt` for pacman, apt, apk, xbps, portage DBs; no live file takeover yet
- [ ] Interepo: translated foreign snapshots install end-to-end
- [ ] Coexistence / lock / live takeover modes vs other package managers
- [ ] Host activation: merged-tree materialization and atomic `/usr` exchange (see [phase.md](./phase.md))

**More detail:** full kanban and phase tables in **[checklist.md](./checklist.md)**. Contract: [SPEC.md](./SPEC.md). Changelog: [phase.md](./phase.md).

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
| `elda-git` | Tag/release inspection (GitHub, GitLab, Gitea, ...) |
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
| `toml` | Configuration and remote documents |
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
| `snapper` or `btrfs` | Optional Btrfs snapshot hooks when configured |

Network access is required for `elda sync`, remote binary lanes, and release-asset fetch unless you use `--offline` with a verified local cache/snapshot.

## What works today

Roughly, the current tree can:

- Parse and install `pkg.lua` recipes (deps, conflicts, replaces, conffiles, hooks, profiles, meta packages, and the rest of the metadata surface in the spec).
- Build from git/local source (Cargo, CMake, Meson, Make, Go, Python, Zig, Nimble) or pull binaries (URLs, GitHub/GitLab/Gitea releases, GPKG, AppImage, generated vendor recipes).
- Sync signed remotes, pin trust, use caches/offline snapshots, and wire dynamic Gentoo/XBPS-style interemotes.
- Plan installs with the solver (upgrade/downgrade, pins, holds, providers, weak deps).
- Keep state in SQLite: ownership, manifests, verify, rollback, conffiles, disposable `/usr` tests, first real host backend.
- Run local `ci` / `forge` / `qa` / publish flows; import bounded Nix/Gentoo/AUR/XBPS metadata; start reading foreign PM databases for migration.

Gaps and ordering live in [phase.md](./phase.md) - that file is the honest checklist, not marketing copy.

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
example for hosts that use `su` as the privilege provider. `[trust].release_keys`
holds trusted release-asset signing keys; recipes that declare signature sidecars fail closed
when the needed key is missing unless an interactive operator imports it at the install gate.

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
- [CONTRIBUTING.md](./CONTRIBUTING.md) - how to contribute (setup, PRs, testing)
- [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) - branching, releases, and review workflow
- [CODE_STANDARDS.md](./CODE_STANDARDS.md) - Rust structure, quality, and testing standards
- [RELEASE_LOG.md](./RELEASE_LOG.md) - public release notes and GitHub release body source
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
