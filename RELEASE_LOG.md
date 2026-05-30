# Elda Release Log

This file is the public release-page source for Elda. It keeps the GitHub release body for the current tagged release and a condensed history of the development releases that led to it.

Repository: https://github.com/Mjoyufull/Elda

Current release title: `[0.1.49-Sumomo]`

Current git tag: `0.1.49`

## [0.1.49-Sumomo] Latest

### Breaking Changes

- First public Sumomo baseline
  - `0.1.49` is the first tagged public release for the Rust Elda line.
  - Development roots, caches, journals, and SQLite databases from older unreleased builds should be backed up or recreated before testing this tag.
  - Elda is pre-1.0. The package-manager paths below are real, but final live-host takeover is not done.
- System mode scope
  - Elda has a first `/usr` backend slice, but the final host-state materialization model is still not implemented.
  - Prefer disposable roots and prefix roots for this release. Treat live `/usr` use as experimental.
- pkgit compatibility
  - Elda keeps the useful pkgit-style direct git workflow, but it does not use pkgit internals as runtime state.
  - `pkgdeps`, `bldit`, flat repo lists, cloned repositories as state, and raw copy/symlink installs are import or reference material only.
- First-time config
  - Copy the root `config.toml` for first-time setup.
  - `examples/config/config.toml` is an annotated reference with non-default choices. Review it before using it as a live host config.

### Added

- Workspace and binary
  - Added the Rust workspace and the `elda` CLI binary.
  - Added crate boundaries for CLI, core runtime, recipes, repos, builds, install transactions, SQLite state, git inspection, AppImage handling, cache population, Linux helpers, Unix helpers, extension hooks, shared types, and maintainer tasks.
  - Added `elda -V` and `elda version` with workspace version, schema, git commit, target, profile, and build date details.
- Package definitions
  - Added `pkg.lua` parsing and validation for local recipes, maintained recipes, profile packages, meta packages, source lanes, binary lanes, dependencies, weak dependencies, conflicts, provides, replaces, conffiles, hooks, sysusers, tmpfiles, alternatives, provider assets, flags, subpackages, and profile policy.
  - Added `build.lua` support for explicit build logic.
  - Added `rc add`, `rc check`, `rc show`, `rc diff`, `rc publish-ready`, and editor-backed local recipe flows.
  - Added pkgit-style import for `pkgdeps` and `bldit` inputs into Elda-native metadata.
- Source and binary lanes
  - Added source builds for git and local recipes with Cargo, CMake, Meson, Make, Go, Python, Zig, and Nimble support.
  - Added binary lane staging for URL archives, GitHub release assets, provider-neutral release assets, GPKG payloads, AppImages, and vendor-generated recipes.
  - Added arch-specific release asset tables for maintained `github_release` recipes.
  - Added direct git installs with commit-derived installed versions.
  - Added explicit branch, tag, and revision selectors for git add, install, upgrade, and downgrade flows.
- AppImage lane
  - Added `source.kind = "appimage"` validation and staging.
  - Added stable launcher symlinks under `usr/bin/` and payload storage under `usr/lib/elda/appimages/`.
  - Added read-only AppImage inspection and desktop/icon/metainfo integration from the embedded SquashFS payload.
- Solver and policy
  - Added a Rust-native PubGrub-style solver for exact dependencies, versioned constraints, alternatives, virtual providers, conflicts, replaces, multi-target closure checks, and reverse-dependency safety.
  - Added provider preferences through `[resolver.provider_preferences]`.
  - Added global, profile, package, version-scoped package, and one-shot CLI flag layers.
  - Added flag descriptions, implied flags, conflicting flags, required-one-of, required-at-most-one, required-any-of groups, and conditional dependencies.
  - Added `variant_id` recording, source fallback for customized builds, and binary-lane blocking when a customized variant has no matching source lane.
  - Added pin, hold, weak dependency refresh, and variant-drift rebuild behavior.
- State and transactions
  - Added SQLite installed-state storage with world anchors, dependency records, manifests, file ownership, policy state, provenance, journals, current state, and archived state metadata.
  - Added manifest-backed install, remove, verify, reverify, recover, files, owner lookup, diff, rollback, and downgrade flows.
  - Added conffile handling with `.eldanew` and `.eldasave` files.
  - Added unmanaged-path collision checks before activation.
  - Added archive-backed prefix rollback from cached payloads and manifests.
- Remotes, trust, and cache
  - Added remote and cache documents under `/etc/elda/remotes.d/` and `/etc/elda/caches.d/`.
  - Added signed remote index verification with pinned trust, TOFU trust, persisted key state, verified snapshots, stale snapshot fallback, offline sync, channel filtering, and explicit rotated-key acceptance.
  - Added remote priority, channel selection, metadata URLs, package-definition URLs, and cache priority.
  - Added secure payload signature verification for synced binary installs.
  - Added cache-first binary installs, local content-addressed payload cache reuse, offline reinstall after a successful fetch, and cache cleanup that keeps installed and rollback payloads.
  - Added `rmt ls`, `rmt add`, `rmt rm`, `rmt info`, `rmt trust`, `rmt preview`, `rmt enable`, `rmt disable`, `rmt set-priority`, and targeted `sync <remote...>`.
- Interemotes and interbuilds
  - Added dynamic Gentoo overlay and XBPS `srcpkgs` interemotes with preview, excludes, parser diagnostics, sync deltas, and targeted sync.
  - Added bounded parser-backed source-lane installs for Nix flakes, Gentoo overlays, AUR PKGBUILDs, and XBPS templates.
  - Added metadata-only raw-link handling through `elda a` and `elda add`, with strategy priority, list-options mode, explicit source option selection, field confidence, and review gates.
  - Added bounded report extraction for Gentoo, AUR, and XBPS phase commands, dependencies, licenses, checksums, arch-specific sources, VCS sources, and parser issues.
- Git and releases
  - Added `elda git tags`, `elda versions`, and `elda git releases` for tag inspection, normalized version confidence, optional release joins, and release asset compatibility scoring.
  - Added GitHub, GitLab, Gitea, Forgejo, SourceHut, Codeberg, and direct manifest release asset classification.
  - Added checksum sidecar matching for release assets, including uppercase `SHA256SUMS` and per-asset sums files.
  - Added release signature sidecar parsing, metadata materialization, field validation, and install-time trust gating.
- Profiles and machine shape
  - Added `pf show`, `pf apply`, `pf add`, `pf rm`, `pf set-init`, `pf clear-init`, `pf set-arch`, `pf add-foreign-arch`, and `pf remove-foreign-arch`.
  - Added profile recipes with machine-shape defaults for native arch, foreign arches, and init provider.
  - Added profile conflict checks and state export/import round trips for active profiles, init policy, native arch, and foreign arch policy.
  - Added pending system-change reports for init-provider changes, foreign-arch policy, and unapplied profile selections.
- Linux system backend
  - Added the first `/usr` backend slice with staged state roots, per-path file-switch activation, `system-*` state IDs, archived system metadata, internal trigger state, boot status, and `linux-copy` activation reporting.
  - Added sysusers, tmpfiles, alternatives, hooks, provider assets, and active init-provider materialization for disposable system roots.
  - Added `fix-triggers`, provider-asset visibility in `info`, and system rollback coverage for disposable roots.
  - Added snapshot request recording for install and remove transactions, with current support for `snapper` and direct `btrfs subvolume snapshot -r`.
- Migration and adoption
  - Added `elda adopt --from <pm> <pkg>` and `elda mg from <pm>`.
  - Added bounded installed-state readers for pacman, apt/dpkg, apk, xbps, and portage.
  - Added adopted package records with provenance, identity, version, architecture, file list, dependency text, source hints, and path-conflict checks.
  - Added human and JSON reports for migration and adoption.
- CI, forge, QA, and cache population
  - Added local filesystem-backed `ci sub`, `ci run`, `ci status`, `ci pr`, `ci retry`, `ci logs`, and `ci batch new/add/push`.
  - Added signed local index publishing, `lock-v1.json.zst`, copied artifacts, minisign sidecars, SPDX sidecars, attestation sidecars, and indexed metadata URLs.
  - Added hosted review creation for GitHub, GitLab, and Gitea-style APIs with token, bearer, SSH, and no-auth modes where applicable.
  - Added `forge search`, `forge browse`, `qa lint`, `qa build`, `qa smoke`, `qa stack`, `qa repro`, `qa diff`, and `daemon run` current-slice behavior.
  - Added `elda-populate` cache seeding with local installed payload push and synced remote mirror support.
- Human output and review gates
  - Added framed human output for install, failure, search, info, list, recipe catalog, state, profiles, remotes, cache, config, triggers, migrations, review, and CI reports.
  - Added live progress for install flows with TTY tree output, plain stream output, JSON event streams, and `--no-stream` final-document behavior.
  - Added build output passthrough for source-lane builds when streaming.
  - Added generated metadata and interbuild review gates with content-addressed stamps, pager support, edit recheck loops, and `review ls`, `review info`, `review diff`, and `review forget`.
  - Added `doctor`, install dry-run preflight, release-readiness checks, conffile surfaces, privilege posture reporting, and post-transaction advisories.

### Changed

- Elda is now a Rust package manager with explicit state, transactions, manifests, verification, rollback, signed remotes, and staged activation.
- Human output now uses command-specific reports instead of dumping generic machine reports.
- Root help now uses the Elda branded screen, grouped command sections, examples, and terminal styling.
- Installed provenance now uses canonical source kinds such as `local_recipe`, `git`, `repo_binary`, `interbuild`, and `adopted`.
- Privilege defaults now use `provider = "auto"` with doas, sudo, run0, and su detection.
- Live system mode now requires `defaults.allow_system_mode = true` or one-shot `elda -S`.
- JSON and unattended sync no longer enroll TOFU trust implicitly.
- Offline sync now uses only cached verified snapshots.
- `--offline` now applies to git and archive fetch paths.

### Fixed

- Fixed dependency closure planning, weak dependency policy, provider ambiguity, conflicts, replaces, reverse-dependency checks, and versioned provider requests.
- Fixed duplicate install rendering when live progress already emitted the progress block.
- Fixed unmanaged terminfo handling for the current system backend.
- Fixed stale package records after remote removal.
- Fixed local metadata import so existing `pkg.lua`, `build.lua`, patches, and source metadata are preserved unless `--replace` is explicit.
- Fixed synced source installs so companion files come from pinned `packages_url` trees instead of only indexed `pkg_lua` text.
- Fixed adoption path checks so already-owned package identities and managed-path collisions fail before state is written.
- Fixed first-time config docs so users copy root `config.toml`, not the annotated example config.
- Fixed the man page entry for `[trust].release_keys` so it is described as a TOML key, not a fake path.
- Fixed test-only tree-style scoping so panic paths restore thread-local state.
- Fixed remove confirmation cleanup so stale `remove-confirm.json` cannot bypass a later interactive remove gate.
- Fixed current build warnings from unused code paths.

### Technical Details

- The workspace currently builds version `0.1.49-Sumomo` with Rust 1.94 or newer.
- Installed state is SQLite-backed.
- Payloads are staged before activation and recorded with payload and manifest digests.
- Prefix roots and disposable system roots are the expected test targets for this release.
- Remote indexes support pinned and TOFU trust. Verified snapshots are persisted per remote.
- Release-asset signature sidecars use `[trust].release_keys`.
- Mutations use journals and block recovery-required states.
- Unsupported paths fail clearly instead of falling back to pkgit-style behavior.
- Interepo translation is architectural and partial. End-to-end foreign-index installation is not complete.
- Migration and adoption import metadata and ownership records. Live takeover and coexistence lock modes are not complete.
- Host-state materialization and atomic `/usr` exchange are not implemented yet.

### Documentation

- Updated `README.md` for project identity, features, current status, build requirements, configuration, examples, and links.
- Updated `USAGE.md` for install, remotes, sync, review gates, trust, profiles, migration, CI, forge, cache, config, triggers, and first-time bootstrap.
- Updated `SPEC.md` for package identity, recipe schemas, flags, dependencies, state, transactions, remotes, trust, cache, interbuild, interepo, NixOS host refusal, release signatures, system activation, and CLI behavior.
- Updated `config.toml`, `su/config.toml`, `examples/`, and `fixtures/` for current config shape and test inputs.
- Added `man/elda.1` as the local man page source.
- Added `eldaforgehosting/` guides for native forge hosting, remotes, indexes, caches, publishing, platforms, and hosting patterns.
- Updated `PROJECT_STANDARDS.md`, `CODE_STANDARDS.md`, `CONTRIBUTING.md`, `checklist.md`, and `phase.md` for current release and contribution workflows.

### Notes

- SemVer: this is a `0.1.x` pre-1.0 release. By version number it is a patch release, but it is also the first public Sumomo baseline.
- Tag: `0.1.49`
- GitHub release title: `[0.1.49-Sumomo]`
- Repository: https://github.com/Mjoyufull/Elda

### Contributors

- `@Mjoyufull`

### Compatibility

- Rust: 1.94 or newer.
- Platform target: Linux-first and Unix-first. macOS and Windows are not implementation or delivery targets for this line.
- Config: root `config.toml` is the conservative first-time setup sample.
- Database and cache: no stable migration promise is made for unreleased development roots before this tag.
- System roots: prefix roots and disposable roots are recommended. Live `/usr` remains experimental.
- pkgit: Elda can import and replace parts of pkgit-style workflow, but pkgit is not a runtime dependency.

## Previously Done

These releases were development checkpoints before the first public Sumomo tag. They are listed here so the GitHub release page has the build-up history in one place.

### v0.1.48 - 2026-05-18

- Added Phase 13 planning for host-state materialization, atomic `/usr` promotion, ephemeral apply, and rename-based state archives.
- Kept the native CLI slice marked as closed for the named runtime surfaces while leaving later polish tracked separately.

### v0.1.47 - 2026-05-18

- Finalized the native hosting layout: recipe monorepo by default, one signed index per channel, dry-run test-tree flow, publish finalize URL rewrite, maintainer host profiles, and interemote handling at sync.

### v0.1.46 - 2026-05-19

- Froze interepo and hosting contract answers in `SPEC.md`, including foreign hook policy, RPM trigger input, ALPM target paths, SELinux handling, CachyOS microarch sync policy, NixOS host refusal, release-asset trust keys, SIGINT/recover behavior, and post-transaction advisories.

### v0.1.45 - 2026-05-18

- Split native forge hosting docs into `eldaforgehosting/` with guides for source-only remotes, binary remotes, caches, trust, interemotes, recipe-to-Git paths, supported forges, static HTTP, LAN mirrors, staging, and stable channels.

### v0.1.44 - 2026-05-16

- Refreshed the public operator docs, config samples, config fixtures, remote examples, cache examples, extension examples, interemote examples, and the local man page.

### v0.1.43 - 2026-05-13

- Started Phase 12 migration and adoption with `elda adopt --from <pm> <pkg>` and `elda mg from <pm>`.
- Added installed-state readers for pacman, apt/dpkg, apk, xbps, and portage.
- Added adopted package state, safety checks, and migration reports.

### v0.1.42 - 2026-05-08

- Extended the recipe flag system with descriptions, cardinality groups, conditional dependencies, atom-versioned overrides, variant IDs, `fl check`, `fl diff`, and variant-drift rebuilds.

### v0.1.41 - 2026-05-04

- Completed the Phase 10 git/release UX pass with SourceHut and direct manifest release inspection, provider-neutral release assets, matching signature sidecars, list-options TTY selection, and source-ref downgrade.

### v0.1.40 - 2026-05-04

- Made pinned ad hoc git upgrades show explicit keep-installed rows instead of hiding pinned no-ops.

### v0.1.39 - 2026-05-01

- Added `metadata.link_option_mode`, source-option reporting, `elda git tags`, `elda versions`, `elda git releases`, explicit git ref selectors, and checksum-backed release metadata conversion.

### v0.1.38 - 2026-04-30

- Added bounded AUR PKGBUILD and XBPS template support, config-backed metadata strategy priority, safer generated metadata, and richer interbuild parser reports.

### v0.1.37 - 2026-04-29

- Started Phase 10 with parser-backed Nix flake and Gentoo overlay source-lane installs.
- Added interbuild review gates, parser metadata, field provenance, and the metadata-first `elda a` / `elda add` path.

### v0.1.36 - 2026-04-25

- Closed the current Phase 9 scheduler slice with queued `ci run`, retry orchestration, richer status/log output, hosted-review configuration by remote, and clearer install progress shaping.

### v0.1.35 - 2026-04-23

- Closed the current Phase 8 proof gap for profile state import and active profile replay.
- Landed the first local Phase 9 CI, forge, QA, and daemon runtime slice.

### v0.1.34 - 2026-04-20

- Changed the `/usr` backend from direct copy-overwrite to staged per-path switch activation.
- Added backend capability and boot-trigger reporting.

### v0.1.33 - 2026-04-19

- Replaced recursive dependency planning with the Rust-native solver graph.
- Added provider policy, multi-target conflict handling, alternatives, dependency-driven upgrades, and weak dependency selection.

### v0.1.32 - 2026-04-19

- Added exact dependency and versioned provider handling, including rejection of unversioned virtual providers for versioned requests.

### v0.1.31 - 2026-04-17

- Added upgrade and install policy work around same-transaction replacements, reverse dependency checks, and provider resolution.

### v0.1.30 - 2026-04-17

- Added more resolver and transaction coverage for installed dependency upgrades and closure consistency.

### v0.1.29 - 2026-04-17

- Added package-level conflict and replacement behavior for install and upgrade planning.

### v0.1.28 - 2026-04-14

- Added remote removal cleanup, target-specific sync, remote trust inspection, remote info, interemote preview details, and sync package delta reporting.

### v0.1.27 - 2026-04-14

- Added selected remote channels, channel-filtered sync, and fail-closed offline/stale snapshot reuse across channel boundaries.

### v0.1.26 - 2026-04-14

- Added source-capable synced remotes through `packages_url` and pinned package-definition tree materialization.

### v0.1.25 - 2026-04-14

- Added `elda-populate` cache seeding for installed payloads and remote mirrors.

### v0.1.24 - 2026-04-14

- Added local cache retention, cache entry access metadata, garbage collection, and cache policy reporting.

### v0.1.23 - 2026-04-14

- Tightened first-use TOFU so unattended sync no longer auto-enrolls trust.

### v0.1.22 - 2026-04-14

- Added cache-node priority, cache-first synced binary installs, local payload cache population, and offline reinstall from cache.

### v0.1.21 - 2026-04-14

- Persisted remote trust keys and added secure remote payload signature verification for synced binary installs.

### v0.1.20 - 2026-04-13

- Added the branded root help screen and filled in command descriptions and argument help.

### v0.1.19 - 2026-04-13

- Removed hardcoded profile fallback shape and added real profile-state resolution and export/import persistence.

### v0.1.18 - 2026-04-12

- Reworked human rendering for high-value inspection commands and added GitHub release asset auto-selection for vendor binaries.

### v0.1.17 - 2026-04-12

- Added archived prefix-state capture and real `elda rollback`.

### v0.1.16 - 2026-04-12

- Added real `elda diff` and candidate manifest comparison.

### v0.1.15 - 2026-04-13

- Replaced install dry-run skeletons with closure-aware install plans, weak dependency handling, alternatives, virtual provides, conflicts, and closure-aware upgrades.
- Changed privilege defaults to `provider = "auto"` and added provider detection for doas, sudo, run0, and su.

### v0.1.14 - 2026-04-12

- Added live-host system-mode gating with `defaults.allow_system_mode` and the one-shot `elda -S` override.

### v0.1.13 - 2026-04-12

- Added desired-state export/import through the normal install path.

### v0.1.12 - 2026-04-12

- Added dependency edge storage, `why`, `rdeps`, `autoremove`, pin/hold policy, and reverse-dependency protection for remove.

### v0.1.11 - 2026-04-12

- Added real remote snapshot sync, search, info, named package resolution, and first prefix upgrade execution.

### v0.1.10 - 2026-04-11

- Added the first vendor binary workflow through `vendor add`, `vendor import`, and `vendor export`.

### v0.1.9 - 2026-04-11

- Added dual source/binary lane recipes, `ig`, `ib`, and lane preference handling.

### v0.1.8 - 2026-04-11

- Locked the maintained package contract for source and binary lanes in one package definition.

### v0.1.7 - 2026-04-11

- Added transaction journals, recovery, manifest-backed verify/reverify, and journal-aware mutation blocking.

### v0.1.6 - 2026-04-11

- Added the first git source build path, canonical staging, payload emission, manifest-backed prefix installs, file ownership, files queries, and remove.

### v0.1.5 - 2026-04-11

- Added remote/cache documents and the first structured dry-run plan skeletons.

### v0.1.4 - 2026-04-11

- Added real declarative `pkg.lua` parsing, validation, recipe scaffolding, direct git-target scaffolding, and pkgit-style source import.

### v0.1.3 - 2026-04-11

- Added the first `rc check` path for local recipe directories.

### v0.1.2 - 2026-04-11

- Added package identity/version parsing, config defaults, SQLite bootstrap, state layout, mutation lock, and structured empty-root output.

### v0.1.1 - 2026-04-11

- Landed the Rust workspace, crate skeleton, `xtask`, thin CLI binary, shared command/report types, and pkgit reference fixtures.

### v0.1.0 - 2026-04-02

- Created the implementation-order phase plan and locked the hard-fork boundaries.
