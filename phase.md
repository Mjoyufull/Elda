# Elda - Phase Plan

**Version:** 0.1.49-Sumomo
**Date:** 2026-05-24
**Status:** Active
**Spec:** `SPEC.md`

## 1. Overview

This file defines the implementation order for Elda. It is not a second spec and it is not a scope-cut document. `SPEC.md` remains the contract. `phase.md` exists to say what lands in what order, what each phase has to prove, and where the hard fork from `pkgit` is handled directly.

Elda is a hard fork of `pkgit` in product direction and operator feel, not in code lineage. We keep the useful parts of `pkgit`'s UX and we replace the internals that stop it from being a real system package manager. That means Rust, SQLite, explicit state, explicit transactions, explicit manifests, explicit verification, and explicit rollback.

The phases here are implementation order only. They do not redefine the product downward. Elda is still the full system described in `Elda/SPEC.md`: native package definitions, binary-first delivery, transactional install state, CI publication, interbuild support, interepo translation, and migration/adoption.

### 1.1 Process Status Percentages

Implementation-process percentages for the current market/UX continuation track:

- **Pre-Phase-11+ focus:** **~97%** for the named native CLI/UX slice (runtime-backed surfaces plus listed post-prephase polish). May 2026 operator-frame pass closed `::` frames, build passthrough, review gate dedup, bulk snapshot parity, and semantic color; remaining gaps: interbuild skip-when-current, upgrade/rm unified transaction frames, MiB preflight polish. Phase-11 interepo/coexistence remains separate.
- **Post-prephase polish (same milestone, not Phase 11):** **~98%** for ledger-listed items; dedicated §13.6 nested inner-build frame deferred in favor of live stdio passthrough (see item 79 in `prephase11+.md`).
- **Phase-11+ focus:** 15%. This counts interepo binary translation, live coexistence/takeover, foreign file DBs, and migration surfaces that require Phase 11+ runtime semantics.
- **Phase 13 focus:** 0%. Host-state materialization and atomic activation (merged tree, ephemeral apply, `/usr` exchange, rename-based state archives). Independent of Phase 11 interepo.
- This slice is treated as Phase 10-adjacent because it improves dynamic interemote, targeted sync, recipe inspection, file/config/trigger inspection, sync delta/failure/cleanup, and remote trust inspection surfaces rather than implementing full Phase 11 interepo consumption.
- Latest delta (2026-05-24): install, bulk snapshot, interbuild, and metadata review frames now use compact `key:: value` rows with plain keys and selective value emphasis (`render_style.rs`); `elda a`/`elda add` skip dispatch confirm so bulk snapshot review owns the import gate; metadata/interbuild/install review gates use a single frame-footer `[Y/n/e]` with no duplicate prompt after pager; live cargo/cmake/meson/make/ctest builds inherit child stdio when streaming (one Elda header line, native tool ANSI/progress like paru); session logs resolve under the invoking operator home through sudo (`ELDA_OPERATOR_HOME`); snapshot-imported interbuild recipes reuse local template trees instead of re-cloning; mutating human commands restore the terminal and exit 130 on Ctrl+C; live progress uses green success checkmarks and orange build-step labels. Prior delta (2026-05-18): `elda review ls`, `elda review info`, `elda review forget`, and `elda review diff` now expose recorded source-definition review stamps; interbuild review opens generated metadata in `ELDA_PAGER`/`PAGER`/`less` before the `[Y/n/e]` gate when content changed; install dry-run preflight now also reports review-memory status, pending configuration files, shared-path policy, and privilege posture; global `--no-stream` now disables live progress streaming for human and JSON runs, `[display].tree_chars` now forces Unicode, ASCII, or automatic tree-frame detection, and privilege re-exec now prints a framed provider/policy handoff before sudo/doas/run0/su; `elda sync <remote...>` now targets named remotes; `rc show`, `rc diff`, and `rc publish-ready` expose recipe source metadata, local-vs-synced drift, and publish-readiness blockers; `ls` has operator filters; `files search`, `config pending`, `config diff`, `config apply`, `config keep`, and `trigger ls/info` now expose installed path search, conffile queue state/resolution, and system-trigger state in runtime output; generated-metadata and interbuild review gates now persist content-addressed review stamps so unchanged recipe content does not re-prompt; generated metadata acceptance now stops at a second install confirmation frame before build/stage/activation; install dry-runs include a preflight block with known managed-byte and filesystem-space data; `elda doctor` now reports bootstrap paths, remotes, backend health, advisories, and release-readiness flags; install/upgrade human plans name replacement targets; Cargo source builds resolve case-mismatched binary targets from Cargo metadata; interbuild and bulk snapshot review frames now expose parser/source/snapshot context more explicitly; `[git].allowed_protocols` now fails closed on disallowed git transports across source, interbuild, remote recipe, preview, and interemote sync paths; `elda git releases --tag <ref>` now filters release inspection to a chosen tag; generated git-release binary recipes now preserve metadata from detected interbuild/native source definitions and emit source+binary lanes when a checksum-backed release asset is available; metadata import paths now preserve existing `pkg.lua` / `build.lua` / patch metadata unless `--replace` is explicit; the public docs/examples pass now makes `/etc/elda` config shape, dynamic interemote remotes, native forge hosting, the quick README, USAGE, fixtures, and the `man/elda.1` operator page line up with that runtime surface.

### 1.2 Current Status

Elda is currently a real disposable-root CLI with a working staged build/install path, a first real `/usr` system-backend slice, solver-backed install/upgrade planning, real profile authoring/editing flow, a first honest local CI/publish slice, and a materially more useful operator surface, not just a command skeleton.

What works right now:

- the `elda-core` runtime has been refactored out of the old giant dispatcher/test-file shape into focused modules (`app_install`, rendering, progress, tests split by area), so the codebase now tracks `CODE_STANDARDS.md` materially better while preserving current behavior
- the current standards pass also split the old `elda-install`, `elda-db` store, `elda-cli` CLI/help and privilege path, `elda-types` package/version parsing, `elda-recipe` parser/vendor/check/import flow, `elda-repo` snapshot layer, `elda-core` runtime slabs including `app_install/dependency/*`, `app_install/binary_source.rs`, `app_repo/*`, the new `cache_policy/*` cleanup path, the new `app_ci/*` runtime, and the expanded `app_profile/*` selection flow into smaller units; the workspace is currently green, and the new profile/CI files for this slice are back under the `350` soft-limit ceiling from `CODE_STANDARDS.md`, though 12 older Rust files elsewhere in the repo still exceed that soft limit
- the written contract in `SPEC.md` now freezes the remaining trust/source behavior around `phase.md` being status-only, trust-rotation metadata shape, explicit operator-confirmed TOFU rotation, ad hoc git version normalization and moving-ref upgrade policy, channel-based delayed-release tracking such as `stable-7d` / `stable-30d`, arch-specific `github_release` authoring, the replacement-grade native build-system floor, the canonical `provider_assets` recipe contract, the root-level `elda a` / `elda add` metadata-first link path, generated metadata confidence tracking, one-time `elda add` native recipe-repo snapshot imports through the `[Y/n/e]` staged edit loop, the first-class source-only maintained-remote model, and the cache-server plus companion-populate workflow for content-addressed cache seeding, maintained-remote mirroring, and interepo payload promotion; the current runtime now also executes source-capable synced remote installs from pinned package-definition repos through remote `packages_url`, enforces per-remote selected channels during sync and offline snapshot reuse, reconciles declared provider assets on the disposable-root `/usr` backend, and persists the current backend's applied init-provider state separately from desired profile policy
- local recipe parsing and validation now preserve the spec-declared declarative metadata families for `sysusers`, `tmpfiles`, `alternatives`, `hooks`, `provider_assets`, flag tables, and `subpackages` instead of dropping them on parse
- local and ad hoc git installs through the current Cargo-backed source lane
- direct ad hoc git installs now normalize installed package versions from the resolved commit as `0.git.<commit_unix>.<shortsha>` instead of persisting the scaffold placeholder version into installed state
- the first bounded Phase 10 interbuild slice is now real for source-lane installs: `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template` recipes fetch git sources, parse their source metadata without invoking foreign package-manager CLIs, fail closed outside the supported static/simple subset, and then hand off to the normal Elda build/stage/install path
- the install/reporting surface is now operator-dense for the current install path: dry-run and success reports show target, resolution, provenance tier, plan, risk, progress, artifacts, result, parser details for interbuild sources, and session-log paths when present; runtime command failures now emit structured `blocked` reports with kind, command, context, next action, and stable exit status instead of only raw frontend `Error:` text
- the human render surface now drops the JSON-fallback dump for unstyled reports and renders `elda ls` plus `elda rc ls` as Nix-profile-list-style per-entry blocks (Name / Version / Provenance / Source ref / Repo commit / Manifest / Payload / Pinned / Hold for installed packages, Name / Version / Provenance / Description / Upstream / Licenses for recipes), so default human output stays operator-readable without leaking machine JSON; `--json` (or `OutputMode::Machine`) still emits the full structured report, and `list` is now a clap alias for `ls` at the root and under `rc`, `cache`, and `ext` so muscle-memory `elda list` / `elda rc list` no longer reroute to search or fail with an unrecognized subcommand
- the first Phase 12 adoption/migration slice is now real for disposable roots: `elda adopt --from <pm> <pkg>` and `elda mg from <pm>` read foreign package-manager databases for `pacman`, `apt`/`dpkg`, `apk`, `xbps`, and `portage`, normalize installed identity/version/file/dependency metadata into Elda's installed-state DB as `source_kind = adopted`, reject already-owned package identities and managed-path collisions before writing state, and render operator-facing migration/adoption frames in human mode without modifying live files
- `elda ext ls` now reads explicit TOML extension registrations from `/etc/elda/extensions.d`, reports name/kind/version/enabled state/binary/capabilities/config path in JSON, renders a human extension frame, and obeys the `capabilities.extension_runtime` gate instead of falling through the generic stub handler.
- the cross-command live progression contract has now moved out of docs and into runtime for the install lane: the shared `ProgressSink` event protocol (`frame_start` / `step_started` / `step_progress` / `step_done` / `step_skipped` / `step_blocked` / `frame_end`), the per-output-mode behavior matrix (live tree on TTY, line-per-event on non-TTY pipes/CI, newline-delimited JSON event stream for `--json`, single final document with `--no-stream`), the "no double rendering" rule, live-build output passthrough for source-lane builds (cargo/cmake/meson/make/ctest inherit child stdio when streaming), and the review-gate-as-frame composition with the `e`/edit recheck loop; the runtime now ships `ProgressSink` live-progress plumbing (TTY tree, plain stream, JSON event stream), shared human `Frame` rendering with Unicode/ASCII tree connectors and `key:: value` rows (`render_style.rs`), and install-step emission (`acquire-source`, `fetch-source`/`fetch-binary`, `verify-binary-source`, `build-source`, `stage-payload`, `analyze-staged-objects`, `activate`, `record-installed-state`); static post-action human renderers for install plans, success, metadata add, search, recipe catalog, interbuild review, and failures share the same frame primitive; install-success output omits duplicate progress blocks when live streaming already ran; the `[Y/n/e]` review gates for generated metadata and interbuild sources now run a real recipe recheck after the editor closes, render a tree-style block listing each issue with severity glyphs, and re-prompt instead of silently dropping operator edits into the build path; frame footers own the sole stdin prompt (no duplicate `Proceed?` line after pager); the `Generated metadata for X is ready` plain prompt has been replaced by a `Metadata Add` frame that names the strategy provenance, output path, and missing required fields per spec §6.1.0; runtime gaps not yet wired (dedicated §13.6 nested inner-build frame, interbuild skip when review memory is `current`, `step_progress` byte/file counters during fetch, JSON event stream shaping for the rest of the matrix in §14, and the framed renderers for `info` / `verify` / `recover` / `pin-unpin` / `hold-unhold` / `vendor *` / `forge *` / `cache *`) remain captured in §12.2
- **`appimage` binary lane (Type 2):** `elda-recipe` validates `source.kind = "appimage"` (direct `url` or forge release fields); `elda-build` stages the verified blob under `usr/lib/elda/appimages/<pkg>/<epoch:ver-rel>/payload/`, symlinks `usr/bin/<binary>` to it, and `elda-appimage` integrates `.desktop`/icons/`usr/share/metainfo` by reading SquashFS only unless the recipe sets `integration = "none"`; `elda appimage inspect` exposes the same read-only view for operators; binary xz stacks align on `liblzma` with the SquashFS reader—see `SPEC.md` §5.2 / §16, `eldastudyappimages.md`
- the current Phase 10 **bounded interbuild slice** is treated as **complete** for the items in the following paragraph (ledger wording—not a formal external audit); remaining parser/build gaps stay tracked in this file and `SPEC.md`: bounded `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template` git-mode source installs, extracted parser metadata, root-level `elda a` / `elda add` metadata-only link handling, Bulk Metadata Snapshot Imports for foreign repositories (Void, Gentoo, Elda) with interactive staging and review, config-ordered local source strategy detection, opt-in priority-ordered source-option reporting through `metadata.link_option_mode = "list-options"`, explicit `--source-option` / `--strategy` source selection for add/install metadata generation, explicit ad hoc git add/install/update ref selectors through `--to-branch`, `--to-tag`, and `--to-rev`, explicit pinned git-ref keep-installed dry-run reporting, config-backed git tag policy defaults, generated recipe metadata filling for AUR/XBPS fields, bounded phase-command extraction for Gentoo/AUR/XBPS reports, AUR/XBPS source-checksum count validation, AUR arch-specific source/checksum validation and `arch_sources` reporting, bounded AUR VCS-source and `pkgver()` reporting, quote-aware AUR/XBPS word parsing for dependency/license-like report fields, exact-key metadata assignment parsing, generated-metadata confidence reporting, fail-closed validation, operator-dense report data, provider-aware `release_asset` binary-lane support for GitHub/GitLab/Gitea/Forgejo/SourceHut/direct manifests, interactive list-options selection, source-ref downgrade for ad hoc git packages, read-only `elda git tags` / `elda versions` inspection, read-only GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest release asset classification, optional `--with-releases` tag-to-release joins, ad hoc release option discovery and checksum-backed pinned metadata conversion for raw-link metadata reports, signature sidecar parsing/materialization plus safe signature-field validation, self-hosted forge host preservation for provider-neutral release assets, broader release-asset checksum sidecar matching that now accepts uppercase `SHA256SUMS`, per-asset `<name>.sums` / `<name>.sha256.txt` variants, and case-insensitive `SHA256SUMS` body lookups so auto-binary detection lands on more upstream conventions without requiring exact filename casing, ad hoc Codeberg release URLs now classify as `forgejo` provenance for honest reporting while existing `gitea`-tagged Codeberg recipes still resolve through the same default-host fallback, `elda git releases --tag <ref>` filters release asset inspection to one tag, `[git].allowed_protocols` fails closed on disallowed transports across source/interbuild/remote-recipe/interemote git clones, generated release-asset recipes can now keep Nix/AUR/XBPS/Gentoo metadata while adding a binary lane instead of overwriting the recipe with blank binary metadata, and the interactive interbuild review gate now shows parser-specific metadata/dependency-family context before build execution; broader Nix evaluation, richer ebuild phase translation, arbitrary generic release scraping, range-fetch / ELF-based release-asset content verification (tracked in `phase10.md §6`), full PKGBUILD/XBPS shell semantics, and Phase 11 binary consumption remain outside this landed slice
- maintained recipes with either a single source definition or dual source/binary lanes
- explicit lane choice through `elda i`, `elda ig`, `elda ib`, `--prefer-source`, and `--prefer-binary`
- config-backed global, profile, package, and one-shot CLI flag layers through `[flags.*]`, `--use=...`, and the now-extended `fl check` / `fl diff` inspection path, with resolved variant IDs recorded in installed state and surfaced in install/upgrade plans; `fl check`/`fl diff` accept `--use=+a,-b` previews and now also surface per-flag descriptions, active per-package layer sources (per-name and per-atom), and cardinality group status; `[flags.package."<atom>"]` entries now apply only when the resolved candidate satisfies the constraint, so version-scoped overrides no longer rebuild every release of the matching package
- recipes (`pkg.lua`) now declare richer flag metadata through `flags_descriptions`, `flags_required_one_of`, `flags_required_at_most_one`, and `flags_required_any_of`; cardinality violations fail closed at resolution time with a structured operator error pointing at the offending group/members
- dependency families (`depends`, `makedepends`, `checkdepends`, `recommends`, `suggests`, `supplements`, `enhances`) now accept conditional entries through `{ name = "constraint", when = "+flag,-other" }` (and the equivalent `any = { ... }` form); the solver filters those entries through the resolved effective flag set before expansion, so unmatched conditional deps never consume a choice slot or appear in plan output
- `elda u --rebuild-variant-drift` pre-fills the upgrade target list with every installed package whose resolved `variant_id` no longer matches the recorded one, so operators can rebuild flag-driven drift in a single command instead of hand-naming each package
- non-default flag variants still fail closed on binary-only targets and otherwise fall back to the source lane automatically, so the current slice does not silently install a mismatched binary for a customized build intent
- solver-backed install and upgrade planning for the current maintained-recipe and synced-snapshot flow, with dependency packages recorded as non-world `dep` installs instead of being treated as explicit anchors
- default `recommends` install on explicit `elda i` when satisfiable, with weak edges recorded so `rdeps --weak` and `why` reflect the real relationship instead of flattening it into a hard dependency
- versioned direct dependency constraints now check actual package versions instead of treating name matches as sufficient
- the current PubGrub-style solver now resolves exact dependencies, `any = { ... }` alternatives, versioned `provides`, and multi-target closure conflicts coherently across the whole requested plan, while ambiguous virtual providers still fail closed instead of guessing
- config-backed provider policy is now real through `[resolver.provider_preferences]`, so operators can override remote-priority provider choice explicitly without resorting to ad hoc local metadata edits
- synced remote indexes through `elda sync`, plus `elda search` and `elda info` against the merged local snapshot
- signed remote index verification for pinned or TOFU remotes, persisted trusted public-key state for the current repo slice, stale verified-snapshot fallback, secure remote payload-signature verification before install, `--offline` sync/refresh use of cached verified snapshots only, fail-closed first-use TOFU policy for JSON/unattended sync while human sync can bootstrap trust explicitly, signed TOFU key rotation through `metadata_url` plus `${metadata_url}.sig`, explicit operator-confirmed rotation through `--accept-rotated-key <remote>`, and `rmt add --priority` / `rmt add --metadata-url` for setting remote precedence and rotation-document location from the CLI instead of hand-editing TOML
- configured cache nodes with explicit priority through `cache add --priority`, cache-first archive resolution for synced binary installs, a content-addressed local payload cache that supports offline reinstall when the payload was already fetched once, and automatic local cache cleanup that retains installed or archived rollback payloads while pruning expired source/archive artifacts and unreferenced payloads once the default threshold is exceeded
- remotes now persist a selected `channel` with `rmt add --channel`, default to `stable`, filter synced package sets to that lane during `sync`, and fail closed if offline or stale snapshot reuse would cross channel boundaries
- the first remote-health/interemote inspection slice is now real: `rmt trust <name>` reports configured and persisted trust state, payload verification readiness, selected key, and snapshot verification timestamps; `rmt info <name>` reports the configured remote, index-vs-interemote kind, synced snapshot state, indexed package names, and installed packages tied to that remote, while `rmt preview <name>` temporarily clones dynamic Gentoo overlay / XBPS `srcpkgs` interemotes and reports commit, parser/source kind, discovered/included/excluded counts, matched excludes, metadata fields, and a package sample before any sync mutation; `elda sync` now embeds those interemote diagnostics plus bounded per-package parser issue rows, per-remote package add/remove deltas, and all-failed index-vs-interemote summaries in the final structured report and human `Sync` frame after package records are generated; targeted `elda sync <remote...>` now refreshes only named remotes and rejects unknown or disabled targets before mutation
- remote management now also includes trust-aware `rmt ls`, `rmt trust`, `rmt enable`, `rmt disable`, and `rmt set-priority`, and command-specific `--help` exits as help instead of being converted into a blocked failure report
- system activation now treats pre-existing unmanaged terminfo entries under `/usr/share/terminfo/**` and `/usr/lib/terminfo/**` as shared database files instead of unmanaged collisions, reusing them without recording Elda ownership
- named installs from synced remote metadata, including binary-lane package definitions carried by the synced index, indexed asset metadata, and recorded remote origin in installed state
- source-capable synced remote installs now materialize the real `packages/<pkgname>/` tree from the remote's cloneable `packages_url` at indexed `repo_commit`, so source-only maintained remotes and explicit `ig` installs from synced packages can build against pinned companion files instead of only the indexed `pkg_lua`
- the companion `elda-populate` tool now implements `cache push-local --installed` and `cache mirror-remote [--channel]`, verifies payload digests before mirroring, emits optional cache-seed manifests, and writes directly to local or `file://` caches for the current cache-seeding slice
- installed provenance now normalizes to canonical persisted `source_kind` values like `local_recipe`, `git`, `repo_binary`, and `interbuild` instead of leaking author-facing fetch kinds such as `url_archive`
- maintained `github_release` recipes now support arch-specific `assets = { <arch> = { ... } }` tables through parse, validation, and binary staging instead of only one top-level asset/checksum pair
- binary vendor recipe generation through `vendor add`, plus `vendor import` and `vendor export`, including GitHub release asset auto-detection for the current OS/arch when the match is clear
- staged payload creation, manifest capture, post-stage ELF shared-library analysis for built payload metadata, ownership tracking, `files`, `files owner`, `verify`, and `recover`
- deterministic conffile handling in prefix mode, including `*.eldanew` on first-ownership collision or local-modified upgrade and `*.eldasave` preservation on remove unless `--purge-conffiles` is used
- recorded dependency introspection through `elda why` and `elda rdeps`, plus orphan cleanup through `elda autoremove`
- package policy controls through `elda pin`, `elda unpin`, `elda hold`, and `elda unhold`, with upgrade blocking for pinned or held packages
- archive-backed downgrade through `elda downgrade`, including dry-run planning, latest-older candidate selection from archived prefix state with cached payloads/manifests, reverse-dependency version checks, and policy blocking for pinned packages
- manifest-aware drift inspection through `elda diff <pkg>` and candidate manifest comparison through `elda diff <pkg> --candidate`
- desired-state export and import through `elda state export` and `elda state import`, including remote-document replay, world-anchor reinstallation through the normal install path, and export of the persisted applied profile base instead of falling back to config defaults
- imported desired state now replays active profile anchors through the same profile-selection path used by `pf apply`, so machine-shape export/import now really round-trips active profiles, init policy, and arch policy on disposable targets instead of only persisting profile metadata
- world-backed upgrade execution through `elda u`, including version comparison against synced metadata, policy-aware blocking, conflict checks, same-transaction `replaces`, closure-aware install of newly required hard dependencies, fail-closed targeted-upgrade reverse-dependency coherence checks, `--refresh-weak-deps` / `refresh_weak_deps` support for newly introduced `recommends`, explicit non-adoption of weak deps unless policy allows them, and operator-visible keep-installed rows for pinned ad hoc git tag/revision refs instead of silently dropping them from dry-run plans
- profile and daemon commands through `elda pf show`, `elda pf apply`, `elda pf add`, `elda pf rm`, `elda pf set-init`, `elda pf clear-init`, `elda pf set-arch`, `elda pf add-foreign-arch`, `elda pf remove-foreign-arch`, `elda daemon status`, `elda daemon refresh`, and `elda fix-triggers`, with honest current-backend reporting instead of stub output for the current prefix slice
- profile application now installs selected profile anchors as `base`, persists init-provider and foreign-arch policy, removes deselected active profile anchors when safe, fails closed if the requested set relies on implicit profile anchors, and now immediately reconciles supported init-provider asset changes on the disposable-root `/usr` backend instead of leaving that slice permanently pending
- profile inspection now resolves machine shape from persisted/imported profile state, installed profile anchors, explicit config, and root/host detection instead of inventing a hardcoded `yoka-core` / `dinit` baseline on empty roots
- profile recipes can now declare first-class machine-shape defaults through `profile = { native_arch?, foreign_arches?, init? }`, and conflicting profile-policy declarations now fail closed during profile application instead of silently picking one
- the `pf` namespace now supports real edit-style mutations through `pf add`, `pf rm`, `pf set-arch`, `pf add-foreign-arch`, `pf remove-foreign-arch`, and `pf clear-init`, while `pf show` now exposes declared profile-policy metadata when the active anchors are locally resolvable
- `pf show`, profile dry-runs, profile edit commands, and `fix-triggers` now derive typed pending system-change handlers for init-provider transitions, foreign-arch policy, and unapplied profile-set reconciliation, report the strongest required activation class instead of hardcoding an empty handler set, and now compare desired profile state against an applied init-provider record on the current `/usr` backend instead of faking the applied side from config/defaults alone
- `rc add --kind profile` now scaffolds a profile-shaped `pkg.lua` instead of forcing profile authors to hand-roll the first recipe from scratch
- `rc show`, `rc diff`, and `rc publish-ready` now provide runtime-backed recipe inspection surfaces for local/synced metadata, local-vs-synced field comparison, and publish-readiness blockers before CI/forge submission
- the `ci` namespace now has a real local/filesystem-backed workspace under the Elda data dir: `ci sub`, `ci run`, `ci status`, `ci pr`, `ci retry`, `ci logs`, and `ci batch new/add/push` now write batch/submission records, commit a package-definition mirror, emit `lock-v1.json.zst`, publish signed local indexes plus copied payload artifacts, emit per-payload `.minisig` / `.spdx.json` / `.attestation.json` sidecars, optionally push submission refs to a configured submission remote/target branch, and expose the published state back through `forge search` / `forge browse`
- `ci pr` is now materially real for the current hosted-review slice too: PR-mode submissions can push their branch refs to the configured submission remote, push mode can update the configured target branch, compare / PR-style URLs now honor the configured base branch, and token-auth `ci pr` can open real GitHub/GitLab/Gitea-style hosted reviews through configured API bases instead of only reporting `null` or compare links
- the first `qa` and daemon execution slice is now real too: `qa lint`, `qa build`, `qa smoke`, `qa stack`, `qa repro`, `qa diff`, and `daemon run` all execute current-slice backend behavior instead of falling through to stub output, and the local Phase 9 scheduler path is now real through queued submission processing, retry orchestration, richer queue/status reporting, and per-remote hosted-review config resolution keyed to the submission's actual remote
- the install UX pass has now moved further into runtime instead of only in docs: human-mode install dry-runs and successful installs render structured target / resolution / plan / result blocks for the main install path, that rendering now surfaces the selected activation backend plus snapshot summaries when present, mutating commands now write persistent per-run session logs under the configured logging directory with level `1` / `2` / `3` control, install rendering surfaces the attached log path, interactive human installs now stop for a generated-metadata review gate when the current session scaffolded recipe metadata on the operator's behalf, grouped per-package progress output now frames package-definition fetch, source/binary acquisition, payload assembly, activation, and snapshot hooks more explicitly, installs with no configured remotes now fail with direct `rmt add` plus `sync` guidance instead of only a generic missing-package error, `rc edit` now opens real local recipe trees through `VISUAL` / `EDITOR` / fallback editor resolution, and the cargo source-build path now hardens rustup fallback handling for system-mode runs by avoiding the bogus `rustup run no cargo` toolchain selection and accepting TOML-form `rust-toolchain` files without the `.toml` extension
- human CLI rendering for `ls`, `state show`, `pf show`, `search`, `info`, `cache ls`, `check`, `verify`, `recover`, and `daemon status` that now shows real tables and state blocks instead of reducing everything to counters, with `cache ls` also exposing the active retention policy and current local cache usage
- the root help surface now has a branded custom screen with the Elda ASCII logo, grouped command sections, examples, and palette-aware terminal theming instead of raw Clap command dumps
- archived prefix-state capture plus real `elda rollback` restoration from cached payloads and manifests
- removal safety in prefix mode that blocks deleting required packages unless `--cascade` is used
- explicit live-host system-mode gating through `defaults.allow_system_mode` or `elda -S`, plus frontend privilege-provider auto-detect/re-exec for live host operations
- disposable system-mode installs under `/usr` now compose the next managed root under `var/lib/elda/states/<state-id>/root`, switch staged entries into the live root through per-path file-switch activation, record backend-aware `system-*` state IDs and `linux-copy` activation backend names, materialize declarative `sysusers` / `tmpfiles` / `alternatives` metadata under the target root, persist internal trigger plus boot status under `var/lib/elda/state/system-backend/`, and let `state show`, `check`, and `fix-triggers` report or repair the current backend slice honestly
- configured `snapshot_tool` requests are now wired for the current `/usr` backend slice: system install/remove transactions record pre/post activation snapshot attempts in their journal, surface them in mutation reports, and persist them into archived state metadata, while the current backend only executes `snapper` semantics and records unsupported tools as failed snapshot requests instead of silently ignoring them
- system-mode build/install now also parse, validate, persist, and reconcile typed `provider_assets`: Elda stores declared provider assets under `/usr/lib/elda/provider-assets/<family>/<provider>/<pkgname>/...`, materializes the active provider's targets into the disposable root, exposes declared and installed provider-asset visibility through `elda info`, and now lets `pf set-init`, `pf clear-init`, `pf apply`, `state import`, and `fix-triggers` reconcile the supported init-provider asset set from an applied backend-state record instead of reading desired profile policy directly
- disposable system-mode rollback now restores archived package system metadata, re-captures the reactivated archived state under the staged-state tree, and reruns system-trigger reconciliation from cached artifacts, with install/remove/fix-triggers/rollback coverage landing in disposable-root tests
- `elda info` now exposes machine-readable provider-asset visibility for the current slice: declarative `sysusers` / `tmpfiles` / `alternatives` / hook metadata, installed system-backend asset state for `/usr` packages, active provider-family state, and pending provider-specific handler transitions
- build and install fetch paths now honor `--offline` for git and archive sources instead of silently reaching out to the network, and archive sources now reuse the content-addressed local payload cache plus configured cache nodes before falling back to origin
- disposable prefix installs that are safe to test repeatedly without touching the host `/usr`
- real upstream binary-release validation through the `fsel` fixture recipe and synced-index smoke runs

What Elda is not yet:

- a live system `/usr` package manager
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
3. `USAGE.md`
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
- **Scope:** Implement bounded `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template` support in git mode, including fail-closed parsing rules, curated Gentoo shim support, and GPKG binary fast-path behavior.
- **Hardness:** H5
- **Dependencies:** Phase 5, Phase 4
- **Done-when:** Supported flake repos install through `elda i <git-url>` without the `nix` CLI, supported overlay packages install without Portage, and unsupported inputs fail closed with explicit errors.

### Phase 11: Interepo Translation and Coexistence
- **Scope:** Implement foreign-repo adapter plumbing, translated index snapshots, verification confidence levels, coexist/warn/lock modes, `ext ls`, and normal install flow for translated packages. This includes complex mapping translation for Alpine imperative `sysusers`, DKMS hook mapping, Gentoo conffile routing, strict transaction atomicity for `coexist` overlap management, and nixpkgs binary-cache consumption (no Nix evaluator) with ELF normalization via `arwen-elf` (pure-Rust patchelf rewriting `RPATH`/`PT_INTERP` to FHS paths so nixpkgs packages share system dependencies).
- **Hardness:** H5
- **Dependencies:** Phase 3, Phase 4, Phase 7
- **Done-when:** At least one foreign repository type syncs into translated metadata and installs through the normal resolver and transaction engine, confidence levels are surfaced in CLI output, and coexistence controls work safely in VM tests.

### Phase 12: Migration, Adoption, and `pkgit` Retirement
- **Scope:** Implement `mg from`, `adopt`, the required v1 migration adapters, provenance preservation, and the final cutover from `pkgit` as the active package-manager model.
- **Hardness:** H5
- **Dependencies:** Phase 11, Phase 8
- **Done-when:** The required adapter set exists for `pacman`, `apt`/`dpkg`, `apk`, `xbps`, and `portage`, adopted packages preserve provenance and pass `check`, and Elda can replace `pkgit` without relying on `pkgit` internals at runtime.

### Phase 13: Host-State Materialization and Atomic Activation
- **Scope:** Moss-inspired host-mode state engine for Yoka-shaped systems: merged managed-tree builder, ephemeral materialization, atomic `/usr` promotion via `RENAME_EXCHANGE`, rename-based previous-tree archives, content-pool hardlink blit, and `state apply` for profile/machine-shape transitions. Package-level `elda i` / `elda u` remain the delta path.
- **Hardness:** H5
- **Dependencies:** Phase 6, Phase 7, Phase 8 (not Phase 11)
- **Done-when:** `elda state apply <profile>` performs atomic host `/usr` handoff with archived rollback tree; ephemeral apply powers installer/CI without live mutation; plan output surfaces honest activation classes; VM tests prove swap + rollback without btrfs-only reliance.

## 8. Status Tracker

| Phase | Status | Slice State | Last Updated | Notes |
| --- | --- | --- | --- | --- |
| Phase 0: Fork Baseline and Workspace Skeleton | Completed | Current slice complete | 2026-04-11 | Rust workspace landed; `cargo check` passes; `elda --help` exposes the canonical namespaces; `fixtures/pkgit/` captures `pkgdeps`, `bldit`, direct git install, and repo search/list reference inputs |
| Phase 1: Core Types, Config, and State Skeleton | Completed | Current slice complete | 2026-04-12 | Canonical identity/version parsing and ordering tests landed; config defaults now include privilege auto-detect policy; SQLite bootstrap, world/journal/manifests layout, and mutation lock exist; `ls`, `state show`, and `check` return structured empty-state output against a bootstrapped root |
| Phase 2: Recipe Model and Legacy Import | Completed | Current slice complete | 2026-04-19 | `rc add` scaffolds declarative recipes and imports local `pkgit`-style sources; imported `pkgdeps` now become best-effort `depends` entries plus an explicit legacy summary file; `rc check` parses the current declarative Lua subset, validates core spec fields, understands declarative `build = { ... }` metadata, accepts dual-lane `source = { lanes = { ... } }` recipe definitions, and now preserves/validates the declared `sysusers`, `tmpfiles`, `alternatives`, `hooks`, `provider_assets`, flag-table, and `subpackages` metadata families without executing arbitrary build logic |
| Phase 3: Remotes, Indexes, Trust, and Cache | Completed | Current slice complete | 2026-05-15 | `rmt add`, `rmt info`, `rmt preview`, `rmt trust`, `rmt rm`, `cache add`, and `cache ls` persist/read/report TOML documents under `remotes.d/` and `caches.d/`; cache nodes now accept explicit priority, remotes can set priority/channel/packages-url/exclude policy directly through CLI flags, synced binary installs try configured caches before origin asset URLs, fetched payloads populate a content-addressed local archive cache that offline installs can reuse, and automatic local cleanup enforces the default retention policy while retaining installed-package and archived-rollback payloads; `cache ls` surfaces the active retention thresholds plus current local cache usage; `sync` fetches local or HTTP-backed index documents, filters records to the configured remote channel, verifies detached Ed25519-signed index sidecars for pinned or TOFU remotes, persists trusted public keys plus per-remote verified snapshot state, marks stale remotes instead of dropping them, supports `--offline` refresh against cached verified snapshots only, rejects implicit first-use TOFU enrollment in JSON/unattended sync, accepts signed TOFU key rotation through `metadata_url` plus `${metadata_url}.sig`, requires explicit operator confirmation through `--accept-rotated-key <remote>` before storing rotated TOFU keys, fails closed if offline or stale snapshot reuse would cross channel boundaries, and secure remote binary installs now verify indexed `payload_sig` values against the remote trust set before staging; source-capable synced remotes accept `packages_url`, interemote remotes can be previewed before sync, sync reports include dynamic-remote parser diagnostics, bounded per-package parser issue rows, package add/remove delta summaries with removed-package samples, and all-failed index-vs-interemote summaries, clears removed-remote stale package records, `rmt trust` reports configured trust policy plus persisted key/verification state, and `search` / `info` continue to query the merged verified snapshot |
| Phase 4: Resolver, Flags, and Planning Engine | Completed | Current slice complete | 2026-04-19 | `i`, `ig`, `ib`, `rm`, `u`, and `autoremove` now emit or execute materially real plans rooted in the current Elda state layout; install requests apply the documented lane-selection policy for both local recipes and synced remote packages; install dry-runs now show the actual dependency closure instead of only the top-level targets; explicit installs now auto-add satisfiable `recommends`; a Rust-native PubGrub-style solver now resolves exact dependencies, `any = { ... }` alternatives, versioned `provides`, virtual providers, and multi-target closure conflicts coherently across the requested transaction; ambiguous virtual providers still fail closed, config-backed provider policy is now real through `[resolver.provider_preferences]`, package-level `conflicts` now block invalid install and upgrade plans, same-transaction `replaces` now work for same-origin installs/upgrades while cross-origin replacement and replacement that would strand hard reverse deps still fail closed, upgrades can now pull in newly required hard dependencies from the same synced snapshot and reject targeted moves that would break installed reverse dependencies, install execution now upgrades already-installed dependencies when the solved candidate changed instead of silently keeping stale versions, and non-default variants still fail closed on binary-only targets while automatically falling back to source when a maintained source lane exists, including synced remotes that must fetch package-definition companion files through `packages_url` |
| Phase 5: Build, Staging, and Payloads | Completed | Current slice complete | 2026-04-23 | `elda-build` now clones `git` sources, auto-detects or honors the first declarative Cargo build path, stages files under canonical `/usr`, emits a `.pkg.tar.zst` payload plus `.manifest`, records payload/manifest hashes, normalizes direct ad hoc git installs to commit-derived versions, supports the real binary-lane staging path for `url_archive` and `github_release`, parses/validates/stages arch-specific `github_release` asset tables, collects typed `provider_assets` into persisted system metadata, performs post-stage ELF shared-library analysis to enrich built package metadata with detected `shlib_provides` / `shlib_requires`, consumes indexed remote `asset_url` / `sha256` metadata for synced binary installs, verifies secure-remote payload signatures before staging, and now has working `vendor add` / `vendor import` / `vendor export` recipe generation for local convenience binaries, including GitHub release asset auto-detection for the current OS/arch when the match is unambiguous; source-capable synced remote installs now materialize pinned package-definition trees from `packages_url` plus indexed `repo_commit` before build so source-only maintained remotes and explicit source-lane synced installs can consume `build.lua`, patches, and companion metadata; the companion `elda-populate` tool now promotes already-built local payloads and synced remote payloads into configured caches with digest verification, optional cache-seed manifests, and explicit channel filtering for maintained-remote mirroring; installed provenance now normalizes author-facing fetch/build kinds into canonical persisted `source_kind` values for local recipes, ad hoc git installs, repo binaries, and interbuild sources |
| Phase 6: Prefix Transaction Backend | Completed | Current slice complete | 2026-04-23 | `elda i` now performs a real manifest-backed prefix install for local/ad hoc git targets, maintained dual-lane recipes, synced remote package names, and the first direct dependency closures, including `ig` / `ib` lane selection and correct world-vs-dependency install reasons; `elda rm`, `elda autoremove`, `elda files`, and `elda files owner` operate on recorded ownership data in the disposable root, with reverse-dependency protection unless `--cascade` is used; prefix transactions persist journals, block new mutations until recovery, support `elda recover`, support manifest-backed `elda verify` / `elda reverify`, archive committed prefix states so `elda rollback` can restore the previous or a named archived state from cached payloads, implement archive-backed `elda downgrade` with dry-run planning and reverse-dependency version checks, implement spec-shaped conffile handling through `*.eldanew` / `*.eldasave` plus `--purge-conffiles`, and now carry explicit regression coverage that unmanaged path collisions fail closed before activation |
| Phase 7: Linux System Backend and Trigger Engine | Completed | Current slice complete | 2026-04-20 | System-mode installs under `/usr` now compose the next managed root under `var/lib/elda/states/<state-id>/root`, switch staged files into the live root through per-path file-switch activation, record `linux-copy` activation backend state plus backend-aware `system-*` state IDs, persist archived system-state metadata, materialize declarative `sysusers` / `tmpfiles` / `alternatives` assets, store typed provider assets under `/usr/lib/elda/provider-assets/<family>/<provider>/<pkgname>/...`, materialize the active provider's targets into the disposable root, persist an applied init-provider backend-state record under `var/lib/elda/state/system-backend/`, run an internal trigger engine with persisted trigger plus boot status, expose activation-backend capability/boot reporting through `state show`, `check`, and `fix-triggers`, record configured pre/post activation snapshot requests into journals, reports, and archived state metadata for install/remove transactions, expose provider-asset visibility through `elda info`, and keep rollback-aligned staged roots in disposable-root tests |
| Phase 8: Profiles, Machine Shape, and Ops Surface | Completed | Current slice complete | 2026-04-23 | `pf show`, `pf apply`, `pf add`, `pf rm`, `pf set-init`, `pf clear-init`, `pf set-arch`, `pf add-foreign-arch`, `pf remove-foreign-arch`, `daemon status`, `daemon refresh`, and `fix-triggers` now all have real current-slice handlers; profile recipes can declare machine-shape defaults through `pkg.profile`, conflicting declarations fail closed, `rc add --kind profile` scaffolds first-class profile recipes, and `state export` / `state import` now really round-trip machine shape by replaying imported active profile anchors plus persisted init/native/foreign-arch policy on disposable targets |
| Phase 9: Native CI and Binary Publishing | Completed | Current slice complete | 2026-04-25 | `ci sub/run/status/pr/retry/logs`, `ci batch new/add/push`, `forge search/browse`, `qa lint/build/smoke/stack/repro/diff`, and `daemon run` now execute a real local/filesystem-backed package-definition and publish workspace, including copied artifacts, `lock-v1.json.zst`, signed local index output, per-payload `.minisig` / `.spdx.json` / `.attestation.json` sidecars, indexed `sbom_url` / `attestation_url` metadata, configurable submission remote/base-branch targeting, local scheduler queue processing through bare `ci run`, retry/state tracking, compare/PR-style URL derivation from recognizable forge remotes with configured base-branch awareness, and token-auth hosted review creation through `ci pr` with per-remote override resolution for GitHub/GitLab/Gitea-style APIs; the current native CI slice is still intentionally local-first and file-backed, but the documented Phase 9 proof points are now met |
| Phase 10: Git-Mode Interbuilds | Completed | Current slice complete | 2026-05-10 | Bounded parser-backed source-lane installs are live for `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template`; `elda a` / `elda add` performs metadata-only raw-link resolution with config-ordered source strategy detection, field-confidence reporting, actionable `--source-option` / `--strategy` selection, ad hoc git add/install/update `--to-branch` / `--to-tag` / `--to-rev` metadata pinning, pinned git-ref keep-installed dry-run reporting, config-backed git tag policy defaults, and generated AUR/XBPS metadata filling; Gentoo/AUR/XBPS reports include accepted phase command lines, and the human interbuild review gate now includes parser-specific metadata/dependency-family context; AUR/XBPS fail closed on unsupported shell expansion, mismatched source/checksum counts, or mismatched AUR arch-specific source/checksum counts; the binary lane accepts provider-aware `release_asset` for GitHub/GitLab/Gitea/Forgejo/SourceHut/direct manifests; human `list-options` installs can prompt for a source option on TTYs; ad hoc git packages can source-ref downgrade with `downgrade --to-tag/--to-rev`; `elda git tags` / `elda versions` list local/remote tags with normalized version confidence and can optionally join matching releases through `--with-releases`; `elda git releases` inspects GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest release assets with OS/arch/libc/format compatibility scoring and self-hosted forge host preservation; and raw release links can convert to pinned `github_release` or `release_asset` metadata when a digest or checksum sidecar is available while keeping source fallback selected when checksum data is missing; the metadata generation review gate now correctly detects missing `binary` fields for release assets and the Lua parser properly handles generated empty tables without false-positive validation errors; GPKG binary fast-path, generated source+binary lanes for checksum-backed git release assets, and release signature sidecar metadata validation are wired for the Phase 10 slice; full install-time signature trust remains post-prephase until the key/format contract is frozen. |
| Phase 11: Interepo Translation and Coexistence | Architectural Research Complete | Architectural baseline, runtime partial | 2026-04-23 | Depends on native install and verification path |
| Phase 12: Migration, Adoption, and `pkgit` Retirement | Started | Started | 2026-05-13 | First database-backed `adopt` / `mg from` slice landed for pacman, apt/dpkg, apk, xbps, and portage installed-state import; live lock/unlock takeover and complete provenance/check integration remain |
| Phase 13: Host-State Materialization and Atomic Activation | Not started | Planning only | 2026-05-18 | Merged-tree materialization, atomic `/usr` promotion, rename-based state archives; 0% runtime |

## 9. Changelog

### v0.1.49 - 2026-05-18

- Release **0.1.49-Sumomo**: workspace version bump, `elda -V` and `elda version` detailed build/schema reporting.
- Public docs (`README.md`, `USAGE.md`, `SPEC.md`, `eldaforgehosting/`, `checklist.md`, `man/elda.1`) updated for landed `host` / `publish` maintainer workflows and public-doc surface rules.
- Phase 13 (host-state materialization) added to build order and status tracker.

### v0.1.48 - 2026-05-18

- Phase 13 planning slice added to build order and status tracker (host activation: merged tree, ephemeral apply, atomic `/usr` promotion, rename archives).
- Pre-Phase-11+ native CLI slice remains **100%** for named runtime surfaces.

### v0.1.47 - 2026-05-18

- Finalized native hosting layout in public docs: recipe monorepo default, per-channel index, `test-tree` dry-run default with `--install` opt-in, `publish finalize` as sole URL rewrite, `/etc/elda/host.d/` maintainer profile, configurable `mr_mode`, interemote = interbuild at sync. Resolved checklist `D-04`.

### v0.1.46 - 2026-05-19

- Froze interepo/hosting contract answers in `SPEC.md`: foreign hook translation (no universal `.hook` runner), RPM transfiletrigger stdin, ALPM `NeedsTargets` paths, SELinux labeling, CachyOS microarch tier sync policy, NixOS host refusal with pkgit redirect, release-asset trust keys, SIGINT/recover semantics, and post-transaction advisories.
- Updated interepo study notes to match those decisions (internal research tree).

### v0.1.45 - 2026-05-18

- Split native forge hosting into `eldaforgehosting/` with per-topic and per-platform how-tos (source-only, binhost, cache, trust, interemotes, recipe-to-Git paths, GitHub/GitLab/Gitea-Forgejo/SourceHut/static HTTP, and AUR/binhost/full-forge/LAN-mirror/staging patterns). `eldaforgehosting.md` remains a short redirect.
- Reconciled git-source UX notes against landed pre-Phase-11+ CLI (`--pick-tag`, dispatch `[Y/n/e]`, `rc format`/`normalize`, `forge fork`).

### v0.1.44 - 2026-05-16

- Updated the public operator documentation set around the current pre-Phase-11+ runtime: `README.md` is now a concise project entrypoint, `USAGE.md` covers current command families, `eldaforgehosting/` (then `eldaforgehosting.md`) has explicit source-only, binary, cache, GitHub/GitLab/Gitea, and interemote hosting examples, and `man/elda.1` now exists as a local man page source.
- Refreshed `config.toml`, `su/config.toml`, `examples/config/`, and `fixtures/config/` so the sample `/etc/elda` shape reflects current runtime fields, metadata replacement policy, git transport policy, dynamic interemote remotes, source-capable `packages_url` remotes, cache documents, and extension documents.
- Added copyable heather-overlay and blackhole-vl interemote remote documents to examples/fixtures so `rmt preview`, `sync <remote>`, `--exclude`, and `rmt rm` are visible from real files instead of only prose.

### v0.1.43 - 2026-05-13

- Started Phase 12 with a real migration/adoption runtime path: root-level `elda adopt --from <pm> <pkg>` and `elda mg from <pm>` now route through `crates/elda-core/src/app_migration/*` instead of the generic stub handler.
- Added bounded database readers for `pacman`, `apt`/`dpkg`, `apk`, `xbps`, and `portage` installed-state databases. The current slice imports package name, normalized `epoch:pkgver-pkgrel`, architecture when available, owned file paths when available, dependency text, and source repo/channel hints when the foreign database exposes them.
- Adopted packages are recorded through the normal Elda DB path with `source_kind = "adopted"`, explicit install reason, imported ownership records, dependency records, and a non-mutating `adopted-live-root` activation marker. The adoption path does not copy, remove, or rewrite live files.
- Safety checks now reject adoption when Elda already owns the same package identity or any imported managed path, and `mg from` preflights the whole foreign batch for duplicate packages and overlapping foreign ownership before committing records.
- Human-mode migration/adoption output now renders framed operator blocks through `app_render_migration.rs`, while JSON keeps the full structured package import data.
- Added regression coverage in `crates/elda-core/src/tests/migration.rs` for pacman single-package adoption, apt/dpkg whole-system import, and path-conflict rejection.

### v0.1.42 - 2026-05-08

- Extended the recipe flag system inside Phase 10 source-lane work as outlined in
  `eldastudyuseflags.md`: `pkg.lua` now declares `flags_descriptions`,
  `flags_required_one_of`, `flags_required_at_most_one`, and `flags_required_any_of`
  alongside the existing flag tables, and dependency families accept conditional entries via
  `{ name = "constraint", when = "+flag,-other" }` (and the `any = { ... }` form).
- Added atom-versioned package flag overrides: `[flags.package."<name><op><version>"]` entries
  in `config.toml` now contribute to the package layer only when the resolved candidate's
  `epoch:pkgver-pkgrel` satisfies the constraint, so version-scoped overrides no longer rebuild
  every release of the matching package.
- Hardened `variant_id`: `"default"` is reported when the resolved effective flag set matches the
  declared defaults; otherwise it remains the canonical `v1-<sha256_prefix>` digest. The
  `customized` flag continues to gate the binary-lane policy (binary lanes are forbidden for
  customized variants).
- Solver wiring: dependency entries with a `when` predicate are filtered through the resolved
  effective flag set in `crates/elda-core/src/app_install/solver/graph.rs::filter_dependency_entries`
  before expansion, so unmatched conditional deps never consume choice slots or appear in plan
  output. Cardinality groups are evaluated after implies/conflicts close and fail closed with a
  structured operator error pointing at the offending group/members.
- CLI surface: `elda fl check [<package>] [--use=+a,-b]` and `elda fl diff [<package>] [--use=+a,-b]`
  now accept one-shot `--use=` previews and surface flag descriptions, the active per-package
  layer sources (per-name and per-atom), and cardinality group status alongside the existing
  variant id and flag delta. `elda u --rebuild-variant-drift` pre-fills the upgrade target list
  with every installed package whose resolved variant id no longer matches the recorded one.
- Human render surface: `app_render_extended/profile_policy.rs` adds explicit `flag deltas`,
  `Package overrides`, `Cardinality groups`, and `Flag descriptions` sections so the framed
  human report mirrors the structured JSON detail tree.
- Fixtures + docs: added `fixtures/recipes/flag-suite-demo/pkg.lua` exercising every new field and
  the conditional-dep predicate, extended `fixtures/config/profile-defaults.toml` with both
  unversioned and atom-versioned package overrides, and updated `SPEC.md`,
  `SPEC.md`, `USAGE.md`, and `phase.md` to describe the new
  recipe surface, configuration surface, variant identity contract, solver wiring, and CLI shape.
- Tests: added `crates/elda-core/src/tests/flags.rs` coverage for cardinality `one-of` blocking and
  passing paths, conditional dependencies with the flag both off and on, atom-versioned override
  matching/non-matching cases, `fl check` description/cardinality output, and the
  `--rebuild-variant-drift` upgrade flow; added recipe-level parse tests in
  `crates/elda-recipe/src/check/tests/parse.rs` for `when` predicates, undeclared-flag rejection,
  and cardinality table parsing.

### v0.1.41 - 2026-05-04

- Completed the Phase 10 git/release UX pass: `elda git releases` now models SourceHut tag
  artifacts and direct `.elda-releases.json` manifests alongside GitHub/GitLab/Gitea;
  `release_asset` validation/build staging accepts `sourcehut`, `direct`, and self-hosted forge
  `host` metadata for provider-neutral release assets; generated release metadata records matching
  signature sidecars; human `list-options` install flows can prompt for a source option on TTYs; and
  `elda downgrade <pkg> --to-tag/--to-rev` rebuilds installed ad hoc git packages from older pinned
  source refs through normal transaction handling.

### v0.1.40 - 2026-05-04

- Made plain `elda u` dry-runs for pinned ad hoc git tag/revision installs report explicit
  `keep-installed` actions with `blocked_reason = "git-ref-pinned"`, `source_ref`, and installed
  commit details instead of hiding the pinned no-op by returning an empty plan.
- Extended human upgrade-plan rendering to show git ref policy, installed/candidate commits, and
  blocked reasons for git-source upgrade rows.

### v0.1.39 - 2026-05-01

- Added config-backed `metadata.link_option_mode` with `priority` as the default and `list-options` as an opt-in source-option reporting mode for ad hoc/local-source metadata resolution; explicit `--source-option <N>` makes listed options actionable, and human non-dry-run install flows can now open a TTY-gated selector.
- `elda a` / `elda add` and raw-link install reports now carry detected source options, selected default option, and priority-ordered human list output when list-options mode is enabled.
- Extended the bounded release-source slice: `release_asset` now validates and stages pinned GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest assets, `elda git releases` normalizes GitHub/GitLab/Gitea/Forgejo release APIs plus SourceHut tag artifacts and direct manifests, and raw release options convert to pinned `github_release` or provider-neutral `release_asset` metadata when a digest or checksum sidecar is available.
- Added read-only `elda git tags <repo-or-path>` and `elda versions <repo-or-path>` inspection backed by `elda-git`, including object IDs, normalized package-version candidates, confidence labels, JSON output, human rendering, and optional `--with-releases` tag-to-release asset joins.
- Added read-only `elda git releases <owner/repo-or-forge-url>` inspection for GitHub/GitLab/Gitea/Forgejo releases, later extended to SourceHut tag artifacts and direct manifests, including normalized tag versions, payload/checksum/signature classification, OS/arch/libc/format detection, native compatibility scoring, recommended asset selection, JSON output, and human rendering.
- Added explicit ad hoc git metadata ref selectors: `--to-branch`, `--to-tag`, and `--to-rev`.
- Added `git_release` to the config-backed raw-link source-option order. Raw GitHub links now surface detected release assets with tag/asset/compatibility/checksum metadata in `list-options` reports, select checksum-backed release metadata when safe, and keep source fallback selected when checksum data is unavailable.

### v0.1.38 - 2026-04-30

- Extended the bounded Phase 10 source-lane interbuild runtime beyond Nix/Gentoo: `aur_pkgbuild` and `xbps_template` now validate source metadata without `makepkg`, `pacman`, or `xbps-src`, then reuse the normal Elda build/stage/install path for supported source trees.
- Interbuild JSON and human install output now include AUR/XBPS parser details, confidence, external-CLI status, dependency counts, and provenance tier data instead of falling back to generic parsed-source output.
- `elda a` / `elda add`, direct local-source installs, ad hoc git installs, and CI target scaffolding now honor `[metadata].link_strategy_priority` rather than using a hardcoded source-family order.
- Generated `pkg.lua` files now fill safe AUR/XBPS metadata fields such as description, licenses, upstream URL, version, release, dependencies, make/check dependencies, provides, conflicts, and replaces when those fields are present in the foreign source definition.
- Interbuild parser reports now expose bounded phase command lines for Gentoo ebuild phases, AUR `build`/`package` functions, and XBPS `do_build`/`do_install` functions; AUR/XBPS now fail closed on unsupported shell expansion instead of reporting opaque function names only, and the AUR parser validates and reports arch-specific `source_<arch>` arrays against matching checksum arrays.
- Added git-source handling study notes comparing Elda's recipe/ad hoc git semantics against tag-first and release-asset selection models, including provider-neutral release discovery, explicit ref-switching, asset classification, and source-ref downgrade UX.
- Extended install UX notes with git tags, provider-neutral release assets, source-version UX, and dedicated downgrade TUI direction. This is a UX proposal and implementation queue, not a landed runtime behavior claim.

### v0.1.37 - 2026-04-29

- Started Phase 10 with real parser-backed git-mode interbuild execution for the bounded source-lane subset: `nix_flake` now validates static default installables from `flake.nix` / optional `flake.lock` without invoking `nix`, and `gentoo_overlay` now validates selected EAPI 8 ebuilds, curated eclasses, required metadata, and simple phase bodies without invoking Portage.
- Interbuild sources now reuse the normal Elda git checkout, build detection, stage-root, manifest, payload, activation, and installed-state paths, so successful interbuild installs record canonical `interbuild` provenance and unsupported inputs fail closed instead of pretending to run foreign package managers.
- Install progress now surfaces explicit parser/translation steps for `nix_flake` and `gentoo_overlay`, and new regressions cover successful Nix-flake and Gentoo-overlay installs plus fail-closed unsupported Gentoo EAPI behavior; re-verified with `cargo fmt --all`, targeted interbuild/build-system tests, and `cargo test --workspace`.
- Advanced human install output now renders target, resolution, provenance, risk, progress, artifacts, result, and log sections from the same JSON report data; interbuild actions carry parser/engine/confidence detail, and runtime failures now return structured `blocked` reports with command context, operator action, and the documented nonzero exit status mapping.
- Phase 10 moved from parser validation only to extracted parser metadata and operator review: Nix lockfiles now report allowed locked-input shape and locked-input counts, Gentoo ebuilds now report EAPI, DEPEND/RDEPEND/BDEPEND/IUSE/KEYWORDS/phase metadata, install results carry typed interbuild reports, and human interactive installs stop at an interbuild review gate before build execution.
- Documentation contract update: `elda a` / `elda add` is now the metadata-first raw-link path, `elda i <link>` reuses the same metadata/source strategy before install, and the UX/interbuild/study docs now define field-level metadata provenance for `description`, `licenses`, `upstream`, dependencies, relationships, variants, and missing-field review.
- Runtime update: root-level `elda a` / `elda add` now resolves raw links/local sources through the same metadata path as install, detects local native metadata/Nix flake/Gentoo overlay strategies before generic build-system fallback, writes local metadata without installing, reports field confidence/missing publish fields, and carries regression coverage; Gentoo interbuild reports now expose DESCRIPTION, HOMEPAGE, LICENSE, SRC_URI, and SLOT alongside dependency/flag metadata.
- Config update: sample `config.toml`, `su/config.toml`, and config fixtures now carry the runtime-backed `[metadata].link_strategy_priority` order used by `elda a` / `elda add`; config parsing has a `MetadataConfig` shape plus fixture-load regression coverage.

### v0.1.36 - 2026-04-25

- Closed Phase 9 for the current native slice: the local `ci` workspace now has a real queued scheduler path behind bare `ci run`, `ci retry` re-enters that scheduler with recorded attempt/error state, and `ci status` / `ci logs` surface the richer queue/orchestration metadata instead of only flat submission records.
- `ci pr` now resolves hosted-review auth/base-branch settings against the submission's actual remote instead of assuming the default configured target, and new CI coverage proves per-remote overrides are honored when the stored submission remote changes.
- Install UX moved another real step forward in runtime: install progress shaping now lives in `crates/elda-core/src/app_install/progress.rs` instead of bloating `app_install.rs`, human output groups progress by package with clearer source/binary step detail, installs without configured remotes now point directly at `elda rmt add ...` plus `elda sync`, and the workspace is green again after carrying the newer `sbom_url` / `attestation_url` fields into `elda-populate` test fixtures.

### v0.1.35 - 2026-04-23

- Closed the proof gap for Phase 8: `state import` now reapplies imported active profile anchors through the normal profile-selection path, and the new Phase 8 regressions prove machine shape now round-trips across disposable roots instead of only persisting profile metadata.
- Landed the first real local Phase 9 runtime slice in `elda-core`: `ci`, `forge`, `qa`, and `daemon run` now have filesystem-backed handlers that maintain local submission/batch records, publish copied payload artifacts, write `lock-v1.json.zst`, emit per-payload signature/SBOM/attestation sidecars, and sign a local index that normal Elda sync/install flows can consume.
- The CI workspace can now also publish submission refs to a configured submission remote and base branch: PR mode pushes a submission branch, trusted push mode updates the configured target branch instead of assuming `main`, submission records persist the pushed remote/ref metadata, and focused CI tests now cover both the legacy `origin/main` path and a non-default remote/base-branch path against bare local remotes.
- `ci pr` can now also create real hosted reviews for the current slice: token-auth GitHub/GitLab/Gitea-style origins can open PR/MR records through configured API bases, created review URLs/IDs persist on the submission record, compare/PR-style fallback URLs honor the configured base branch, and the human CI renderer now exposes the remote/review metadata instead of burying it in JSON.
- Split the new `app_ci/*` and expanded profile-selection code back under the repo file-size ceiling so the Phase 8/9 runtime work still adheres to the code-standards direction instead of reintroducing oversized dispatcher slabs.

### v0.1.34 - 2026-04-20

- The `/usr` system backend no longer copies staged files straight into place: activation now materializes per-path switch targets and renames them into the live root, so the current backend finally performs a real staged file-switch cutover instead of direct copy-overwrite behavior.
- `elda-linux` now exposes activation-backend capability flags, `elda-install` persists boot status under `var/lib/elda/state/system-backend/boot.json`, and `state show`, `check`, and `fix-triggers` now surface honest backend capability plus boot-trigger status for the current slice.
- Added regressions for backend capability reporting, boot-input tracking, and critical boot-trigger visibility, then re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.33 - 2026-04-19

- Replaced the old recursive direct-closure dependency planner with a Rust-native PubGrub-style solver graph under `crates/elda-core/src/app_install/solver/`, and wired both install and upgrade planning through that shared solver boundary.
- Added config-backed provider policy through `[resolver.provider_preferences]`, so virtual-provider choice can now be steered explicitly while unresolved ambiguity still fails closed.
- Install planning/execution now handles coherent multi-target conflicts, `any = { ... }` alternative backtracking, dependency-driven upgrades of already-installed packages, and solver-backed weak-dependency selection instead of only recursive closure collection.
- Added resolver regressions for provider-preference override, cross-target version conflicts, and conflict-driven `any` alternative backtracking, then re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

### v0.1.32 - 2026-04-19

- The disposable-root `/usr` backend now persists an applied init-provider record under `var/lib/elda/state/system-backend/profile-state.json` instead of inferring the current provider family directly from desired profile policy.
- `pf set-init`, `pf clear-init`, `pf apply`, `state import`, and `fix-triggers` now reconcile the supported init-provider asset set immediately on the current `/usr` backend, while `pf show` and dry-runs compare desired policy against that applied backend state instead of reporting a permanently deferred init transition after the backend has already switched assets.
- Added regressions that prove init-provider changes switch provider assets immediately in system mode, `fix-triggers` repairs drifted provider assets from the applied backend state, and prefix-only pending-handler tests now pin the backend explicitly instead of relying on the default `/usr` mode.
- Re-verified the workspace with `cargo fmt --all` and `cargo test --workspace`.

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

- Implemented local cache retention and garbage collection against the frozen defaults in `SPEC.md`: 90-day payload retention, 30-day source/archive retention, and cleanup once usage crosses the smaller of 20 GiB or 10% of the backing filesystem.
- Added cache-entry access metadata so source artifacts, built payloads, manifests, and rollback-restored payloads can refresh their local retention timestamps without relying on filesystem atime behavior.
- Automatic cleanup now retains payloads needed by currently installed packages and archived rollback states, and `elda cache ls` now reports the active cache policy plus current local cache usage.

### v0.1.23 - 2026-04-14

- Tightened first-use TOFU behavior so JSON and unattended sync paths no longer auto-enroll trust for a remote on first contact.
- Human sync can still bootstrap a TOFU remote once, and later noninteractive sync reuses the persisted trust state instead of re-enrolling implicitly.
- Added regressions in both `elda-repo` and `elda-core` for allowed and denied first-use TOFU enrollment, and updated the import/repo fixture coverage to use pinned trust where unattended sync is intended.

### v0.1.22 - 2026-04-14

- Added real cache-node priority handling through `elda cache add --priority` and sorted `cache ls` output.
- Synced binary installs now try configured cache nodes before the indexed origin asset URL, using a content-addressed `<cache base>/<sha256>` lookup contract defined in `SPEC.md`.
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
- Updated status ledgers to reflect the current codebase instead of leaving the audit stale.

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
- Added end-to-end tests for both live drift detection and candidate-manifest comparison, and updated status ledgers to track the shipped behavior.

### v0.1.17 - 2026-04-12

- Implemented archived prefix-state capture in `elda-install` and wired a real `elda rollback` command through `elda-core`.
- `elda rollback` now restores the previous archived prefix state by default and can also target a named archived state id, rebuilding installed state from cached payloads and manifests instead of re-running the source build.
- Added a real rollback regression over a local package upgrade path, including default-target selection that skips the intermediate remove-only archive emitted during upgrade transactions.
- Verified the rollback slice with workspace fmt/tests/clippy and updated status ledgers to reflect that prefix rollback is now code-backed and test-backed.

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
