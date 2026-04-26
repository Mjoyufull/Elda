# Elda Master Checklist (Kanban Tracker)

This is the single development tracker for what is done, what is in progress, and what is still missing in Elda.

Status source used for this tracker:
- `SPEC.md` (behavior contract)
- `pkgitfork.md` (fork direction and architecture context)
- `phase.md` (implementation ledger)
- `CODEBASE_AUDIT.md` (runtime reality and issue list)
- `stage.md` (pkgit-replacement gate)
- `idk.md` (resolved/remaining contract gaps)

---

## Legend

- `[x]` done
- `[~]` in progress / partially shipped
- `[ ]` not started
- `(!)` blocker or high-risk gap

---

## Global Kanban Board

| Done | In Progress | Todo |
|---|---|---|
| [x] Workspace/crate baseline exists (`elda-cli`, `elda-core`, `elda-db`, etc.) | [~] Repo/trust/cache slice is real but not full spec-complete supply-chain model | [ ] Broader daemon/system-management lifecycle on top of the current backend |
| [x] SQLite installed-state DB + ownership model + manifests | [~] Build/runtime source-kind surface still narrower than declared model (`nix_flake`, `gentoo_overlay` gap) | [ ] Executable support for declared interbuild source kinds (`nix_flake`, `gentoo_overlay`) |
| [x] Recipe parse/check flow for `pkg.lua`, with legacy import path | [~] Some command namespaces still not fully backended (`mg`, `adopt`, `ext`) | [ ] Remove remaining `handle_stub()` surfaces and land real backend implementations |
| [x] Source/binary lane model (`i`, `ig`, `ib`, `--prefer-*`) | [~] Prefix rollback is real and the current `/usr` backend now performs staged switch activation with archived-state rollback parity | [ ] Broader daemon/system-management layers on top of the current backend |
| [x] Direct git installs + multi build-system execution (`cargo/cmake/meson/make/go/zig/python/nimble`) | [~] Profile/machine-shape slice is complete for the current target roots, but broader daemon/system-management integration remains | [ ] Full typed trigger engine + provider-family/system-change handlers on live backend |
| [x] Vendor workflow (`vendor add/import/export`) | [~] Install/bootstrap UX now has real review-gate, grouped progress, logging, and better missing-remote guidance, but broader first-run/setup flow remains | [ ] Guided `setup` / `doctor` bootstrap flow and broader install/takeover UX |
| [x] Sync/search/info from verified merged snapshots | [~] Interepo/migration architecture documented with full store-path normalization pipeline for nixpkgs | [ ] Interepo adapters + translation confidence flow + coexistence modes + full nixpkgs normalization (ELF + shebangs + wrappers + text + symlinks) |
| [x] Trust model baseline: signed index, TOFU/pinned, explicit rotated-key acceptance, offline verified snapshots | [~] Replacement claim is still blocked by remaining pkgit-workflow coverage gaps from `stage.md` | [ ] Reach "pkgit replacement ready" gate and then full spec-complete gate |
| [x] Policy + introspection (`pin`, `hold`, `why`, `rdeps`, `autoremove`, `diff`, `downgrade`) | [~] File-size/code-standards cleanup mostly done, a few files still over soft limit | [ ] Finish standards cleanup and keep large files split sustainably |
| [x] Prefix-safe transaction + verify/recover + conffile behavior (`.eldanew`/`.eldasave`) | [~] The current `/usr` backend is real; broader system-management and later-phase Linux work remain | [ ] Finish broader daemon/system-management behavior on top of the backend |
| [x] Full PubGrub-style dependency solver with provider-policy/config control | [~] The current `/usr` backend is real; broader system-management and later-phase Linux work remain | [ ] Finish broader daemon/system-management behavior on top of the backend |

---

## Phase Tracker (from `phase.md`)

| Phase | Status | Checklist |
|---|---|---|
| Phase 0: Fork baseline + skeleton | Done | [x] Workspace landed; [x] canonical CLI surface wired; [x] pkgit fixtures captured |
| Phase 1: Core types/config/state | Done | [x] identity/version parsing; [x] config defaults; [x] SQLite bootstrap + layout + lock |
| Phase 2: Recipe model + legacy import | Done | [x] `rc add`; [x] `rc check`; [x] parser/validation for declarative metadata families, including `provider_assets` |
| Phase 3: Remotes/index/trust/cache | Completed | [x] signed snapshot sync; [x] TOFU/pinned trust; [x] metadata rotation acceptance; [x] source-capable synced remotes can declare `packages_url` for pinned package-definition fetches; [x] cache priority + retention cleanup landed; [~] broader trust/cache completeness remains later hardening, not a Phase 3 blocker |
| Phase 4: Resolver/flags/planning | Completed | [x] PubGrub-style install/upgrade solver; [x] conflicts/replaces/pin/hold checks; [x] config-backed provider preferences through `[resolver.provider_preferences]`; [x] synced source-lane resolution can require remote `packages_url` when the selected lane needs a real package tree |
| Phase 5: Build/staging/payloads | Completed | [x] stage + manifest + payload; [x] git/archive/github_release paths; [x] synced source builds materialize pinned `packages/<pkgname>/` trees from `packages_url` + `repo_commit`; [x] build metadata now captures typed `provider_assets`; [~] source-kind parity is a later source-model/runtime gap, not a Phase 5 blocker |
| Phase 6: Prefix transaction backend | Completed | [x] install/remove/upgrade journals + verify/recover; [x] rollback + downgrade in prefix; [x] prefix transaction parity for the documented slice landed |
| Phase 7: Linux system backend + triggers | Completed | [x] staged `/usr` backend; [x] live file-switch activation; [x] internal trigger engine + `check`/`fix-triggers`; [x] archive/rollback parity on the current `/usr` backend; [x] provider-asset storage/materialization/reconciliation on the current backend; [x] applied init-provider backend state persists under `state/system-backend/` and drives current provider-asset materialization; [x] configured `snapshot_tool` requests now record pre/post activation attempts into journals, reports, and archived state metadata for system install/remove transactions; [x] activation-backend capability reporting plus persisted boot status landed |
| Phase 8: Profiles/machine shape/ops | Completed | [x] `pf show/apply/add/rm/set-init/clear-init/set-arch/add-foreign-arch/remove-foreign-arch`; [x] state export/import; [x] profile recipes can declare `pkg.profile` machine-shape defaults; [x] imported desired state now really reapplies active profile anchors and round-trips machine shape; [~] broader daemon/system-management integration remains later work |
| Phase 9: Native CI + binary publishing | Completed | [x] local CI submission pipeline; [x] local DAG/layer generation; [x] local artifact/signature/index publish path; [x] compressed `lock-v1.json.zst` output plus per-payload `.minisig` / `.spdx.json` / `.attestation.json` sidecars; [x] indexed `sbom_url` / `attestation_url` metadata; [x] `forge search/browse`, `qa *`, and `daemon run` current-slice handlers; [x] `ci pr` now derives compare/PR-style URLs from recognizable forge remotes with configured base-branch awareness; [x] git-remote submission publication landed for configurable submission remote/base-branch targets in PR and trusted push modes; [x] hosted review creation landed for token-auth GitHub/GitLab/Gitea-style `ci pr`; [x] per-remote submission override resolution now follows the submission's actual remote; [x] local scheduler/orchestration landed through bare `ci run`, retry-state tracking, and richer queue/log visibility |
| Phase 10: Git-mode interbuilds | Not Started | [ ] `nix_flake` bounded execution; [ ] `gentoo_overlay` bounded execution |
| Phase 11: Interepo translation/coexistence | Architectural Research Complete | [x] Architecture fully documented in `phase11.md`; [x] 16-step implementation order frozen; [x] nixpkgs full store-path normalization pipeline (ELF via `arwen-elf` + byte-level hash scanning for shebangs/wrappers/text + symlink rewriting + cleanup); [x] per-family edge cases documented (ALPM scriptlets, APK triggers, Portage USE/SLOT, Nix wrappers, takeover/migration state carry-over, user-scope generated state, and per-PM admin-state quirks); [x] PM replacement validation stories (CachyOS/Arch, Gentoo, NixOS, mixed-source); [x] all deep integration anomalies resolved in `idk.md`; [ ] foreign adapters; [ ] translated snapshot install path; [ ] coexist/warn/lock modes |
| Phase 12: Migration/adoption/pkgit retirement | Not Started | [ ] `mg from` adapters; [ ] `adopt`; [ ] final pkgit cutover gates |

---

## Feature Domain Checklist

### 1) Core Install/State Integrity
- [x] Installed-state DB and ownership manifest recording exist
- [x] Mutation locking and transaction journal exist
- [x] `verify`, `reverify`, and `recover` are real
- [x] Prefix rollback and downgrade are real
- [x] Backend parity between prefix and the current `/usr` backend is real for activation/archive/rollback/trigger behavior
- [x] The current `/usr` backend now has staged-state activation plus archive/rollback/trigger behavior
- [x] The current `/usr` backend now records configured activation-snapshot requests, performs staged file-switch activation, and persists backend/boot status for operator reporting
- [~] Broader system-management layers on top of the current backend are still Phase 8+ work

### 2) Package Definition + Build Runtime
- [x] `pkg.lua` parse/validate path exists
- [x] Single-lane and dual source/binary lane definitions supported
- [x] Build-system floor executed for current slice (`cargo/cmake/go/make/meson/nimble/python/zig`)
- [x] GitHub release arch-specific asset tables supported
- [x] Synced source-capable remotes can build from the pinned `packages/<pkgname>/` tree instead of snapshot text alone
- [x] Typed `provider_assets` metadata is parsed, validated, and collected into persisted system metadata
- [~] Runtime support lags declared source schema breadth
- [ ] Implement `nix_flake` and `gentoo_overlay` execution path

### 3) Sync/Repo/Trust/Cache
- [x] `sync` against signed indexes + local verified snapshots
- [x] TOFU and pinned trust modes, plus explicit rotated-key acceptance
- [x] Secure remote payload verification path exists
- [x] Cache priority routing + local retention cleanup exist
- [x] Synced remotes may declare `packages_url` for pinned source-lane package-definition fetches
- [~] Full repo/cache/trust depth from spec not complete yet
- [ ] Complete supply-chain/audit surfaces and remaining trust features

### 4) Resolver/Planning/Policy
- [x] Closure-aware install/upgrade planning exists
- [x] Versioned dependency and explicit versioned `provides` checks exist
- [x] `pin`/`hold` policy and upgrade gating exist
- [x] Weak dep policy (`recommends` default install + refresh policy) exists
- [x] PubGrub-style solver and policy configuration model landed

### 5) Operator Commands + UX
- [x] Core command set is materially useful (`i/rm/u/sync/search/info/files/verify/recover/rollback/pf/...`)
- [x] Human-readable output paths improved
- [x] Native forge hosting documentation now covers source-only and full binary remote setups, cache population, and GitHub/GitLab/Gitea operator patterns
- [~] Install/migration/takeover UX contract is now documented more fully, and the runtime now has structured human install rendering, persistent per-run session logs for mutating commands, activation-backend plus snapshot summaries in install output, a grouped per-package progress surface for source/binary/install steps, direct missing-remote bootstrap guidance, and the first interactive generated-metadata review gate for session-generated recipes (`eldainstallaztionuxandcliimprovements.md`), but the broader first-run/setup/takeover flow is still partial overall
- [x] Branded help and command descriptions exist
- [~] Several namespaces still mostly stubs (`mg/adopt/ext`)
- [ ] Complete all remaining stubbed namespaces

### 6) Profiles/Daemon/System Shape
- [x] Profile read/apply/edit current slice exists (`pf show/apply/add/rm/set-init/clear-init/set-arch/add-foreign-arch/remove-foreign-arch`)
- [x] Profile recipes can declare typed machine-shape defaults
- [x] Desired state export/import exists
- [x] Desired state export/import now really round-trips machine shape by replaying imported active profile anchors
- [x] Disposable-root `/usr` mode now reconciles active provider assets and can reapply them through `fix-triggers`
- [x] Disposable-root `/usr` mode now persists applied init-provider backend state and uses it for immediate supported `pf set-init` / `pf apply` / `state import` reconciliation
- [x] `daemon run` now executes a real foreground refresh pass for the current slice instead of falling through to a stub
- [~] Typed pending system-change reporting exists, but full daemon/system-change behavior is still pending
- [ ] Full system change handler lifecycle and trigger repair flow on live backend

### 7) Replacement Readiness
- [x] Many pkgit-equivalent workflows are already real in prefix mode
- [~] "Replacement ready" still blocked by the remaining workflow-coverage gaps called out in `stage.md`
- [ ] Satisfy all replacement gates from `stage.md`
- [ ] After replacement gate, satisfy full spec-complete fork gates (Phases 9-12)

---

## Open Decisions / Design Blockers

- [x] `D-01` Linux activation materialization strategy finalized as staged tree + explicit current-state metadata
- [ ] `D-02` First isolated build backend implementation strategy `(!)`
- [x] `D-03` Interepo adapter order landing sequence frozen: ALPM first (Arch/Artix), then APK (Chimera/Alpine), then AUR, then Portage, then nixpkgs (second-wave with `arwen-elf`)
- [ ] `D-04` Native index publish layout decision (`yoka-ci/index` vs generated branch/artifact)
- [x] `D-05` Canonical package-definition contract for provider-specific assets frozen in `SPEC.md` / `pkgitfork.md`

---

## High-Risk Gaps (Audit-Derived)

- [~] `ELDA-01` CLI surface still exceeds backend coverage in places
- [~] `ELDA-02` Repo trust/cache slice is materially improved but still partial vs full spec
- [ ] `ELDA-03` Declared source model wider than executable runtime (`nix_flake`, `gentoo_overlay`) `(!)`
- [x] `ELDA-04` Linux `/usr` backend + trigger engine now perform staged file-switch activation, archive/rollback parity, and backend/boot status reporting
- [x] `ELDA-05` Archived rollback now covers the current prefix and `/usr` backends
- [~] `ELDA-06` Placeholder crates still dominate some boundaries (`elda-fetch`, `elda-git`, `elda-ext`, `elda-unix`, `xtask`); `elda-linux` is now partial rather than boundary-only

---

## Current Work Queue (Editable "Now" Lane)

Use this as the immediate sprint board.

- [x] Previous focus completed: persist applied init-provider backend state for disposable-root `/usr` mode and stop reporting it as permanently deferred after asset reconciliation succeeds
- [x] Previous focus completed: land the full PubGrub-style solver and config-backed provider policy
- [ ] Next active focus: finish one remaining stub namespace end-to-end and keep broadening daemon/system-management behavior on top of the current backend
- [ ] Break next focus into 3-7 concrete PR-sized tasks
- [ ] Mark exactly one next task as "currently being worked on"
- [ ] Keep all other active tasks queued in this section

Suggested immediate candidates (based on current blockers):
- [ ] Replace one remaining stub namespace end-to-end (`mg`, `adopt`, or `ext`)
- [ ] Push broader daemon/system-management behavior on top of the now-real `/usr` backend
- [ ] Push broader daemon/system-management behavior on top of the now-real `/usr` backend
- [ ] Implement first executable `nix_flake` bounded path
- [ ] Implement first executable `gentoo_overlay` bounded path

---

## Definition of Done for This Tracker

- [ ] Every merged feature updates this file in the same PR
- [ ] "In Progress" entries must have at least one concrete next task listed
- [ ] "Done" entries must be code-backed and test-backed
- [ ] Replacement-ready claim only flips after all `stage.md` gates are met
- [ ] Full-fork-complete claim only flips after Phase 12 done
