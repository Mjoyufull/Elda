<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaasciidark.png#gh-dark-mode-only">
<img align="center" src="https://raw.githubusercontent.com/Mjoyufull/Elda/refs/heads/dev/assets/eldaascii.png#gh-light-mode-only">

<p align="center"><i>Elda universality in your choices</i></p>

<br><br>
<br>
Elda is a Unix-first, Linux-first package manager built to replace the split between "system package manager", "install from git", "vendor binary downloader", "foreign repo bridge", and "migration tool" with one coherent package manager.

It uses a binary-first, git-capable, CI-native model built around explicit package state, staged payloads, recorded ownership, deterministic verification, and recoverable transactions.
</div>

## About

Elda keeps the part people actually like from lightweight git-first tools: install something directly, keep the CLI simple, and do not force a maintainer workflow on every user action.

The difference is that Elda does not stop at "clone repo, run build script, copy some files." It treats direct git installs, maintained `pkg.lua` recipes, published binary releases, foreign repositories, meta packages, adopted systems, and machine-shape profiles as parts of the same package manager with the same state model.

That means:

- one package identity model
- one dependency model
- one install database
- one manifest and verification model
- one transaction and rollback story
- one CLI surface

## Feature Checklist
check [checklist](./checklist.md) for the dev checklist
### Package acquisition

- [x] Install maintained packages with `elda i`.
- [x] Force source or binary lanes with `elda ig`, `elda ib`, `--prefer-source`, and `--prefer-binary`.
- [x] Keep one package identity even when a package ships both source and binary acquisition lanes.
- [x] Install directly from git URLs instead of requiring a curated package first.
- [x] Pull release binaries from `url_archive` and `github_release` sources.
- [x] Import one-off vendor binaries without turning them into a second-class workflow.
- [x] Treat packaged binaries, direct git installs, vendor binaries, foreign repos, and adopted packages as one package-manager domain instead of separate tools.

### Package definitions

- [x] Use `pkg.lua` as the main package-definition format.
- [x] Keep `build.lua` optional for the packages that genuinely need imperative logic.
- [x] Support one maintained definition for both source and binary lanes.
- [x] Model dependencies, weak dependencies, provides, conflicts, replaces, flags, conffiles, hooks, `sysusers`, `tmpfiles`, and alternatives in package metadata.
- [x] Support normal packages, meta packages, and profile packages as first-class package kinds.
- [x] Support split-package output from one staged build.
- [x] Preserve legacy `pkgit` import as a compatibility lane instead of making it the long-term runtime model.

### Build and payload model

- [x] Stage every build into a controlled package root instead of copying files from a live checkout.
- [x] Emit canonical `.pkg.tar.zst` payloads plus manifests, signatures, SBOMs, and attestations.
- [x] Use declarative common-case build definitions for systems like Cargo, CMake, Meson, Go, Zig, Python, and Make.
- [x] Keep build isolation explicit through host, isolated, and remote build backends.
- [x] Run post-stage analysis for manifests, shared-library metadata, split allocation, and verification data.
- [x] Publish the same payload shape from local builds and CI builds.

### Dependency and solver behavior

- [x] Use canonical `epoch:pkgver-pkgrel` ordering.
- [x] Resolve against synced snapshots instead of guessing from live repos.
- [x] Treat hard deps, weak deps, provider choice, pinning, and holds as part of one solver model.
- [x] Expose `why`, `rdeps`, `pin`, `unpin`, `hold`, `unhold`, `downgrade`, and `autoremove` as normal operator tools.
- [x] Refuse resolver-broken partial upgrades.
- [x] Keep exact package names, versioned dependencies, virtual provides, and multiarch identities explicit.

### Installed state and safety

- [x] Record installed state in SQLite instead of using cloned repositories as the database.
- [x] Track path ownership per package and per manifest entry.
- [x] Verify files, symlinks, metadata, and conffiles against recorded state.
- [x] Journal install, remove, upgrade, and repair operations.
- [x] Support explicit `recover` and backend-aware `rollback`.
- [x] Handle conffiles deterministically with `*.eldanew` and `*.eldasave` semantics.
- [x] Keep one global mutation lock so package mutations stay transactional.
- [x] Fail loudly on unmanaged path collisions and unsafe ownership takeover.

### Machine shape and system management

- [x] Treat profiles as first-class install targets instead of external setup scripts.
- [x] Apply base machine shape with `pf apply`.
- [x] Report active profile anchors, provider families, pending handlers, and activation class with `pf show`.
- [x] Track world anchors, base packages, dependency packages, and adopted packages explicitly.
- [x] Export and import desired machine shape with `state show`, `state export`, and `state import`.
- [x] Support explicit init-provider, multilib, and machine-policy transitions as typed system changes.
- [x] Keep prefix mode and system mode under the same conceptual package manager.

### Remotes, caches, trust, and offline behavior

- [x] Register remotes and caches explicitly instead of hiding them in ad hoc repo lists.
- [x] Keep metadata remotes separate from payload caches.
- [x] Sync into verified snapshots with `elda sync`.
- [x] Support multiple remotes, multiple caches, priorities, and freshness policy.
- [x] Support pinned keys or explicit TOFU bootstrap for remotes.
- [x] Prefer caches for payload delivery while keeping metadata authoritative from remotes.
- [x] Allow offline operation against verified snapshots and cached payloads by policy.

### Foreign packages and migration

- [x] Support interbuild frontends such as `nix_flake` and `gentoo_overlay` in git mode.
- [x] Support interepo adapters that translate foreign repositories into native Elda metadata.
- [x] Install translated foreign packages through the same resolver and transaction engine as native packages.
- [x] Adopt whole systems with `mg from <pm>`.
- [x] Adopt individual packages with `adopt --from <pm> <pkg>`.
- [x] Preserve provenance for adopted packages instead of pretending they were native from the start.
- [x] Support coexist, warn, and lock modes for migration away from another package manager.

### CI, forge, and publishing

- [x] Use PR/MR-first submission instead of hidden upload magic.
- [x] Submit maintained packages with `ci sub` and `ci run`.
- [x] Model package-definition repos, build DAGs, lock records, and topological build layers explicitly.
- [x] Publish binaries, manifests, signatures, SBOMs, attestations, and index updates from CI.
- [x] Default normal installs to the published binary lane when one exists.
- [x] Keep forge discovery separate from the solver with `forge search` and `forge browse`.
- [x] Support stack or batch submission for large desktop and platform closures.

### Ops, QA, and extensions

- [x] Expose `check`, `verify`, `reverify`, `fix-triggers`, and `diff` as normal maintenance commands.
- [x] Expose daemon control with `daemon run`, `daemon status`, and `daemon refresh`.
- [x] Support QA entrypoints such as `qa lint`, `qa build`, `qa smoke`, `qa stack`, `qa repro`, and `qa diff`.
- [x] Keep the extension model bounded, explicit, and capability-scoped.
- [x] Support activation backends, build backends, object analyzers, boot backends, interepo adapters, migration adapters, and provider migrators without letting plugins redefine package-manager semantics.

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
  flags_default = {},
  flags_allowed = {},
  flags_implies = {},
  flags_conflicts = {},
  subpackages = {},
}
```

## Configuration At A Glance

```toml
[defaults]
remote = "yoka-main"
cache_policy = "prefer"
origin_style = "tag"
install_preference = "binary"
build_fallback = "local"
build_mode = "isolated"
activation = "auto"
prefix = "/usr"
allow_system_mode = false
snapshot_tool = "snapper"
install_recommends = true
refresh_weak_deps = false

[privilege]
provider = "auto"
interactive = true

[profile]
base = "yoka-core"
native_arch = "amd64"
foreign_arches = ["i386"]
init = "dinit"

[submission]
mode = "pr"
auto_open = true

[daemon]
refresh = "30m"
notify_upgrades = true
```

## Docs

- `USAGE.md` covers the command-line flows.
- `eldaforgehosting.md` covers operator-side native forge, index, and cache hosting.
