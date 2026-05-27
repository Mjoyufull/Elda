# Elda Master Checklist (Kanban Tracker)

This is the single development tracker for what is done, what is in progress, and what is still missing in Elda.

Status source used for this tracker:
- `SPEC.md` (behavior contract)
- `phase.md` (implementation ledger)
- `USAGE.md` (operator workflows)
- `eldaforgehosting/` (native hosting guides)

---

## Legend

- `[x]` done
- `[~]` in progress / partially shipped
- `[ ]` not started
- `(!)` blocker or high-risk gap
- Status labels describe the current implemented slice; they are not full-product completion claims

---

## Global Kanban Board

| Done | In Progress | Todo |
|---|---|---|
| [x] Workspace/crate baseline exists (`elda-cli`, `elda-core`, `elda-db`, etc.) | [~] Repo/trust/cache slice is real but not full spec-complete supply-chain model | [ ] Broader daemon/system-management lifecycle on top of the current backend |
| [x] SQLite installed-state DB + ownership model + manifests | [~] Build/runtime source-kind surface still narrower than declared model (bounded interbuild families now real, full semantics still partial) | [ ] Broader interbuild semantics beyond bounded parser-backed source installs |
| [x] Recipe parse/check flow for `pkg.lua`, with legacy import path | [~] `ext ls`, `adopt`, `mg from`, and `doctor` now have bounded runtime paths; all remaining unsupported command paths fail closed instead of returning success | [~] Replace remaining fail-closed unsupported command surfaces with real backend implementations only when they are in release scope |
| [x] Source/binary lane model (`i`, `ig`, `ib`, `--prefer-*`) | [~] Prefix rollback is real and the current `/usr` backend now performs staged switch activation with archived-state rollback parity | [ ] Broader daemon/system-management layers on top of the current backend |
| [x] Direct git installs + multi build-system execution (`cargo/cmake/meson/make/go/zig/python/nimble`) | [~] Profile/machine-shape slice is complete for the current target roots, but broader daemon/system-management integration remains | [ ] Full typed trigger engine + provider-family/system-change handlers on live backend |
| [x] Vendor workflow (`vendor add/import/export`) | [~] Install/bootstrap UX now has real review-gate, preflight, grouped progress, logging, better missing-remote guidance, and `elda doctor` bootstrap/release-readiness checks | [ ] Broader guided `setup` and install/takeover UX beyond the current `doctor` check |
| [x] Sync/search/info from verified merged snapshots | [~] Interepo/migration architecture documented with full store-path normalization pipeline for nixpkgs | [ ] Interepo adapters + translation confidence flow + coexistence modes + full nixpkgs normalization (ELF + shebangs + wrappers + text + symlinks) |
| [x] Trust model baseline: signed index, TOFU/pinned, explicit rotated-key acceptance, offline verified snapshots | [~] Replacement claim is still blocked by remaining pkgit-workflow coverage gaps from `stage.md` | [ ] Reach "pkgit replacement ready" gate and then full spec-complete gate |
| [x] Policy + introspection (`pin`, `hold`, `why`, `rdeps`, `autoremove`, `diff`, `downgrade`) | [~] File-size/code-standards cleanup improved, but latest scan still has 57 Rust files over the 350-line soft limit | [ ] Finish standards cleanup and keep large files split sustainably |
| [x] Prefix-safe transaction + verify/recover + conffile behavior (`.eldanew`/`.eldasave`) | [~] The current `/usr` backend is real; broader system-management and later-phase Linux work remain | [ ] Finish broader daemon/system-management behavior on top of the backend |
| [x] Full PubGrub-style dependency solver with provider-policy/config control | [~] The current `/usr` backend is real; broader system-management and later-phase Linux work remain | [ ] Finish broader daemon/system-management behavior on top of the backend |

---

## Phase Tracker (from `phase.md`)

| Phase | Status | Slice State | Checklist |
|---|---|---:|---|
| Phase 0: Fork baseline + skeleton | Done | Current slice complete | [x] Workspace landed; [x] canonical CLI surface wired; [x] pkgit fixtures captured |
| Phase 1: Core types/config/state | Done | Current slice complete | [x] identity/version parsing; [x] config defaults; [x] SQLite bootstrap + layout + lock |
| Phase 2: Recipe model + legacy import | Done | Current slice complete | [x] `rc add`; [x] `rc check`; [x] parser/validation for declarative metadata families, including `provider_assets` |
| Phase 3: Remotes/index/trust/cache | Completed | Current slice complete | [x] signed snapshot sync; [x] TOFU/pinned trust; [x] metadata rotation acceptance; [x] source-capable synced remotes can declare `packages_url` for pinned package-definition fetches; [x] cache priority + retention cleanup landed; [~] broader trust/cache completeness remains later hardening, not a Phase 3 blocker |
| Phase 4: Resolver/flags/planning | Completed | Current slice complete | [x] PubGrub-style install/upgrade solver; [x] conflicts/replaces/pin/hold checks; [x] config-backed provider preferences through `[resolver.provider_preferences]`; [x] synced source-lane resolution can require remote `packages_url` when the selected lane needs a real package tree |
| Phase 5: Build/staging/payloads | Completed | Current slice complete | [x] stage + manifest + payload; [x] git/archive/github_release paths; [x] synced source builds materialize pinned `packages/<pkgname>/` trees from `packages_url` + `repo_commit`; [x] build metadata now captures typed `provider_assets`; [~] source-kind parity is a later source-model/runtime gap, not a Phase 5 blocker |
| Phase 6: Prefix transaction backend | Completed | Current slice complete | [x] install/remove/upgrade journals + verify/recover; [x] rollback + downgrade in prefix; [x] prefix transaction parity for the documented slice landed |
| Phase 7: Linux system backend + triggers | Completed | Current slice complete | [x] staged `/usr` backend; [x] live file-switch activation; [x] internal trigger engine + `check`/`fix-triggers`; [x] archive/rollback parity on the current `/usr` backend; [x] provider-asset storage/materialization/reconciliation on the current backend; [x] applied init-provider backend state persists under `state/system-backend/` and drives current provider-asset materialization; [x] configured `snapshot_tool` requests now record pre/post activation attempts into journals, reports, and archived state metadata for system install/remove transactions; [x] activation-backend capability reporting plus persisted boot status landed |
| Phase 8: Profiles/machine shape/ops | Completed | Current slice complete | [x] `pf show/apply/add/rm/set-init/clear-init/set-arch/add-foreign-arch/remove-foreign-arch`; [x] state export/import; [x] profile recipes can declare `pkg.profile` machine-shape defaults; [x] imported desired state now really reapplies active profile anchors and round-trips machine shape; [~] broader daemon/system-management integration remains later work |
| Phase 9: Native CI + binary publishing | Completed | Current slice complete | [x] local CI submission pipeline; [x] local DAG/layer generation; [x] local artifact/signature/index publish path; [x] compressed `lock-v1.json.zst` output plus per-payload `.minisig` / `.spdx.json` / `.attestation.json` sidecars; [x] indexed `sbom_url` / `attestation_url` metadata; [x] `forge search/browse`, `qa *`, and `daemon run` current-slice handlers; [x] `ci pr` now derives compare/PR-style URLs from recognizable forge remotes with configured base-branch awareness; [x] git-remote submission publication landed for configurable submission remote/base-branch targets in PR and trusted push modes; [x] hosted review creation landed for token-auth GitHub/GitLab/Gitea-style `ci pr`; [x] per-remote submission override resolution now follows the submission's actual remote; [x] local scheduler/orchestration landed through bare `ci run`, retry-state tracking, and richer queue/log visibility |
| Phase 10: Git-mode interbuilds | Completed | Current slice complete | [x] bounded `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template` source installs; [x] parser metadata reports and interbuild review gates; [x] config-ordered add/link strategy plus explicit `--source-option` / `--strategy`; [x] ad hoc git ref selection and source-ref downgrade; [x] provider-aware `release_asset` binary lane through GitHub/GitLab/Gitea/Forgejo/SourceHut/direct manifests; [x] release signature sidecar metadata and safe-field validation; [x] `elda git releases --tag <ref>`; [x] `[git].allowed_protocols` fail-closed clone gating; [x] GPKG binary fast path; [x] installed file search, config queue resolution, trigger inspection, and dynamic remote/interemote UX; [~] install-time release signature trust, broader Nix evaluation, and full PKGBUILD/XBPS shell semantics remain Phase-11+/post-prephase depth, not current-slice blockers |
| Phase 11: Interepo translation/coexistence | Architectural Research Complete | Architectural baseline, runtime partial | [x] Architecture and edge-case notes captured in SPEC/phase tracker; [ ] foreign adapters; [ ] translated snapshot install path; [ ] coexist/warn/lock modes |
| Phase 12: Migration/adoption/pkgit retirement | Started | Started | [x] bounded DB-backed `mg from` and `adopt` import current foreign installed-state metadata without file takeover; [ ] live takeover/coexistence lock modes; [ ] final pkgit cutover gates |

---

## Feature Domain Checklist

### 1) Core Install/State Integrity
- [x] Current slice — Installed-state DB and ownership manifest recording exist
- [x] Current slice — Mutation locking and transaction journal exist
- [x] Current slice — `verify`, `reverify`, and `recover` are real
- [x] Current slice — Prefix rollback and downgrade are real
- [x] Current slice — Backend parity between prefix and the current `/usr` backend is real for activation/archive/rollback/trigger behavior
- [x] Current slice — The current `/usr` backend now has staged-state activation plus archive/rollback/trigger behavior
- [x] Current slice — The current `/usr` backend now records configured activation-snapshot requests, performs staged file-switch activation, and persists backend/boot status for operator reporting
- [~] Partial — Broader system-management layers on top of the current backend are still Phase 8+ work

### 2) Package Definition + Build Runtime
- [x] Current slice — `pkg.lua` parse/validate path exists
- [x] Current slice — Single-lane and dual source/binary lane definitions supported
- [x] Current slice — Build-system floor executed for current slice (`cargo/cmake/go/make/meson/nimble/python/zig`)
- [x] Current slice — GitHub release arch-specific asset tables supported
- [x] Current slice — Synced source-capable remotes can build from the pinned `packages/<pkgname>/` tree instead of snapshot text alone
- [x] Current slice — Typed `provider_assets` metadata is parsed, validated, and collected into persisted system metadata
- [~] Partial — Runtime support lags declared source schema breadth
- [x] Current slice — Implement bounded `nix_flake`, `gentoo_overlay`, `aur_pkgbuild`, and `xbps_template` execution paths

### 3) Sync/Repo/Trust/Cache
- [x] Current slice — `sync` against signed indexes + local verified snapshots
- [x] Current slice — TOFU and pinned trust modes, plus explicit rotated-key acceptance
- [x] Current slice — Secure remote payload verification path exists
- [x] Current slice — Cache priority routing + local retention cleanup exist
- [x] Current slice — Synced remotes may declare `packages_url` for pinned source-lane package-definition fetches
- [~] Partial — Full repo/cache/trust depth from spec not complete yet
- [ ] Not started — Complete supply-chain/audit surfaces and remaining trust features

### 4) Resolver/Planning/Policy
- [x] Current slice — Closure-aware install/upgrade planning exists
- [x] Current slice — Versioned dependency and explicit versioned `provides` checks exist
- [x] Current slice — `pin`/`hold` policy and upgrade gating exist
- [x] Current slice — Weak dep policy (`recommends` default install + refresh policy) exists
- [x] Current slice — PubGrub-style solver and policy configuration model landed

### 5) Operator Commands + UX
- [x] Current slice — Core command set is materially useful (`i/rm/u/sync/search/info/files/verify/recover/rollback/pf/...`)
- [x] Current slice — Human-readable output paths improved
- [x] Current slice — Native forge hosting documentation (`eldaforgehosting/`) covers source-only and binary remotes, cache population, per-platform guides (GitHub/GitLab/Gitea-Forgejo/SourceHut/static HTTP), hosting patterns (AUR-style, binhost, full forge, LAN mirror, staging/stable), recipe-to-Git workflows, dynamic interemotes, and current `rmt` management commands
- [~] Partial — Install/migration/takeover UX contract is now documented more fully, and the runtime now has structured human install rendering with `key:: value` frames, persistent per-run session logs under invoking operator home, activation-backend plus snapshot summaries in install output, grouped per-package progress with semantic color, live build stdio passthrough (cargo/cmake/meson/make/ctest), deduplicated review gates (single footer prompt), unchanged interbuild review fast path, bulk snapshot review for Void/Gentoo imports, direct missing-remote bootstrap guidance, generated-metadata review gates, ranked source-option reporting/selection, git tag/release inspection, and ad hoc git ref switching; remaining gaps: upgrade/rm/sync framed transaction plans, bootstrap privilege deferral, and MiB preflight polish
- [x] Current slice — Branded help and command descriptions exist
- [~] Partial — Unsupported command paths now fail closed with structured blocked reports; `ext ls`, `adopt`, and `mg from` are runtime-backed, while any remaining release-scope commands still need real handlers
- [~] Partial — Replace any remaining fail-closed unsupported command paths with real handlers when they are in release scope

### 6) Profiles/Daemon/System Shape
- [x] Current slice — Profile read/apply/edit current slice exists (`pf show/apply/add/rm/set-init/clear-init/set-arch/add-foreign-arch/remove-foreign-arch`)
- [x] Current slice — Profile recipes can declare typed machine-shape defaults
- [x] Current slice — Desired state export/import exists
- [x] Current slice — Desired state export/import now really round-trips machine shape by replaying imported active profile anchors
- [x] Current slice — Disposable-root `/usr` mode now reconciles active provider assets and can reapply them through `fix-triggers`
- [x] Current slice — Disposable-root `/usr` mode now persists applied init-provider backend state and uses it for immediate supported `pf set-init` / `pf apply` / `state import` reconciliation
- [x] Current slice — `daemon run` now executes a real foreground refresh pass for the current slice instead of falling through to a stub
- [~] Partial — Typed pending system-change reporting exists, but full daemon/system-change behavior is still pending
- [ ] Not started — Full system change handler lifecycle and trigger repair flow on live backend

### 7) Replacement Readiness
- [x] Current slice — Many pkgit-equivalent workflows are already real in prefix mode
- [~] Partial — "Replacement ready" still blocked by the remaining workflow-coverage gaps called out in `stage.md`
- [ ] Not started — Satisfy all replacement gates from `stage.md`
- [ ] Not started — After replacement gate, satisfy full spec-complete fork gates (Phases 9-12)

---

## Open Decisions / Design Blockers

- [x] Current slice — `D-01` Linux activation materialization strategy finalized as staged tree + explicit current-state metadata
- [ ] Not started — `D-02` First isolated build backend implementation strategy `(!)`
- [x] Current slice — `D-03` Interepo adapter order landing sequence frozen: ALPM first (Arch/Artix), then APK (Chimera/Alpine), then AUR, then Portage, then nixpkgs (second-wave with `arwen-elf`)
- [x] Current slice — `D-04` Native index publish layout: recipe **monorepo** + **one signed index per channel** (multi-`arch` rows in same index); production URL rewrite only in `publish finalize`; upload target per **`/etc/elda/host.d/`** profile — documented in [eldaforgehosting/host-maintainer-tools.md](./eldaforgehosting/host-maintainer-tools.md)
- [x] Current slice — `D-05` Canonical package-definition contract for provider-specific assets frozen in `SPEC.md`

---

## High-Risk Gaps (Audit-Derived)

- [~] Partial — `ELDA-01` CLI surface still exceeds backend coverage in places
- [~] Partial — `ELDA-02` Repo trust/cache slice is materially improved but still partial vs full spec
- [x] Current slice — `ELDA-03` Resolved for the current pre-Phase-11+ slice: bounded parser-backed families, source-option selection, ad hoc git ref selectors, provider-aware release assets, read-only multi-provider release inspection, release signature sidecar metadata validation, GPKG fast path, config queue resolution, and inspection UX are runtime-backed; broader full-foreign semantics move to Phase-11+/post-prephase work
- [x] Current slice — `ELDA-04` Linux `/usr` backend + trigger engine now perform staged file-switch activation, archive/rollback parity, and backend/boot status reporting
- [x] Current slice — `ELDA-05` Archived rollback now covers the current prefix and `/usr` backends
- [~] Partial — `ELDA-06` Placeholder crates still dominate some boundaries (`elda-fetch`, `elda-ext`, `elda-unix`, `xtask`; `elda-git` is now partial); `elda-linux` is now partial rather than boundary-only

---

## Current Work Queue (Editable "Now" Lane)

Use this as the immediate sprint board.

- [x] Current slice — Previous focus completed: persist applied init-provider backend state for disposable-root `/usr` mode and stop reporting it as permanently deferred after asset reconciliation succeeds
- [x] Current slice — Previous focus completed: land the full PubGrub-style solver and config-backed provider policy
- [x] Current slice — Current active focus completed: Phase 10/source UX and pre-Phase-11+ market surfaces are closed for this milestone
- [x] Current slice — Break current Phase 10/UX focus into config/reporting, release-asset compatibility, and later discovery/TUI slices
- [x] Current slice — Completed: ad hoc source-option UX, release-source compatibility, remote/interemote UX, config queue resolution, inspection surfaces, public README/USAGE/forge docs, fixtures/examples config refresh, and `man/elda.1`
- [x] Current slice — Current docs/examples/manpage refresh recorded; remaining active tasks below are next-slice candidates, not part of this docs pass

Suggested immediate candidates (based on current blockers):
- [x] Current slice — Unsupported runtime command paths fail closed; `elda doctor` exposes bootstrap/release-readiness state
- [ ] Not started — Push broader daemon/system-management behavior on top of the now-real `/usr` backend
- [x] Current slice — Implement first executable `nix_flake` bounded path
- [x] Current slice — Implement first executable `gentoo_overlay` bounded path
- [x] Current slice — Implement first executable `aur_pkgbuild` bounded path
- [x] Current slice — Implement first executable `xbps_template` bounded path
- [x] Current slice — Honor configured add/link strategy priority for generated metadata
- [x] Current slice — Add opt-in list-options source-option reporting for ad hoc/local-source metadata resolution
- [x] Current slice — Add first bounded provider-aware `release_asset` recipe/build support for GitHub/GitLab/Gitea/Forgejo/SourceHut/direct manifests
- [x] Current slice — Add read-only `elda git tags` / `elda versions` inspection with normalized version confidence
- [x] Current slice — Add read-only GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest release discovery/classification command
- [x] Current slice — Add raw-link GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest release option reporting ahead of source fallback
- [x] Current slice — Add explicit `--strategy <name>` metadata strategy override for add/install links
- [x] Current slice — Add explicit ad hoc git ref selectors `--to-branch`, `--to-tag`, and `--to-rev` for generated metadata
- [x] Current slice — Render pinned ad hoc git tag/revision plain-upgrade no-ops as explicit keep-installed rows
- [x] Current slice — Add checksum-backed release-to-metadata conversion for selected GitHub/GitLab/Gitea/Forgejo/SourceHut/direct-manifest release assets with API digests or checksum sidecars
- [x] Current slice — Add SourceHut tag-artifact and direct `.elda-releases.json` release inspection
- [x] Current slice — Extend `release_asset` validation/build staging to SourceHut, direct manifests, and self-hosted forge `host` metadata
- [x] Current slice — Record matching signature sidecars in generated release metadata
- [x] Current slice — Add TTY-gated interactive selector for `metadata.link_option_mode = "list-options"`
- [x] Current slice — Add parser-specific metadata/dependency context to the interbuild review gate
- [x] Current slice — Add ad hoc git source-ref downgrade through `downgrade --to-tag/--to-rev`
- [x] Current slice — Add interactive selection for list-options mode before generated metadata review; explicit `--source-option <N>` still works for noninteractive flows
- [x] Current slice — Surface bounded Gentoo/AUR/XBPS phase command lines in interbuild reports
- [x] Current slice — Surface bounded AUR VCS source and `pkgver()` metadata in reports
- [x] Current slice — Parse AUR/XBPS quoted metadata words without splitting inside descriptions
- [x] Current slice — Validate release signature fields for empty/traversal values without claiming trust enforcement
- [x] Current slice — Fail closed on mismatched AUR source/checksum, AUR arch-specific source/checksum, and XBPS distfile/checksum counts

---

## Definition of Done for This Tracker

- [ ] Not started — Every merged feature updates this file in the same PR
- [ ] Not started — "In Progress" entries must have at least one concrete next task listed
- [ ] Not started — "Done" entries must be code-backed and test-backed
- [ ] Not started — Replacement-ready claim only flips after all `stage.md` gates are met
- [ ] Not started — Full-fork-complete claim only flips after Phase 12 done
