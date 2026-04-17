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
| [x] Workspace/crate baseline exists (`elda-cli`, `elda-core`, `elda-db`, etc.) | [~] Resolver is closure-aware but still not full PubGrub-grade solver behavior | [ ] Full PubGrub-style dependency solver with full provider-policy/config control |
| [x] SQLite installed-state DB + ownership model + manifests | [~] Repo/trust/cache slice is real but not full spec-complete supply-chain model | [ ] Full Linux `/usr` activation backend and full system transaction lifecycle |
| [x] Recipe parse/check flow for `pkg.lua`, with legacy import path | [~] Build/runtime source-kind surface still narrower than declared model (`nix_flake`, `gentoo_overlay` gap) | [ ] Executable support for declared interbuild source kinds (`nix_flake`, `gentoo_overlay`) |
| [x] Source/binary lane model (`i`, `ig`, `ib`, `--prefer-*`) | [~] Some command namespaces still not fully backended (`ci`, `forge`, `mg`, `adopt`, `ext`, `qa`, `daemon run`) | [ ] Remove remaining `handle_stub()` surfaces and land real backend implementations |
| [x] Direct git installs + multi build-system execution (`cargo/cmake/meson/make/go/zig/python/nimble`) | [~] Prefix rollback is real; system backend rollback story is still pending | [ ] System backend archived-state + rollback parity |
| [x] Vendor workflow (`vendor add/import/export`) | [~] Profile/daemon area has current slice support but broader system-change/trigger integration remains | [ ] Full typed trigger engine + provider-family/system-change handlers on live backend |
| [x] Sync/search/info from verified merged snapshots | [~] CI/forge publication pipeline is not started | [ ] Native CI DAG/layer build + publish pipeline + `ci` namespace completion |
| [x] Trust model baseline: signed index, TOFU/pinned, explicit rotated-key acceptance, offline verified snapshots | [~] Interepo/migration architecture is documented but implementation not landed | [ ] Interepo adapters + translation confidence flow + coexistence modes |
| [x] Policy + introspection (`pin`, `hold`, `why`, `rdeps`, `autoremove`, `diff`, `downgrade`) | [~] Replacement claim is still blocked by remaining backend/system gaps | [ ] Reach "pkgit replacement ready" gate and then full spec-complete gate |
| [x] Prefix-safe transaction + verify/recover + conffile behavior (`.eldanew`/`.eldasave`) | [~] File-size/code-standards cleanup mostly done, a few files still over soft limit | [ ] Finish standards cleanup and keep large files split sustainably |

---

## Phase Tracker (from `phase.md`)

| Phase | Status | Checklist |
|---|---|---|
| Phase 0: Fork baseline + skeleton | Done | [x] Workspace landed; [x] canonical CLI surface wired; [x] pkgit fixtures captured |
| Phase 1: Core types/config/state | Done | [x] identity/version parsing; [x] config defaults; [x] SQLite bootstrap + layout + lock |
| Phase 2: Recipe model + legacy import | Done | [x] `rc add`; [x] `rc check`; [x] parser/validation for declarative metadata families |
| Phase 3: Remotes/index/trust/cache | In Progress | [x] signed snapshot sync; [x] TOFU/pinned trust; [x] metadata rotation acceptance; [~] broader trust/cache completeness still pending |
| Phase 4: Resolver/flags/planning | In Progress | [x] closure-aware planning; [x] conflicts/replaces/pin/hold checks; [~] full PubGrub + broader provider policy still pending |
| Phase 5: Build/staging/payloads | In Progress | [x] stage + manifest + payload; [x] git/archive/github_release paths; [~] source-kind parity not complete |
| Phase 6: Prefix transaction backend | In Progress | [x] install/remove/upgrade journals + verify/recover; [x] rollback + downgrade in prefix; [~] system backend parity pending |
| Phase 7: Linux system backend + triggers | Not Started | [ ] live `/usr` backend; [ ] trigger engine; [ ] system change handlers; [ ] backend archive/rollback parity |
| Phase 8: Profiles/machine shape/ops | In Progress | [x] `pf show/apply/set-init`; [x] state export/import; [~] broader daemon/system-management integration pending |
| Phase 9: Native CI + binary publishing | Not Started | [ ] CI submission pipeline; [ ] DAG/layer generation; [ ] artifact/signature/index publish path |
| Phase 10: Git-mode interbuilds | Not Started | [ ] `nix_flake` bounded execution; [ ] `gentoo_overlay` bounded execution |
| Phase 11: Interepo translation/coexistence | Not Started | [ ] foreign adapters; [ ] translated snapshot install path; [ ] coexist/warn/lock modes |
| Phase 12: Migration/adoption/pkgit retirement | Not Started | [ ] `mg from` adapters; [ ] `adopt`; [ ] final pkgit cutover gates |

---

## Feature Domain Checklist

### 1) Core Install/State Integrity
- [x] Installed-state DB and ownership manifest recording exist
- [x] Mutation locking and transaction journal exist
- [x] `verify`, `reverify`, and `recover` are real
- [x] Prefix rollback and downgrade are real
- [~] Backend parity between prefix and system-mode still incomplete
- [ ] Full live system backend parity with archive/rollback/trigger behavior

### 2) Package Definition + Build Runtime
- [x] `pkg.lua` parse/validate path exists
- [x] Single-lane and dual source/binary lane definitions supported
- [x] Build-system floor executed for current slice (`cargo/cmake/go/make/meson/nimble/python/zig`)
- [x] GitHub release arch-specific asset tables supported
- [~] Runtime support lags declared source schema breadth
- [ ] Implement `nix_flake` and `gentoo_overlay` execution path

### 3) Sync/Repo/Trust/Cache
- [x] `sync` against signed indexes + local verified snapshots
- [x] TOFU and pinned trust modes, plus explicit rotated-key acceptance
- [x] Secure remote payload verification path exists
- [x] Cache priority routing + local retention cleanup exist
- [~] Full repo/cache/trust depth from spec not complete yet
- [ ] Complete supply-chain/audit surfaces and remaining trust features

### 4) Resolver/Planning/Policy
- [x] Closure-aware install/upgrade planning exists
- [x] Versioned dependency and explicit versioned `provides` checks exist
- [x] `pin`/`hold` policy and upgrade gating exist
- [x] Weak dep policy (`recommends` default install + refresh policy) exists
- [~] Not yet a complete full-search PubGrub/provider-policy engine
- [ ] Land full solver and policy configuration model

### 5) Operator Commands + UX
- [x] Core command set is materially useful (`i/rm/u/sync/search/info/files/verify/recover/rollback/...`)
- [x] Human-readable output paths improved
- [x] Branded help and command descriptions exist
- [~] Several namespaces still mostly stubs (`ci/forge/mg/adopt/ext/qa`)
- [ ] Complete all remaining stubbed namespaces

### 6) Profiles/Daemon/System Shape
- [x] Profile read/apply/set-init current slice exists
- [x] Desired state export/import exists
- [~] Provider-asset reconciliation and full daemon/system-change behavior pending
- [ ] Full system change handler lifecycle and trigger repair flow on live backend

### 7) Replacement Readiness
- [x] Many pkgit-equivalent workflows are already real in prefix mode
- [~] "Replacement ready" still blocked by backend/solver/system completion
- [ ] Satisfy all replacement gates from `stage.md`
- [ ] After replacement gate, satisfy full spec-complete fork gates (Phases 9-12)

---

## Open Decisions / Design Blockers

- [ ] `D-01` Linux activation materialization strategy finalization (staged tree vs alternatives) `(!)`
- [ ] `D-02` First isolated build backend implementation strategy `(!)`
- [ ] `D-03` Interepo adapter order landing sequence (ALPM/APK/Portage)
- [ ] `D-04` Native index publish layout decision (`yoka-ci/index` vs generated branch/artifact)

---

## High-Risk Gaps (Audit-Derived)

- [~] `ELDA-01` CLI surface still exceeds backend coverage in places
- [~] `ELDA-02` Repo trust/cache slice is materially improved but still partial vs full spec
- [ ] `ELDA-03` Declared source model wider than executable runtime (`nix_flake`, `gentoo_overlay`) `(!)`
- [ ] `ELDA-04` Linux `/usr` backend + trigger engine missing `(!)`
- [ ] `ELDA-05` Archived rollback story still prefix-focused
- [ ] `ELDA-06` Placeholder crates still boundary-only (`elda-fetch`, `elda-git`, `elda-ext`, `elda-linux`, `elda-unix`, `xtask`)

---

## Current Work Queue (Editable "Now" Lane)

Use this as the immediate sprint board.

- [ ] Choose one active focus area for this session
- [ ] Break active focus into 3-7 concrete PR-sized tasks
- [ ] Mark exactly one task as "currently being worked on"
- [ ] Keep all other active tasks queued in this section
- [ ] Update status at end of each work block

Suggested immediate candidates (based on current blockers):
- [ ] Finish remaining resolver/provider-policy behavior
- [ ] Start Linux `/usr` backend scaffolding in `elda-linux` + `elda-unix`
- [ ] Replace one stub namespace end-to-end (e.g., one `ci` command path)
- [ ] Implement first executable `nix_flake` bounded path
- [ ] Implement first executable `gentoo_overlay` bounded path

---

## Definition of Done for This Tracker

- [ ] Every merged feature updates this file in the same PR
- [ ] "In Progress" entries must have at least one concrete next task listed
- [ ] "Done" entries must be code-backed and test-backed
- [ ] Replacement-ready claim only flips after all `stage.md` gates are met
- [ ] Full-fork-complete claim only flips after Phase 12 done

