# Elda - Phase Plan

**Version:** 0.1.0  
**Date:** 2026-04-02  
**Status:** Active  
**Spec:** `Elda/SPEC.md`  
**Fork Reference:** `pkgitfork.md`, `pkgit/`

## 1. Overview

This file defines the implementation order for Elda. It is not a second spec and it is not a scope-cut document. `Elda/SPEC.md` remains the contract. `phase.md` exists to say what lands in what order, what each phase has to prove, and where the hard fork from `pkgit` is handled directly.

Elda is a hard fork of `pkgit` in product direction and operator feel, not in code lineage. We keep the useful parts of `pkgit`'s UX and we replace the internals that stop it from being a real system package manager. That means Rust, SQLite, explicit state, explicit transactions, explicit manifests, explicit verification, and explicit rollback.

The phases here are implementation order only. They do not redefine the product downward. Elda is still the full system described in `Elda/SPEC.md`: native package definitions, binary-first delivery, transactional install state, CI publication, interbuild support, interepo translation, and migration/adoption.

### 1.1 Current Status

Elda is currently a real disposable-root CLI with a working staged build/install path, a first real `/usr` system-backend slice, real profile authoring/editing flow, and a materially more useful operator surface, not just a command skeleton.

What works right now:

- the `elda-core` runtime has been refactored out of the old giant dispatcher/test-file shape into focused `app_*.rs` modules, `app_install/*`, and split test modules under `crates/elda-core/src/tests/`, so the codebase now tracks the repo code standards materially better while preserving current behavior
- the current standards pass also split the old `elda-install`, `elda-db` store, `elda-cli` CLI/help and privilege path, `elda-types` package/version parsing, `elda-recipe` parser/vendor/check/import flow, `elda-repo` snapshot layer, `elda-core` runtime slabs including `app_install/dependency/*`, `app_install/binary_source.rs`, `app_repo/*`, and the new `cache_policy/*` cleanup path, and the largest test modules into smaller units; the workspace is currently green, though three Rust files still sit slightly above the `350` soft-limit ceiling from `CODE_STANDARDS.md` at `389`, `352`, and `352` lines
- the written contract in `SPEC.md` and `pkgitfork.md` now freezes the remaining trust/source behavior around `phase.md` being status-only, trust-rotation metadata shape, explicit operator-confirmed TOFU rotation, ad hoc git version normalization, arch-specific `github_release` authoring, the replacement-grade native build-system floor, the canonical `provider_assets` recipe contract, and the first-class source-only maintained-remote model; the current runtime now also executes source-capable synced remote installs from pinned package-definition repos through remote `packages_url`, while provider-asset reconciliation remains implementation work
- local recipe parsing and validation now preserve the spec-declared declarative metadata families for `sysusers`, `tmpfiles`, `alternatives`, `hooks`, flag tables, and `subpackages` instead of dropping them on parse
- local and ad hoc git installs through the current Cargo-backed source lane
- direct ad hoc git installs now normalize installed package versions from the resolved commit as `0.git.<commit_unix>.<shortsha>` instead of persisting the scaffold placeholder version into installed state
- maintained recipes with either a single source definition or dual source/binary lanes
- explicit lane choice through `elda i`, `elda ig`, `elda ib`, `--prefer-source`, and `--prefer-binary`
- config-backed global, profile, package, and one-shot CLI flag layers through `[flags.*]`, `--use=...`, and the new `fl check` / `fl diff` inspection path, with resolved variant IDs now recorded in installed state and surfaced in install/upgrade plans
- non-default flag variants now fail closed on binary-only targets and otherwise fall back to the source lane automatically, so the current slice does not silently install a mismatched binary for a customized build intent
- closure-aware install planning and execution for the current maintained-recipe and synced-snapshot flow, with dependency packages recorded as non-world `dep` installs instead of being treated as explicit anchors
- default `recommends` install on explicit `elda i` when satisfiable, with weak edges recorded so `rdeps --weak` and `why` reflect the real relationship instead of flattening it into a hard dependency
- versioned direct dependency constraints now check actual package versions instead of treating name matches as sufficient
- `any = { ... }` dependency alternatives and unique virtual `provides` now fail closed on ambiguity instead of silently taking the first resolvable package, versioned dependency requests now honor the explicit-versioned-`provides` rule from the spec instead of letting plain `provides` satisfy them, and synced multi-provider resolution now uses remote priority then same-priority version ordering before failing on true ambiguity
- synced remote indexes through `elda sync`, plus `elda search` and `elda info` against the merged local snapshot
- signed remote index verification for pinned or TOFU remotes, persisted trusted public-key state for the current repo slice, stale verified-snapshot fallback, secure remote payload-signature verification before install, `--offline` sync/refresh use of cached verified snapshots only, fail-closed first-use TOFU policy for JSON/unattended sync while human sync can bootstrap trust explicitly, signed TOFU key rotation through `metadata_url` plus `${metadata_url}.sig`, explicit operator-confirmed rotation through `--accept-rotated-key <remote>`, and `rmt add --priority` / `rmt add --metadata-url` for setting remote precedence and rotation-document location from the CLI instead of hand-editing TOML
- configured cache nodes with explicit priority through `cache add --priority`, cache-first archive resolution for synced binary installs, a content-addressed local payload cache that supports offline reinstall when the payload was already fetched once, and automatic local cache cleanup that retains installed or archived rollback payloads while pruning expired source/archive artifacts and unreferenced payloads once the default threshold is exceeded
- named installs from synced remote metadata, including binary-lane package definitions carried by the synced index, indexed asset metadata, and recorded remote origin in installed state
- source-capable synced remote installs now materialize the real `packages/<pkgname>/` tree from the remote's cloneable `packages_url` at indexed `repo_commit`, so source-only maintained remotes and explicit `ig` installs from synced packages can build against pinned companion files instead of only the indexed `pkg_lua`
- installed provenance now normalizes to canonical persisted `source_kind` values like `local_recipe`, `git`, `repo_binary`, and `interbuild` instead of leaking author-facing fetch kinds such as `url_archive`
- maintained `github_release` recipes now support arch-specific `assets = { <arch> = { ... } }` tables through parse, validation, and binary staging instead of only one top-level asset/checksum pair
- binary vendor recipe generation through `vendor add`, plus `vendor import` and `vendor export`, including GitHub release asset auto-detection for the current OS/arch when the match is clear
- staged payload creation, manifest capture, ownership tracking, `files`, `files owner`, `verify`, and `recover`
- deterministic conffile handling in prefix mode, including `*.eldanew` on first-ownership collision or local-modified upgrade and `*.eldasave` preservation on remove unless `--purge-conffiles` is used
- recorded dependency introspection through `elda why` and `elda rdeps`, plus orphan cleanup through `elda autoremove`
- package policy controls through `elda pin`, `elda unpin`, `elda hold`, and `elda unhold`, with upgrade blocking for pinned or held packages
- archive-backed downgrade through `elda downgrade`, including dry-run planning, latest-older candidate selection from archived prefix state with cached payloads/manifests, reverse-dependency version checks, and policy blocking for pinned packages
- manifest-aware drift inspection through `elda diff <pkg>` and candidate manifest comparison through `elda diff <pkg> --candidate`
- desired-state export and import through `elda state export` and `elda state import`, including remote-document replay, world-anchor reinstallation through the normal install path, and export of the persisted applied profile base instead of falling back to config defaults
- world-backed upgrade execution through `elda u`, including version comparison against synced metadata, policy-aware blocking, conflict checks, same-transaction `replaces`, closure-aware install of newly required hard dependencies, fail-closed targeted-upgrade reverse-dependency coherence checks, `--refresh-weak-deps` / `refresh_weak_deps` support for newly introduced `recommends`, and explicit non-adoption of weak deps unless policy allows them
- profile and daemon commands through `elda pf show`, `elda pf apply`, `elda pf add`, `elda pf rm`, `elda pf set-init`, `elda pf clear-init`, `elda pf set-arch`, `elda pf add-foreign-arch`, `elda pf remove-foreign-arch`, `elda daemon status`, `elda daemon refresh`, and `elda fix-triggers`, with honest current-backend reporting instead of stub output for the current prefix slice
- profile application now installs selected profile anchors as `base`, persists init-provider and foreign-arch policy, removes deselected active profile anchors when safe, and fails closed if the requested set relies on implicit profile anchors
- profile inspection now resolves machine shape from persisted/imported profile state, installed profile anchors, explicit config, and root/host detection instead of inventing a hardcoded `yoka-core` / `dinit` baseline on empty roots
- profile recipes can now declare first-class machine-shape defaults through `profile = { native_arch?, foreign_arches?, init? }`, and conflicting profile-policy declarations now fail closed during profile application instead of silently picking one
- the `pf` namespace now supports real edit-style mutations through `pf add`, `pf rm`, `pf set-arch`, `pf add-foreign-arch`, `pf remove-foreign-arch`, and `pf clear-init`, while `pf show` now exposes declared profile-policy metadata when the active anchors are locally resolvable
- `pf show`, profile dry-runs, profile edit commands, and `fix-triggers` now derive typed pending system-change handlers for init-provider transitions, foreign-arch policy, and unapplied profile-set reconciliation, and they report the strongest required activation class instead of hardcoding an empty handler set
- `rc add --kind profile` now scaffolds a profile-shaped `pkg.lua` instead of forcing profile authors to hand-roll the first recipe from scratch
- human CLI rendering for `ls`, `state show`, `pf show`, `search`, `info`, `cache ls`, `check`, `verify`, `recover`, and `daemon status` that now shows real tables and state blocks instead of reducing everything to counters, with `cache ls` also exposing the active retention policy and current local cache usage
- the root help surface now has a branded custom screen with the Elda ASCII logo, grouped command sections, examples, and palette-aware terminal theming instead of raw Clap command dumps
- archived prefix-state capture plus real `elda rollback` restoration from cached payloads and manifests
- removal safety in prefix mode that blocks deleting required packages unless `--cascade` is used
- explicit live-host system-mode gating through `defaults.allow_system_mode` or `elda -S`, plus frontend privilege-provider auto-detect/re-exec for live host operations
- disposable system-mode installs under `/usr` now compose the next managed root under `var/lib/elda/states/<state-id>/root`, activate the live root from that staged materialization, record backend-aware `system-*` state IDs and `linux-copy` activation backend names, materialize declarative `sysusers` / `tmpfiles` / `alternatives` metadata under the target root, persist internal trigger state under `var/lib/elda/state/system-backend/`, and let `check` / `fix-triggers` report or repair current trigger work for the current backend slice
- disposable system-mode rollback now restores archived package system metadata, re-captures the reactivated archived state under the staged-state tree, and reruns system-trigger reconciliation from cached artifacts, with install/remove/fix-triggers/rollback coverage landing in disposable-root tests
- `elda info` now exposes machine-readable provider-asset visibility for the current slice: declarative `sysusers` / `tmpfiles` / `alternatives` / hook metadata, installed system-backend asset state for `/usr` packages, active provider-family state, and pending provider-specific handler transitions
- build and install fetch paths now honor `--offline` for git and archive sources instead of silently reaching out to the network, and archive sources now reuse the content-addressed local payload cache plus configured cache nodes before falling back to origin
- disposable prefix installs that are safe to test repeatedly without touching the host `/usr`
- real upstream binary-release validation through the `fsel` fixture recipe and synced-index smoke runs

What Elda is not yet:

- a live system `/usr` package manager
- a full dependency resolver and provider-selection engine
- a finished `pkgit` replacement

## 2. Scope

### 2.1 In Scope

- Implementing the full Elda contract from `Elda/SPEC.md`.
- Using phased delivery as dependency order and proof checkpoints, not as product shrinkage.
- Preserving the good `pkgit` operator energy: direct git install flow, simple command surface, and local override/import workflows.
- Replacing `pkgit`'s state, dependency, install, and update model with Elda's DB-backed transaction engine.
- Building the native Elda path first and then finishing CI, interbuild, interepo, and migration until the fork is complete.

### 2.2 Out of Scope

- A line-by-line port of the Nim code.
- Shipping a renamed `pkgit` with the same internals and calling it Elda.
- Keeping `pkgdeps`, `bldit`, flat repo lists, cloned repos, or interactive prompts as Elda's core runtime model.
- Treating heuristic file copying into live prefixes as an acceptable temporary install model.
- Letting macOS, Windows, the Nix store model, or general Portage emulation drive the early architecture.

## 3. Delivery & Risk

**Rollout Strategy:** Elda is built in parallel to the existing `pkgit/` tree. The early bring-up path is crate skeleton -> DB and resolver -> staged payloads -> prefix backend -> Linux system backend -> CI/native publish -> interbuild -> interepo -> migration. We do not call Elda the replacement for `pkgit` until the transaction engine and Linux activation backend are real.

**Reversibility:** Before system-mode activation is ready, Elda runs in fixture repos, disposable prefixes, chroots, and VMs. No live `/usr` claims until manifests, ownership, journals, recovery, and rollback are implemented and tested. Snapshot tooling is integration, not the recovery model.

**Known Tech Debt:**

- Host build mode will likely land before isolated and remote execution are fully implemented, but the build-mode contract is fixed from day one and system-manager cutover waits for the stronger backend story.
- Native CI publishing and foreign-repo adapters come after the core transaction engine, but their interfaces are still locked to the spec now so early code does not grow the wrong seams.
- Some CLI namespaces will appear before every backend is implemented. Unsupported paths must fail clearly and never silently degrade into `pkgit`-style behavior.

**Cutover Gates:**

- Elda is not a real system package manager until Phase 7 is complete.
- Elda is not a full spec-complete fork until Phase 12 is complete.

## 4. Fork Rules

The fork is "no nonsense" only if the boundaries stay explicit.

| Keep from `pkgit` | Remove from core immediately |
| --- | --- |
| direct `i <git-url>` energy | flat repo list as package truth |
| simple CLI feel | cloned checkout as installed-state database |
| local override/import lane | `pkgdeps` as live dependency model |
| easy search/list/files mental model | `bldit` as the first-class build format |
| ad hoc install convenience | heuristic file-copy installation into live prefixes |
| operator-readable output | interactive prompts inside core package operations |
| one explicit tool | shell-string execution as normal control flow |

Relevant local reference points from the upstream code:

- `pkgit/src/pkgit.nim` is the current CLI split and the main example of the simple operator feel worth preserving.
- `pkgit/src/installPkg.nim` shows the direct install flow that Elda should preserve at the UX level while replacing the internals.
- `pkgit/src/getDeps.nim` is exactly the dependency model Elda should import and retire, not keep alive.
- `pkgit/src/buildPkg.nim` is the clearest example of the current heuristic build and live-copy behavior that Elda must replace with staged payloads.
- `pkgit/src/removePkg.nim` and `pkgit/src/updatePkgs.nim` show why cloned repos and prompts cannot remain the state model.

The `pkgit/` directory in this repo is reference material, fixture input, and importer source. It is not the product codebase we will keep extending.

## 5. Architecture & Shape

Source-of-truth order:

1. `Elda/SPEC.md`
2. `phase.md`
3. `pkgitfork.md`
4. `pkgit/` as reference and import material only

Target crate landing order:

- Foundation: `elda-cli`, `elda-core`, `elda-db`
- Metadata and source ingestion: `elda-recipe`, `elda-fetch`, `elda-git`, `elda-repo`
- Execution: `elda-build`, `elda-install`, `elda-unix`, `elda-linux`
- Extension and tooling: `elda-ext`, `elda-types` if needed, `xtask`

Architecture rules:

- `elda-cli` stays thin.
- All package-manager logic must be testable without invoking the binary.
- There is one conceptual Elda across native, interbuild, interepo, and adopted packages.
- Prefix mode is a bring-up and safety lane, not a second product definition.
- There is no hidden runtime fallback to `pkgit`.

## 6. Open Decisions

These are implementation choices, not spec holes.

| ID | Question | Options | Leaning | Blocker? |
| --- | --- | --- | --- | --- |
| D-01 | First Linux activation materialization strategy | staged tree + file switch / symlink tree / overlay-backed live root | staged tree + explicit current-state metadata | Yes |
| D-02 | First isolated build backend implementation | native namespaces / wrapper like bubblewrap / host-only until later | native namespaces with capability detection; host mode only for bring-up | Yes |
| D-03 | First foreign adapter order after native flow | ALPM first / APK first / Portage first | ALPM first, then APK, then Portage | No |
| D-04 | First publish layout for the native index | separate `yoka-ci/index` repo / generated branch or artifact in `yoka-ci/pkgs` | generated branch or artifact first | No |

## 7. Build Order

### Phase 0: Fork Baseline and Workspace Skeleton
- **Scope:** Create the Rust workspace and crate skeleton from `Elda/SPEC.md` section 19, wire the canonical CLI namespaces, and capture `pkgit` fixture inputs for behaviors we are preserving or importing.
- **Hardness:** H1
- **Dependencies:** None
- **Done-when:** `cargo check` passes for the workspace skeleton, `elda --help` exposes the canonical namespaces from the spec, and fixture samples exist for `pkgdeps`, `bldit`, direct git installs, and repo search/list flows from `pkgit/`.

### Phase 1: Core Types, Config, and State Skeleton
- **Scope:** Implement package identity and version parsing, config loading, privilege re-exec scaffolding, SQLite schema bootstrap, world/journal/manifests layout, and the global mutation lock.
- **Hardness:** H2
- **Dependencies:** Phase 0
- **Done-when:** An empty Elda root bootstraps cleanly, schema and version-ordering tests pass, and read-only commands such as `ls`, `state show`, and `check` can return structured empty-state output.

### Phase 2: Recipe Model and Legacy Import
- **Scope:** Implement `pkg.lua` and `build.lua` loading, schema validation, local recipe management commands, and one-way import of `pkgit`-style `bldit` and `pkgdeps` into Elda metadata.
- **Hardness:** H2-H3
- **Dependencies:** Phase 1
- **Done-when:** Local recipes validate through `rc check`, imported `pkgit` packages become normalized Elda recipe directories, and declarative package definitions can be parsed without executing arbitrary build logic.

### Phase 3: Remotes, Indexes, Trust, and Cache
- **Scope:** Implement remote definitions, cache definitions, signed index sync, trust-store behavior, stale/offline snapshot policy, and cache registration and inspection commands.
- **Hardness:** H3
- **Dependencies:** Phase 1
- **Done-when:** A signed fixture remote syncs into a verified snapshot, offline mode uses only cached payloads plus verified snapshots, and cache priority plus retention policy are visible and testable.

### Phase 4: Resolver, Flags, and Planning Engine
- **Scope:** Implement the PubGrub-style resolver, provider choice rules, weak-dependency policy, hold/pin handling, variant identity, and machine-readable transaction planning for install, remove, upgrade, and diff flows.
- **Hardness:** H4
- **Dependencies:** Phase 2, Phase 3
- **Done-when:** `--dry-run --json` emits stable plans, ambiguity and partial-upgrade rules match the spec, and solver tests cover native metadata plus interepo-shaped translated metadata.

### Phase 5: Build, Staging, and Payloads
- **Scope:** Implement fetch and git source handling, build orchestration for `git`, `url_archive`, and `github_release`, shared source/binary lane metadata for one maintained package definition, `vendor add/import/export` on top of the same normalized source model, staged payload assembly, manifests, hashes, split packages, and post-stage object analysis.
- **Hardness:** H4
- **Dependencies:** Phase 2, Phase 4
- **Done-when:** Elda can build a recipe into a `.pkg.tar.zst` payload with manifest and metadata, maintained packages can expose both source and binary acquisition lanes in one recipe, git URLs and vendor archives normalize into the same staging model, and staged outputs never write directly into the live root.

### Phase 6: Prefix Transaction Backend
- **Scope:** Implement ownership tracking, conffile behavior, manifest verification, journaled install/remove/upgrade, `recover`, `rollback`, and the non-`/usr` prefix activation backend.
- **Hardness:** H4
- **Dependencies:** Phase 4, Phase 5
- **Done-when:** End-to-end install/remove/upgrade works in a disposable prefix, crash-recovery tests pass, `files` and `files owner` work from recorded manifests, and unmanaged path collisions fail loudly.

### Phase 7: Linux System Backend and Trigger Engine
- **Scope:** Implement the Linux `/usr` activation backend, archived states, current-state tracking, system triggers, declarative `sysusers`/`tmpfiles`/`alternatives`, and the first typed system-change handlers.
- **Hardness:** H5
- **Dependencies:** Phase 6
- **Done-when:** A VM can use Elda against a real system root with archived states, `rollback` reactivates archived state where supported, and trigger repair plus provider-asset visibility behave as specified.

### Phase 8: Profiles, Machine Shape, and Ops Surface
- **Scope:** Implement profile and meta packages, `pf apply`, `pf show`, desired-state export and import, `autoremove`, `check`, `reverify`, `fix-triggers`, and the daemon surface.
- **Hardness:** H3-H4
- **Dependencies:** Phase 7
- **Done-when:** A fresh root can bootstrap through profile application, desired-state export and import reproduce machine shape on a disposable target, and daemon refresh/status operate against synced snapshots.

### Phase 9: Native CI and Binary Publishing
- **Scope:** Implement the package-definition repo workflow, `ci` namespace behavior, DAG and layer generation, `lock-v1.json.zst`, payload publication, index publication, `forge search/browse`, and the first `qa` entrypoints tied to the published pipeline.
- **Hardness:** H4
- **Dependencies:** Phase 3, Phase 5, Phase 8
- **Done-when:** A merged package definition produces payloads, signatures, SBOM, attestation, and index updates, and native Elda clients default to the published binary lane through `elda i` while `elda ig` and `elda ib` cleanly override that choice.

### Phase 10: Git-Mode Interbuilds
- **Scope:** Implement bounded `nix_flake` and `gentoo_overlay` support in git mode, including fail-closed parsing rules, curated Gentoo shim support, and GPKG binary fast-path behavior.
- **Hardness:** H5
- **Dependencies:** Phase 5, Phase 4
- **Done-when:** Supported flake repos install through `elda i <git-url>` without the `nix` CLI, supported overlay packages install without Portage, and unsupported inputs fail closed with explicit errors.

### Phase 11: Interepo Translation and Coexistence
- **Scope:** Implement foreign-repo adapter plumbing, translated index snapshots, verification confidence levels, coexist/warn/lock modes, `ext ls`, and normal install flow for translated packages.
- **Hardness:** H5
- **Dependencies:** Phase 3, Phase 4, Phase 7
- **Done-when:** At least one foreign repository type syncs into translated metadata and installs through the normal resolver and transaction engine, confidence levels are surfaced in CLI output, and coexistence controls work safely in VM tests.

### Phase 12: Migration, Adoption, and `pkgit` Retirement
- **Scope:** Implement `mg from`, `adopt`, the required v1 migration adapters, provenance preservation, and the final cutover from `pkgit` as the active package-manager model.
- **Hardness:** H5
- **Dependencies:** Phase 11, Phase 8
- **Done-when:** The required adapter set exists for `pacman`, `apt`/`dpkg`, `apk`, `xbps`, and `portage`, adopted packages preserve provenance and pass `check`, and Elda can replace `pkgit` without relying on `pkgit` internals at runtime.

## 8. Status Tracker

| Phase | Status | Last Updated | Notes |
| --- | --- | --- | --- |
| Phase 0: Fork Baseline and Workspace Skeleton | Completed | 2026-04-11 | Rust workspace landed; `cargo check` passes; `elda --help` exposes the canonical namespaces; `fixtures/pkgit/` captures `pkgdeps`, `bldit`, direct git install, and repo search/list reference inputs |
| Phase 1: Core Types, Config, and State Skeleton | Completed | 2026-04-12 | Canonical identity/version parsing and ordering tests landed; config defaults now include privilege auto-detect policy; SQLite bootstrap, world/journal/manifests layout, and mutation lock exist; `ls`, `state show`, and `check` return structured empty-state output against a bootstrapped root |
| Phase 2: Recipe Model and Legacy Import | Completed | 2026-04-14 | `rc add` scaffolds declarative recipes and imports local `pkgit`-style sources; imported `pkgdeps` now become best-effort `depends` entries plus an explicit legacy summary file; `rc check` parses the current declarative Lua subset, validates core spec fields, understands declarative `build = { ... }` metadata, accepts dual-lane `source = { lanes = { ... } }` recipe definitions, and now preserves/validates the declared `sysusers`, `tmpfiles`, `alternatives`, `hooks`, flag-table, and `subpackages` metadata families without executing arbitrary build logic |
| Phase 3: Remotes, Indexes, Trust, and Cache | In Progress | 2026-04-19 | `rmt add`, `cache add`, and `cache ls` persist and read TOML documents under `remotes.d/` and `caches.d/`; cache nodes now accept explicit priority, remotes can now set explicit priority directly through `rmt add --priority`, synced binary installs now try configured caches before origin asset URLs, fetched payloads now populate a content-addressed local archive cache that offline installs can reuse, and automatic local cleanup now enforces the default retention policy while retaining installed-package and archived-rollback payloads; `cache ls` surfaces the active retention thresholds plus current local cache usage; `sync` fetches local or HTTP-backed index documents, verifies detached Ed25519-signed index sidecars for pinned or TOFU remotes, persists trusted public keys plus per-remote verified snapshot state, marks stale remotes instead of dropping them, supports `--offline` refresh against cached verified snapshots only, rejects implicit first-use TOFU enrollment in JSON/unattended sync, accepts signed TOFU key rotation through `metadata_url` plus `${metadata_url}.sig`, requires explicit operator confirmation through `--accept-rotated-key <remote>` before storing rotated TOFU keys, and secure remote binary installs now verify indexed `payload_sig` values against the remote trust set before staging; source-capable synced remotes now also accept `packages_url`, and `search` / `info` continue to query the merged verified snapshot |
| Phase 4: Resolver, Flags, and Planning Engine | In Progress | 2026-04-19 | `i`, `ig`, `ib`, `rm`, `u`, and `autoremove` now emit or execute materially real plans rooted in the current Elda state layout; install requests apply the documented lane-selection policy for both local recipes and synced remote packages; install dry-runs now show the actual dependency closure instead of only the top-level targets; explicit installs now auto-add satisfiable `recommends`; `any = { ... }` alternatives and unique virtual `provides` now fail closed on ambiguity instead of silently picking the first hit; direct versioned dependency constraints now check actual candidate versions, versioned dependency requests now honor explicit versioned `provides` while plain unversioned `provides` no longer satisfy them, and synced multi-provider resolution now uses remote priority then same-priority version ordering before failing on true ambiguity; package-level `conflicts` now block invalid install and upgrade plans; same-transaction `replaces` now work for same-origin installs/upgrades while cross-origin replacement and replacement that would strand hard reverse deps still fail closed; upgrades can now pull in newly required hard dependencies from the same synced snapshot, reject targeted moves that would break installed reverse dependencies, and only add newly introduced weak deps when `--refresh-weak-deps` or config policy enables it; flag layers are now real through config plus `--use=...`, resolved variant IDs now persist in installed state, `fl check` / `fl diff` now expose effective flag state and drift, and non-default variants now fail closed on binary-only targets while automatically falling back to source when a maintained source lane exists, including synced remotes that must fetch package-definition companion files through `packages_url`; full PubGrub-style solving and user-config/provider-policy control beyond current remote priority are still pending |
| Phase 5: Build, Staging, and Payloads | In Progress | 2026-04-19 | `elda-build` now clones `git` sources, auto-detects or honors the first declarative Cargo build path, stages files under canonical `/usr`, emits a `.pkg.tar.zst` payload plus `.manifest`, records payload/manifest hashes, normalizes direct ad hoc git installs to commit-derived versions, supports the first real binary-lane staging path for `url_archive` and `github_release`, now parses/validates/stages arch-specific `github_release` asset tables, consumes indexed remote `asset_url` / `sha256` metadata for synced binary installs, verifies secure-remote payload signatures before staging, and now has working `vendor add` / `vendor import` / `vendor export` recipe generation for local convenience binaries, including GitHub release asset auto-detection for the current OS/arch when the match is unambiguous; source-capable synced remote installs now materialize pinned package-definition trees from `packages_url` plus indexed `repo_commit` before build so source-only maintained remotes and explicit source-lane synced installs can consume `build.lua`, patches, and companion metadata; installed provenance now normalizes author-facing fetch/build kinds into canonical persisted `source_kind` values for local recipes, ad hoc git installs, repo binaries, and interbuild sources; richer build-system coverage and broader binary/source-kind handling are still pending |
| Phase 6: Prefix Transaction Backend | In Progress | 2026-04-14 | `elda i` now performs a real manifest-backed prefix install for local/ad hoc git targets, maintained dual-lane recipes, synced remote package names, and the first direct dependency closures, including `ig` / `ib` lane selection and correct world-vs-dependency install reasons; `elda rm`, `elda autoremove`, `elda files`, and `elda files owner` operate on recorded ownership data in the disposable root, with reverse-dependency protection unless `--cascade` is used; prefix transactions persist journals, block new mutations until recovery, support `elda recover`, support manifest-backed `elda verify` / `elda reverify`, archive committed prefix states so `elda rollback` can restore the previous or a named archived state from cached payloads, implement archive-backed `elda downgrade` with dry-run planning and reverse-dependency version checks, and now implement spec-shaped conffile handling through `*.eldanew` / `*.eldasave` plus `--purge-conffiles`; live-host Linux/system activation work is still pending |
| Phase 7: Linux System Backend and Trigger Engine | In Progress | 2026-04-19 | System-mode installs under `/usr` now compose the next managed root under `var/lib/elda/states/<state-id>/root`, activate the live root from that staged materialization, record `linux-copy` activation backend state plus backend-aware `system-*` state IDs, persist archived system-state metadata, materialize declarative `sysusers` / `tmpfiles` / `alternatives` assets, run an internal trigger engine with persisted trigger state plus `check` / `fix-triggers` repair, expose provider-asset visibility through `elda info`, and keep rollback-aligned staged roots in disposable-root tests; the written contract now also freezes native `provider_assets` metadata and source-only maintained remotes, while live-host file-switch activation, real provider-asset reconciliation, and broader boot/backend integration are still pending |
| Phase 8: Profiles, Machine Shape, and Ops Surface | In Progress | 2026-04-17 | `state export` and `state import` now round-trip the first real desired-state document for the supported prefix slice, including world anchors, remotes, and current profile policy fields; imported profile state is now persisted and reused by `pf show`, applied profile state now drives exported `profile.base`, and empty roots no longer fabricate a distro profile when none is recorded; `pf show`, `pf apply`, `pf add`, `pf rm`, `pf set-init`, `pf clear-init`, `pf set-arch`, `pf add-foreign-arch`, `pf remove-foreign-arch`, `daemon status`, `daemon refresh`, and `fix-triggers` now report or mutate the current backend shape honestly instead of falling through to stubs, with typed pending system-change reporting for init-provider transitions, foreign-arch policy, unapplied profile-set reconciliation, and current pending system-trigger repair; `pf apply` installs selected profile anchors as `base`, consumes declared profile-policy defaults from profile recipes, and persists init-provider/foreign-arch/native-arch policy for the current prefix slice; `rc add --kind profile` now scaffolds first-class profile recipes; live provider-asset reconciliation and broader daemon/system-management work remain pending |
| Phase 9: Native CI and Binary Publishing | Not Started | 2026-04-02 | Depends on stable payload and index contracts |
| Phase 10: Git-Mode Interbuilds | Not Started | 2026-04-02 | Depends on native build/staging engine |
| Phase 11: Interepo Translation and Coexistence | Not Started | 2026-04-02 | Depends on native install and verification path |
| Phase 12: Migration, Adoption, and `pkgit` Retirement | Not Started | 2026-04-02 | Final fork completion gate |

## 9. Changelog

### v0.1.31 - 2026-04-17

- The disposable-root `/usr` backend no longer builds the next system state straight into the live root: it now composes the next managed tree under `var/lib/elda/states/<state-id>/root` and activates from that staged materialization.
- `rollback` now re-captures the reactivated archived system state back into the staged-state tree so the active state pointer and staged root stay aligned after rollback, and journal recovery now cleans created paths plus restores backups for interrupted staged activation work.
- Added disposable-root regressions that prove staged system states contain the composed next root across install/remove/rollback, then re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.30 - 2026-04-17

- `elda info` now exposes machine-readable provider-asset visibility for the current slice, including declarative system-asset metadata from local or synced recipes, installed `/usr` system-backend asset state, active provider families, and pending provider-specific handler transitions.
- Added regressions for local-recipe and installed-system-package `info` visibility, and kept the new repo/runtime code under the file-size ceiling by splitting the info backend into its own `app_repo/info.rs` module.
- Re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.29 - 2026-04-17

- Landed the first disposable-root Phase 7 slice for `/usr` mode: system installs now record backend-aware `system-*` state IDs, persist `linux-copy` activation backend names, archive package system metadata, materialize declarative `sysusers` / `tmpfiles` / `alternatives` assets, and run an internal trigger engine with persisted repair state.
- `check` now surfaces pending system-trigger repair records, `fix-triggers` now repairs current system trigger outputs for the `/usr` backend slice, and disposable-root regressions now cover install/remove/fix-triggers/rollback behavior against a system-mode root.
- Split the new system-backend tests into `crates/elda-core/src/tests/system_backend/*` so the Phase 7 coverage stayed under the repo file-size ceiling, then re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.28 - 2026-04-14

- Completed the remaining declared Phase 2 recipe-model gap by preserving and validating the spec-declared declarative metadata families for `sysusers`, `tmpfiles`, `alternatives`, `hooks`, flag tables, and `subpackages`.
- Upgrade planning now supports same-transaction same-origin `replaces`, rejects targeted upgrades that would strand installed hard reverse dependencies, and honors `--refresh-weak-deps` or `defaults.refresh_weak_deps = true` before adding newly introduced `recommends`.
- Added CLI, core, and recipe regressions for the new upgrade policy and metadata-family coverage, then re-verified the full workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.27 - 2026-04-14

- `rmt add` now accepts `--priority`, so remote precedence for sync and resolver policy can be set from the CLI instead of requiring a manual edit under `remotes.d/`.
- Added CLI and core regressions to prove the parsed operand round-trip and the persisted remote document both keep the chosen priority value.

### v0.1.26 - 2026-04-14

- Synced virtual-provider resolution no longer hard-fails every multi-provider case by default; it now uses remote priority first and same-priority version ordering second, then still fails closed when ambiguity remains.
- Added end-to-end regressions covering both best-priority provider selection and same-priority highest-version provider selection from synced remotes.
- The current provider-policy slice is still intentionally conservative: mixed local-vs-synced provider sets and other unresolved policy cases continue to fail loudly instead of guessing.

### v0.1.25 - 2026-04-14

- Added typed dependency/provide constraint parsing in `elda-types` so versioned dependency semantics are no longer string-sliced ad hoc inside the resolver.
- `rc check` now validates dependency, `provides`, `conflicts`, and `replaces` constraint syntax, including rejecting versioned `provides` that use anything other than `=`.
- The install resolver now enforces versioned direct dependencies against actual candidate versions and supports explicit versioned virtual `provides`, while still refusing to let plain unversioned `provides` satisfy versioned requests.
- Added end-to-end dependency-policy regressions for versioned exact dependencies, explicit versioned virtual providers, rejection of unversioned providers for versioned requests, and exact-name precedence over virtual providers.

### v0.1.24 - 2026-04-14

- Implemented local cache retention and garbage collection against the frozen defaults from `idk.md`: 90-day payload retention, 30-day source/archive retention, and cleanup once usage crosses the smaller of 20 GiB or 10% of the backing filesystem.
- Added cache-entry access metadata so source artifacts, built payloads, manifests, and rollback-restored payloads can refresh their local retention timestamps without relying on filesystem atime behavior.
- Automatic cleanup now retains payloads needed by currently installed packages and archived rollback states, and `elda cache ls` now reports the active cache policy plus current local cache usage.

### v0.1.23 - 2026-04-14

- Tightened first-use TOFU behavior so JSON and unattended sync paths no longer auto-enroll trust for a remote on first contact.
- Human sync can still bootstrap a TOFU remote once, and later noninteractive sync reuses the persisted trust state instead of re-enrolling implicitly.
- Added regressions in both `elda-repo` and `elda-core` for allowed and denied first-use TOFU enrollment, and updated the import/repo fixture coverage to use pinned trust where unattended sync is intended.

### v0.1.22 - 2026-04-14

- Added real cache-node priority handling through `elda cache add --priority` and sorted `cache ls` output.
- Synced binary installs now try configured cache nodes before the indexed origin asset URL, using a content-addressed `<cache base>/<sha256>` lookup contract recorded in `idk.md`.
- Archive downloads now populate and reuse a content-addressed local payload cache under the existing source-cache root, so offline reinstall works after one successful online fetch even when the origin payload disappears.
- Added end-to-end regressions for cache-hit installs and offline reinstall from the local payload cache, and kept the codebase under the Rust file-size ceiling after the cache slice landed.

### v0.1.21 - 2026-04-14

- Persisted accepted remote public keys alongside the verified snapshot state so secure remotes can reuse the same trust material for later payload checks.
- Secure synced binary installs now verify indexed `payload_sig` values against the remote trust set before staging, and they consume indexed `asset_url` / `sha256` metadata when that data is present.
- Installed-state records now preserve the remote origin for synced packages instead of dropping it during activation and rollback recovery.
- Added end-to-end regression coverage for secure remote installs, including a negative case where a signed index points at a payload with no `payload_sig`.

### v0.1.15 - 2026-04-13

- Replaced the old install dry-run skeleton with a closure-aware plan that shows real dependency actions, selected lanes, weak-vs-hard edges, and already-installed packages.
- Explicit installs now auto-install satisfiable `recommends` by default and record them as weak dependency edges so `why` and `rdeps --weak` reflect the actual relationship.
- Implemented fail-closed handling for `any = { ... }` dependency alternatives and unique virtual `provides`, so unattended installs no longer silently pick the first matching provider.
- Added package-level conflict validation to install and upgrade planning.
- Upgrades now carry newly required hard dependencies forward from the same synced snapshot and still do not auto-add newly introduced weak dependencies.
- Added end-to-end tests for recommends behavior, unique/ambiguous provider handling, conflict failures, and closure-aware upgrades.

### v0.1.13 - 2026-04-12

- Implemented the first real desired-state document path through `elda state export` and `elda state import`.
- Export now captures world anchors, installed package intent metadata, current profile policy fields, installation mode, prefix, and configured remote documents in one JSON document.
- Import now writes remote documents back into the target root, refreshes the synced snapshot, and reinstalls imported world anchors through the normal install path instead of bypassing staging or transaction logic.
- Added end-to-end tests and a disposable-root CLI smoke for desired-state export/import.
- Updated `CODEBASE_AUDIT.md`, `phase.md`, and `TIMEAWAY.MD` to reflect the current codebase instead of leaving the audit stale.

### v0.1.14 - 2026-04-12

- Added `defaults.allow_system_mode = false` as the explicit live-host gate for system package-manager behavior and wired `elda -S` as the one-shot frontend override.
- Implemented frontend privilege-provider re-exec for live host operations using the configured `doas` / `sudo` / `run0` provider path, with clear errors when the provider is unavailable or disabled.
- Changed the host-root failure mode from raw permission-denied bootstrap errors to direct operator-facing messages that explain whether Elda is gated out of system mode or missing its configured privilege provider.
- Updated the public config docs and sample config to document the new system-mode gate.

### v0.1.15 - 2026-04-12

- Changed the default privilege frontend policy from a hardcoded `doas` assumption to `provider = "auto"`.
- Documented and implemented provider detection in the order `doas`, `sudo`, `run0`, then `su`, with last-resort `su` support now wired in the CLI frontend.
- Explicit provider settings now fall back to the detected-provider order when the requested binary is unavailable, and the frontend reports that fallback instead of surfacing a raw missing-provider failure.
- Verified the change with workspace fmt/tests/clippy plus a rebuilt host CLI smoke where `elda -S ls` now reaches the real `sudo` path on this machine instead of failing on missing `doas`.

### v0.1.16 - 2026-04-12

- Implemented a real `elda diff` path instead of routing the command through the generic stub handler.
- Plain `elda diff <pkg>` now reports live drift for recorded managed paths by reusing the manifest-backed verification layer.
- `elda diff <pkg> --candidate` now compares the installed manifest against the next resolver-selected candidate manifest and reports added, removed, and modified paths.
- Added end-to-end tests for both live drift detection and candidate-manifest comparison, and updated `CODEBASE_AUDIT.md`, `phase.md`, and `TIMEAWAY.MD` to track the shipped behavior.

### v0.1.17 - 2026-04-12

- Implemented archived prefix-state capture in `elda-install` and wired a real `elda rollback` command through `elda-core`.
- `elda rollback` now restores the previous archived prefix state by default and can also target a named archived state id, rebuilding installed state from cached payloads and manifests instead of re-running the source build.
- Added a real rollback regression over a local package upgrade path, including default-target selection that skips the intermediate remove-only archive emitted during upgrade transactions.
- Verified the rollback slice with workspace fmt/tests/clippy and updated `CODEBASE_AUDIT.md`, `phase.md`, and `TIMEAWAY.MD` to reflect that prefix rollback is now code-backed and test-backed.

### v0.1.18 - 2026-04-12

- Reworked the human CLI renderer so high-value inspection commands stop collapsing into summary-plus-count output and instead show the actual state Elda already knows.
- `ls`, `state show`, `pf show`, `search`, `info`, `cache ls`, `check`, `verify`, `recover`, `sync`, and `daemon status` now render operator-readable tables or aligned state blocks in human mode.
- `vendor add owner/repo@tag --binary <name>` now auto-selects the current platform's GitHub release asset when there is one clear payload match, ignores checksum/signature sidecars, and still fails closed with `--asset` when the match is ambiguous.
- Verified the slice with workspace fmt/tests/clippy plus a disposable-root smoke that generated and installed `fsel-bin` from `Mjoyufull/fsel@latest` without passing `--asset`, then checked the new `ls`, `pf show`, `info`, and `state show` human output.

### v0.1.19 - 2026-04-13

- Removed the hardcoded fallback profile shape from default config loading so empty roots no longer pretend they are `yoka-core` with `dinit` and `i386` enabled.
- Added real profile-state resolution that prefers persisted/imported machine state, then explicit config, then installed package metadata and root/host detection for native arch and init family.
- `state export` now emits the resolved profile state instead of raw config defaults, and `state import` now persists imported profile state so `pf show` reflects the imported machine shape even before `pf apply` exists.
- Verified the slice with workspace fmt/tests/clippy plus a disposable-root smoke where `pf show` first reported no active profile anchors on an empty root, then reported imported `import-base` / `import-desktop` anchors and `dinit` after `state import`.

### v0.1.20 - 2026-04-13

- Replaced the raw top-level Clap help dump with a branded custom root help screen that centers the Elda ASCII logo, groups commands by operator workflow, and includes examples.
- Wired the help theming to the repo palette and added palette-aware Clap styles so generated subcommand help also uses the same visual language in a real terminal.
- Added real `about` and argument help text across the CLI surface so `elda <command> --help` stops showing empty command lists and unlabeled operands.
- Verified the slice with workspace fmt/tests/clippy plus live binary smokes of `elda help` and `elda search --help`.

### v0.1.12 - 2026-04-12

- Extended the installed-state schema so prefix installs now persist dependency edges plus package policy state for pinned and held packages.
- Implemented the first real direct dependency-install path for maintained recipes and synced snapshot packages, with dependency installs recorded as `dep` instead of polluting the world file.
- Wired `elda why` and `elda rdeps` to the recorded dependency graph and made `elda autoremove` use that state for real orphan cleanup in prefix mode.
- Implemented `elda pin`, `elda unpin`, `elda hold`, and `elda unhold`, and taught `elda u` to skip held or version-pinned packages instead of blindly rebuilding them.
- Hardened prefix removal so required packages are blocked unless `--cascade` is explicitly requested.
- Verified the slice with `cargo fmt --all`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, plus new end-to-end tests for dependency installs, `why` / `rdeps`, orphan cleanup, and pin/hold-aware upgrade behavior.

### v0.1.11 - 2026-04-12

- Replaced the old internal `phase-*` command-report labels and summaries in runtime output with semantic `area` values and direct operator-facing text.
- Implemented the first real repo snapshot path in `elda-repo`: remote listing, snapshot persistence, local and HTTP index fetch, package-record parsing, package search, package info lookup, and named package resolution from synced metadata.
- Wired `elda sync`, `elda search`, and `elda info` through the real core path instead of the CLI stub path.
- Extended installs so named targets can resolve from the synced snapshot when they are not local recipes, filesystem paths, or raw git targets.
- Implemented the first real `elda u` execution path for prefix mode by comparing installed versions to current candidates, rebuilding newer payloads, and reinstalling them over the recorded world set.
- Added fixture coverage for a real upstream binary package definition with `fixtures/recipes/fsel/pkg.lua`.
- Verified the slice with `cargo fmt --all`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, a direct maintained-recipe install of `fsel` from the real GitHub release asset, and a synced-index `rmt add` + `sync` + `search` + `info` + `i fsel` smoke run in a disposable root.

### v0.1.10 - 2026-04-11

- Implemented the first real vendor binary workflow: `vendor add` now resolves local/raw binary sources and GitHub release specs into pinned local recipes, while `vendor import` and `vendor export` round-trip manifest and lock data.
- Wired the vendor commands through the real CLI/core path instead of leaving them as stub namespaces.
- Verified the slice with workspace fmt/tests plus end-to-end disposable-root tests that add a local vendor recipe, install it, and export/import the lock metadata.
- Added a maintained status block near the top of `phase.md` so the file now states what Elda can actually do at the current checkpoint.

### v0.1.9 - 2026-04-11

- Extended the recipe model so `pkg.lua` can represent both the legacy single `source.kind` form and the new multi-lane `source = { default_lane = ..., lanes = { ... } }` form.
- Implemented `elda ig`, `elda ib`, and `elda i --prefer-source/--prefer-binary` in the real CLI and install path, with default lane selection wired to `defaults.install_preference`.
- Taught the current build slice to stage the first binary-lane payloads from `url_archive` and `github_release` sources, including checksum verification and executable extraction into the staged prefix tree.
- Verified the slice with `cargo fmt --all`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`, including disposable-root tests that prove default binary-lane installs, forced source-lane installs, and rejection of `ib` on raw git targets.

### v0.1.8 - 2026-04-11

- Locked the maintained-package contract for upstreams that expose both a source-install path and a release-binary path: one `pkg.lua` may now declare both acquisition lanes instead of forcing paired package names.
- Added the operator contract for lane selection across `elda i`, `elda ig`, `elda ib`, `--prefer-source`, `--prefer-binary`, and the default config preference of `install_preference = "binary"`.
- Clarified that `vendor add/import/export` remain the local convenience/import path for one-off or unsupported binaries, not the normal maintained-package answer to first-class upstream release assets.

### v0.1.7 - 2026-04-11

- Extended Phase 6 with transaction journals for the prefix backend under `/var/lib/elda/db/journal/`.
- Added journal-aware install/remove behavior that blocks new mutations when recovery is required instead of mutating through an incomplete transaction.
- Implemented `elda recover` for the current prefix backend so incomplete prepared/files-applied transactions can be rolled back and committed transactions can be finalized cleanly.
- Implemented manifest-backed `elda verify` and `elda reverify` for installed prefix packages, including missing-file, content-mismatch, metadata-drift, and path-collision reporting.
- Verified the slice with workspace fmt/tests/clippy plus disposable-root CLI runs of `verify` after deliberate file drift and `recover` against a synthesized pending install journal.

### v0.1.6 - 2026-04-11

- Started Phase 5 by adding the first real `elda-build` path for `git` sources with canonical staging, manifest generation, and `.pkg.tar.zst` payload emission into the Elda cache namespace.
- Extended Phase 2 so `pkg.lua` parsing and validation understand the first declarative `build = { system = "cargo", ... }` table used by the execution slice.
- Started Phase 6 by adding manifest-backed prefix installs and removals in `elda-install`, plus file ownership persistence in `elda-db`.
- Wired `elda i` to build and install local or ad hoc git Cargo targets into a disposable prefix root instead of always returning a stub plan when not using `--dry-run`.
- Wired `elda rm`, `elda files <pkg>`, and `elda files owner <path>` to the real installed-state database and recorded manifests.
- Verified the slice with `cargo fmt --all`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and disposable-root CLI runs of `i`, `files`, `files owner`, and `rm` against a locally generated git Cargo repository.

### v0.1.5 - 2026-04-11

- Started Phase 3 by adding the first remote and cache document layer in `elda-repo`.
- Implemented `elda rmt add`, `elda cache add`, and `elda cache ls` with persisted TOML documents under `/etc/elda/remotes.d/` and `/etc/elda/caches.d/`.
- Started Phase 4 by wiring structured dry-run plan skeletons for `elda i`, `elda rm`, `elda u`, and `elda autoremove`.
- Verified the repo/cache slice with workspace tests/clippy plus disposable-root runs that created and listed remote/cache documents on disk.
- Verified the planner slice with workspace tests/clippy plus disposable-root `--dry-run --json` runs for install, upgrade, and autoremove flows.

### v0.1.4 - 2026-04-11

- Extended Phase 2 from directory-shape checks into real declarative `pkg.lua` parsing and validation.
- Added a parser for the current declarative Lua subset used by `pkg.lua`, including nested tables, arrays, booleans, strings, integers, and `depends = { { any = { ... } } }` entries.
- Added spec-driven validation for package identity fields, canonical architecture labels, package kind, source-kind required fields, and conffile path shape.
- Implemented `elda rc add` for new-recipe scaffolding, direct git-target scaffolding, and local `pkgit`-style source import with legacy `pkgdeps` / `bldit` preservation.
- Fixed local-path imports to preserve a real `file://` source when no git remote exists.
- Added best-effort `pkgdeps` normalization into generated `depends` entries and a persisted `legacy/pkgit-import.json` audit summary for imported `pkgit` sources.
- Verified the new recipe flow with workspace tests/clippy plus disposable-root CLI runs of `rc add` and `rc check` for both scaffolded and imported recipes.

### v0.1.3 - 2026-04-11

- Started Phase 2 by adding the first `elda-recipe` validation path for local recipe directories under `/etc/elda/recipes/<pkgname>/`.
- Wired `elda rc check [pkg]` into the real state/bootstrap path so recipe validation runs against the bootstrapped Elda root.
- Added structured recipe diagnostics for missing `pkg.lua`, invalid `build.lua`, and invalid `patches` path shapes.
- Verified `rc check` against both an empty disposable recipe root and a deliberately broken recipe directory.

### v0.1.2 - 2026-04-11

- Completed Phase 1 by adding package identity and version parsing with spec-matching comparison coverage.
- Added config loading defaults, a privilege-provider scaffold, and an internal root override seam for disposable-root bring-up and tests.
- Added the first SQLite state layer with schema bootstrap, state-layout directory creation, world/current-state files, and a global mutation lock file.
- Wired `elda ls`, `elda state show`, and `elda check` to bootstrap an empty root and return structured state output instead of generic stubs.
- Verified the Phase 1 baseline with `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and disposable-root runs of `ls`, `state show`, and `check`.

### v0.1.1 - 2026-04-11

- Completed Phase 0 by landing the Rust workspace, crate skeleton, and `xtask` placeholder under `Elda/`.
- Added a thin `elda` CLI binary that exposes the canonical root commands and namespace tree from `Elda/SPEC.md`.
- Added shared command/report types in `elda-types` and a minimal execution seam in `elda-core` so the CLI wiring is testable without burying logic in `main.rs`.
- Captured Phase 0 `pkgit` reference fixtures for legacy `pkgdeps`, `bldit`, direct git install targets, and flat repo-list flows.
- Verified the baseline with `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace`, and `cargo run -p elda-cli -- --help`.

### v0.1.0 - 2026-04-02

- Created `phase.md` as the implementation-order companion to `Elda/SPEC.md`.
- Locked in the hard-fork rule that phases sequence the full product and do not redefine Elda downward.
- Added explicit fork boundaries for what Elda keeps from `pkgit` and what it removes immediately.
- Defined the default phase order from workspace bring-up through migration and `pkgit` retirement.
