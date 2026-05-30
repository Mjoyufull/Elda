# Elda Usage Guide

This guide explains how to run a machine day to day with Elda as the package manager: what
Elda records, how installs actually land on disk, and which commands to use in which order.
Command names are listed where they help, but the goal is understanding - not a cheat sheet.

Exact behavior contracts live in [SPEC.md](./SPEC.md). Native hosting, indexes, and publish
pipelines are in [eldaforgehosting/](./eldaforgehosting/README.md). Worked recipes and config
fragments are under [examples/](./examples/).

> [!WARNING]
> **Illustrative examples:** URLs, remotes, signing keys, and third-party repo names in this
> file are placeholders unless they are infrastructure you operate.

---

## How Elda models your machine

Elda is one state engine for several jobs that other ecosystems split apart: synced remotes,
local recipes, direct git installs, vendor binaries, interbuild foreign metadata, and (in
progress) migration from another package manager.

**Authoritative state** lives in SQLite under `/var/lib/elda/db/elda.sqlite`, not in scattered
marker files. For each installed package Elda stores identity, version, how it was installed,
dependency edges, manifest hashes, and which activation backend owns the files.

**World** (`/var/lib/elda/db/world`) is the set of packages you intend to keep: everything
installed with reason `base` or `explicit`. Dependency-only packages (`dep`) are derived from
the graph; they are not world anchors. `elda autoremove` removes orphans - packages nothing in
world or active profiles still requires.

**Transactions** wrap mutations. Each install/remove/upgrade writes a journal under
`/var/lib/elda/db/journal/`. If a run is interrupted, `elda recover` finishes or rolls back
using that journal before new mutations are allowed.

**Staging then activation:** Elda does not blindly copy into `/usr`. It builds or fetches a
payload, stages under a transaction root, verifies, then activates through a backend:

| Backend | Typical root | What it means for you |
| --- | --- | --- |
| Prefix | `defaults.prefix` (e.g. `~/.local` or a test root) | Safe experimentation; no boot integration |
| System | Live host `/usr` when system mode is allowed | Real machine changes; needs privilege escalation |

System mode composes the next tree under `/var/lib/elda/states/<state-id>/` and switches files
into the live root. Prefix mode keeps everything inside the configured prefix.

**One package identity, two lanes:** Maintained packages can ship both a **source** lane (build
from `build.lua` / git) and a **binary** lane (prebuilt payload from an index). `elda i` picks
using policy; `elda ig` / `elda ib` force a lane. Ad hoc git URLs always go through source
metadata generation and review before build.

---

## Installation modes: where Elda is allowed to write

### Disposable prefix (default for learning)

Set in `config.toml`:

```toml
[defaults]
prefix = "/home/you/.local/elda-root"
allow_system_mode = false
```

All installs, rollbacks, and triggers stay under that prefix. No `sudo` required. Use this for
CI, recipe development, and `elda host test-tree --install` smokes.

Point `ELDA_ROOT` or use a chroot if you need a different root without editing config.

### Live host system mode

On a real system, `defaults.prefix` is usually `/usr` and `allow_system_mode` controls whether
Elda may manage the host root without a one-shot override.

```toml
[defaults]
prefix = "/usr"
allow_system_mode = false   # gate live /usr management
```

Run a single command on the live host with:

```sh
elda -S i ripgrep
```

`-S` requests system mode for that invocation only. Elda re-execs through the configured
privilege provider (`[privilege].provider`: `auto`, `doas`, `sudo`, `run0`, `su`, or `none`).

Before re-exec, human mode prints a **Privilege Escalation** frame (provider chosen, policy).
Already running as root continues without prompting.

**Unprivileged user prefix:** A user-owned prefix (e.g. under `$HOME`) never escalates; Elda
skips privilege providers entirely. That is the supported way to use Elda without touching the
host system.

---

## Configuration: policy before commands

Elda reads `/etc/elda/config.toml` (or `<root>/etc/elda/config.toml` in a disposable root).
Drop-ins:

```text
/etc/elda/config.toml
/etc/elda/remotes.d/*.toml
/etc/elda/caches.d/*.toml
/etc/elda/extensions.d/*.toml
/etc/elda/host.d/*.toml          # maintainer publish profiles
/etc/elda/recipes/<pkg>/pkg.lua  # local package definitions
```

| Section | Daily impact |
| --- | --- |
| `[defaults]` | Default remote, prefix, `install_recommends`, `install_preference` (binary vs source bias), system-mode gate |
| `[privilege]` | How `-S` and host mutations escalate |
| `[profile]` | Base profile, init family, native/foreign arch |
| `[flags.*]` | Feature flags that change dependency resolution and `variant_id` |
| `[resolver.provider_preferences]` | Which virtual provider wins when several packages provide the same name |
| `[git]` | Tag policy, allowed protocols for git clones |
| `[logging]` | Session logs for mutating commands (`level` 1-3, or `--log-level`) |
| `[display]` | Human vs machine output, tree characters for live progress |
| `[metadata]` | Order Elda tries strategies for `elda a` / raw-link installs |
| `[trust].release_keys` | Trusted keys for signed release assets; declared sidecars fail closed when keys are missing |

Use [examples/config/](./examples/config/) as a starting point. [su/config.toml](./su/config.toml)
shows a host-oriented privilege layout.

**Remotes** are signed indexes plus optional `packages_url` (git repo of recipes). **Caches**
are optional HTTP mirrors keyed by payload SHA256 - lookups only; `elda sync` never reads a cache
as a package index.

---

## The daily operator loop

A typical session on a machine that already has Elda configured:

1. **Refresh metadata** - `elda sync` (all remotes) or `elda sync yoka-main` (named only).
2. **See what changed** - read sync summary (add/remove counts, stale remotes, trust issues).
3. **Plan a change** - `elda i <pkg> --dry-run` or `elda u --dry-run` before mutating.
4. **Apply** - run without `--dry-run`; watch live progress on a TTY (or `--no-stream` in CI).
5. **Verify** - `elda check`, `elda verify <pkg>`, or `elda doctor` after large changes.
6. **Read advisories** - human success output lists reboot/restart hints when kernel, init, or
   boot assets changed.

```sh
elda sync
elda search hypr
elda i hyprland --dry-run
elda i hyprland
elda check
```

Use `elda doctor` when bootstrap paths, remotes, trust, or release-readiness look wrong - it is
the aggregated "is this root healthy?" command, not a substitute for `elda check` after every
small install.

---

## Global CLI behavior

```sh
elda --help
elda <command> --help
elda -V                    # detailed version, build, schema report
elda version              # same as -V
elda --json version       # automation
```

| Flag | Effect |
| --- | --- |
| `--json` | Machine-readable reports (stable shapes for scripting) |
| `--dry-run` | Plan only; no journal commit |
| `--no-stream` | Suppress live progress; print final report once |
| `--offline` | Sync/install uses cached verified snapshots and local payloads only |
| `--log-level 0-3` | Per-run session log verbosity (overrides `[logging].level`) |
| `-S` | One-shot system mode on the live host |
| `--accept-rotated-key <remote>` | Confirm TOFU key rotation for that remote |

**Exit codes:** `0` success, `1` runtime/operator failure, `2` resolution/validation failure,
`3` trust/auth failure.

**Human output** uses framed sections (target, resolution, plan, progress, result). Install
dry-runs include a **Preflight** block (disk space, managed bytes, privilege posture). Mutating
commands can emit **post-transaction advisories** (reboot required, kernel follow-up).

**Review memory:** Generated metadata and interbuild definitions get content-addressed stamps.
`elda review ls|info|diff|forget` manages them; unchanged hashes skip repeat `[Y/n/e]` prompts.

---

## First-time bootstrap on a host

```sh
sudo elda init                    # create layout + default config if missing
sudo install -Dm0644 config.toml /etc/elda/config.toml
sudo install -d /etc/elda/remotes.d /etc/elda/caches.d /etc/elda/recipes

elda rmt add yoka-main=https://example.invalid/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:... \
  --signature-url https://example.invalid/index-v1.json.zst.sig \
  --metadata-url https://example.invalid/remote-metadata-v1.toml \
  --packages-url https://github.com/you/pkgs.git \
  --channel stable

elda cache add lan=https://cache.example.invalid/elda --priority 20
elda sync
elda doctor
```

`elda rmt ls` / `elda rmt info yoka-main` / `elda rmt trust yoka-main` inspect registration and
verification state. `elda cache ls` shows cache policy and usage.

**Trust models:**

- **Pinned** - you supply trusted keys up front; safest for production remotes.
- **TOFU** - first successful sync can enroll a key; later rotations need explicit operator
  confirmation (`--accept-rotated-key`) in non-interactive paths.

**Channels** - each remote document selects a channel (`stable`, `staging`, ...). The index must
publish that channel; Elda fails closed if you request a channel the remote does not ship.

---

## Installing software

### What you can pass to `elda i`

| Target kind | Example | What happens |
| --- | --- | --- |
| Synced name | `elda i ripgrep` | Resolve from local snapshot of remote index |
| Local recipe | `elda i ./examples/recipes/02-source-cargo/ripgrep` | Use `pkg.lua` under recipes dir |
| Git URL | `elda i https://github.com/org/repo` | Metadata strategy -> review -> build/install |
| Explicit lane | `elda ig foo` / `elda ib foo` | Force source or binary |

Lane selection for maintained packages follows `install_preference`, remote priority, and
recipe `default_lane`. Overrides:

```sh
elda i foo --prefer-source
elda i foo --prefer-binary
elda i foo --use=+wayland,-x11    # one-shot flag override (see flags section)
```

### Dependency behavior

- **Hard dependencies** (`depends`) are installed automatically and recorded as `dep`.
- **Recommends** install by default when satisfiable (`install_recommends = true`); disable in
  config or per command when you want a minimal closure.
- **Conflicts** and **replaces** are enforced at plan time - ambiguous or illegal plans fail
  closed with a structured blocked report.
- **Virtual providers** (`provides`) are resolved using remote priority and
  `[resolver.provider_preferences]`; ambiguity without policy is an error.

Always dry-run non-trivial closures:

```sh
elda i plasma-meta --dry-run
```

### Git and release installs

```sh
elda i https://github.com/Mjoyufull/fsel
elda i https://github.com/Mjoyufull/fsel --to-tag v3.3.1
elda i https://github.com/Mjoyufull/fsel --to-rev abcdef1
elda i https://github.com/Mjoyufull/fsel --strategy git_release
```

Inspect upstream before committing:

```sh
elda git tags https://github.com/Mjoyufull/fsel --with-releases
elda git releases Mjoyufull/fsel --tag v3.3.1
```

VCS-style installed packages stay pinned to the commit installed until you explicitly upgrade
with a new ref (`elda u pkg --to-tag ...`).

### Metadata without installing (`elda a` / `elda add`)

`elda a <link>` runs the same discovery as a raw-link install but **stops after** writing or
updating local `pkg.lua` (and companions). Use it to scaffold recipes, import foreign trees, or
refresh metadata without touching installed state.

- Without `--replace`, existing `pkg.lua`, `build.lua`, patches, and vendor data are preserved.
- Bulk git monorepos may open a staged import review (`Y` / `n` / `e` to edit tree).
- Human interactive mode uses `[Y/n/e]` review gates; `e` opens the editor and re-validates.

```sh
elda a https://github.com/Mjoyufull/fsel
elda a https://github.com/heather7283/heather7283-overlay --exclude firefox vlc
```

### Interactive search shorthand

```sh
elda hypr          # human mode: interactive search, then optional install
elda search hypr --interactive
```

---

## Removing, upgrading, and pinning policy

### Remove

```sh
elda rm ripgrep
elda rm gnome --cascade          # remove dependents that become invalid
elda rm foo --purge-conffiles    # drop saved conffile backups too
```

Removal updates world, journals, and (in system mode) staged activation. Packages required by
other installed packages are blocked unless `--cascade`.

### Upgrade

```sh
elda sync                        # required: refresh index before comparing versions
elda u                           # upgrade world + required closure from current snapshots
elda u ripgrep
elda u --rebuild-variant-drift   # rebuild when flags changed variant_id
elda u fsel --to-tag v3.4.0      # move a git package to a new ref
```

`elda u` compares installed versions to the **current synced snapshot** for each remote. It does
not auto-upgrade held packages or packages blocked by pins unless you change policy.

### Pin, hold, downgrade

```sh
elda pin discord-bin 0:0.0.99-1   # exact version constraint
elda unpin discord-bin
elda hold mesa --source yoka-main # block upgrades from that remote
elda unhold mesa

elda downgrade hyprland
elda downgrade hyprland 0:0.44.0-1
elda downgrade fsel --to-tag v3.2.0
```

**Pin** - stay on an exact version. **Hold** - skip upgrades during `elda u`. **Downgrade** -
install an older cached/archived build; reverse dependencies are checked.

### Orphans

```sh
elda autoremove --dry-run
elda autoremove
```

Removes packages with install reason `dep` that are no longer required by world, profiles, or
weak-dep policy you still want.

---

## Inspecting installed state

| Command | Use when |
| --- | --- |
| `elda ls` | Quick inventory; filters: `--explicit`, `--deps`, `--held`, `--pinned`, `--source-kind` |
| `elda info <pkg>` | Full metadata, deps, provides, provenance, provider assets |
| `elda files <pkg>` | Path list from manifest |
| `elda files owner <path>` | Which package owns a path |
| `elda files search <term>` | Search installed paths |
| `elda why <pkg>` | Why it is installed (`base`, `explicit`, `dep`, profile) |
| `elda rdeps <pkg>` | Reverse dependencies; `--all`, `--weak` |
| `elda diff <pkg>` | Live drift vs manifest; `--candidate` vs next upgrade |
| `elda versions <git-target>` | Upstream version candidates (git-backed) |

```sh
elda ls --held
elda info ripgrep
elda why ripgrep
elda rdeps openssl --all
elda diff firefox --candidate
```

---

## Health, verification, and recovery

| Command | Role |
| --- | --- |
| `elda check` | Aggregated health: orphans, trust, triggers, adoption warnings, backend notes |
| `elda doctor` | Bootstrap paths, remotes, advisories, release-readiness |
| `elda verify [<pkg>]` | Manifest vs disk; collisions are errors |
| `elda reverify <pkg>` | Same, scoped |
| `elda recover` | Finish or roll back incomplete transaction |
| `elda rollback [<state-id>]` | Restore archived prefix/system state when supported |
| `elda fix-triggers` | Repair pending system trigger outputs |

After a failed install, run `elda recover` before retrying. If activation completed but the
result is wrong, `elda rollback` uses archived state (prefix mode is fully supported in current
slice; system mode depends on backend archives).

---

## Configuration files on disk (conffiles)

When a packaged file is marked as a conffile and you edited it, upgrades may write
`path.eldanew` and keep `path.eldasave` instead of overwriting silently.

```sh
elda config pending              # queue of pending conffile decisions
elda config diff /etc/foo.conf
elda config apply /etc/foo.conf  # accept the package version (merge policy applies)
elda config keep /etc/foo.conf   # keep your modified copy
```

Resolve conffiles before assuming a upgrade "failed" - sometimes activation succeeded but conffiles
await a decision.

---

## Triggers and system integration

Packages may ship declarative `tmpfiles`, `sysusers`, `alternatives`, and hook metadata. Elda
materializes these during activation on the system backend.

```sh
elda trigger ls
elda trigger info ldconfig
elda trigger run <name>          # when exposed for repair
elda fix-triggers              # reconcile drifted trigger state
```

`elda pf show` lists pending init/provider transitions and activation class (`live`,
`restart-required`, `relog-required`, `reboot-required`). Kernel/init/boot changes are honest
**reboot-required** - Elda updates on-disk state; it does not hot-swap a running kernel.

---

## Profiles and machine shape

Profiles are package anchors that express **desired machine shape**: desktop stack, base system,
init family, foreign architectures.

```sh
elda pf show
elda pf apply yoka-core
elda pf apply yoka-core yoka-desktop-hyprland --init dinit --foreign-arch i386
elda pf add yoka-desktop-hyprland
elda pf rm yoka-desktop-hyprland
elda pf set-init dinit
elda pf clear-init
elda pf add-foreign-arch i386
```

`pf apply` installs profile packages and records them as world anchors. Policy fields persist
in the profile-state record and feed `pf show` / `state export`.

**Desired-state documents** capture intent for reproducibility - not a disk image:

```sh
elda state show
elda state export > workstation.eldastate
elda state import workstation.eldastate   # still runs normal solver + transactions
```

Import reapplies remotes, world, and profile anchors through the same verification path as
manual installs.

---

## Flags and variants

Feature flags (wayland, pipewire, optional codecs) live in config and recipes. They affect
dependency edges and produce a **variant_id** recorded on install.

```sh
elda fl check
elda fl check firefox --use=+mp4,-h264
elda fl diff firefox
```

After changing `[flags.global]` or profile flags, installed packages may **drift** from the
resolved variant. `elda u --rebuild-variant-drift` rebuilds affected packages.

Per-package overrides use `[flags.package."name"]` or version atoms like
`[flags.package."firefox>=130"]` in `config.toml`.

---

## Remotes, sync, and offline use

**Native remote** - `index_url` points at a signed `index-v1.json.zst` (plus `.sig`). Optional
`packages_url` enables source builds from a pinned recipe git commit in the index.

**Interemote** - `index_url` is a git URL to an overlay or `srcpkgs` tree. Sync runs bounded
parsers (Gentoo overlay, Void-style templates, etc.) and merges translated metadata into the
local snapshot - no foreign `emerge` / `xbps-src` at sync time.

```sh
elda rmt preview heather-overlay
elda sync heather-overlay
elda rmt disable heather-overlay
elda rmt enable heather-overlay
elda rmt set-priority yoka-main 90
elda rmt rm old-remote
```

### `--exclude` on remotes and bulk import

`--exclude` must be **last** on the command line. Every operand after it is a package name to
skip (space- or comma-separated). Flags after `--exclude` are rejected.

```sh
elda rmt add overlay=https://example.invalid/overlay.git --exclude firefox vlc
elda a https://example.invalid/overlay.git --exclude pkg1, pkg2
```

Invalid: `elda i foo --exclude bar --prefer-binary` (`--prefer-binary` would be parsed as a name).

### Offline

```sh
elda sync --offline              # refresh from last verified local snapshot only
elda i foo --offline             # install from local payload cache when indexed
```

Caches speed binary installs: Elda tries `cache base/<sha256>` before the origin `asset_url`.

---

## Local recipes (`elda rc`)

Maintained packages live under `/etc/elda/recipes/<pkgname>/` (`pkg.lua`, optional `build.lua`).

```sh
elda rc add mytool ./src/tree
elda rc add yoka-core --kind profile
elda rc check
elda rc ls
elda rc show mytool
elda rc diff mytool
elda rc publish-ready mytool
elda rc edit mytool
elda rc rm mytool                 # only when not installed
```

`rc check` validates Lua and spec fields before you install. `publish-ready` lists blockers for
CI/index publication.

---

## Vendor binaries and AppImages

**Vendor** - pin third-party release binaries as local recipes:

```sh
elda vendor add rg-bin BurntSushi/ripgrep@14.1.0 --binary rg
elda vendor import vendor.lock.json
elda vendor export vendor.lock.json
```

**AppImage** - inspect before authoring `source.kind = "appimage"` recipes:

```sh
elda appimage inspect ./AppName.AppImage
```

---

## Migration and adoption (current slice)

Elda can **import installed-state metadata** from other package managers for planning and
transition. Current adapters: `pacman`, `apt` (and `dpkg`), `apk`, `xbps`, `portage`.

```sh
elda adopt --from pacman firefox    # one package into Elda DB
elda mg from pacman                 # whole-system metadata import
```

**Important:** Adoption records what the foreign PM believes is installed; it does not
silently take over files on disk. Path collisions fail closed. `mg lock` / `mg unlock` for live
coexistence are not fully landed - treat migration as metadata-first today.

Use `elda check` for adoption warnings (including "zombie" adoptions with no upgrade path).

---

## Maintainer and CI workflows (overview)

If you publish indexes or run a forge, use the dedicated hosting guide. Day-to-day **machine**
operation does not require these commands.

```sh
# Recipe tree on a maintainer machine
elda host scan-tree ./packages
elda host test-tree ./packages
elda host doctor --profile yoka-main

# Publish pipeline (after CI builds payloads)
elda publish plan ./packages --channel stable
elda publish run ./packages --channel stable
elda publish finalize --channel stable
```

See [eldaforgehosting/host-maintainer-tools.md](./eldaforgehosting/host-maintainer-tools.md).

**CI / forge** (local workspace slice):

```sh
elda ci sub https://github.com/you/pkg
elda ci run https://github.com/you/pkg
elda ci status pkg
elda ci pr pkg
elda forge search term
elda qa lint pkg
```

---

## Extensions and background service

```sh
elda ext ls                      # registrations under extensions.d
elda daemon status
elda daemon refresh              # trigger sync/notification pass when configured
```

Extensions are gated by `[capabilities].extension_runtime` in config.

---

## Scripting tips

- Prefer `--json` for automation; combine with `--dry-run` to capture plans.
- Use `--no-stream` in CI logs to avoid cursor control sequences.
- Parse exit codes; trust failures are distinct from resolution failures.
- Session logs (when enabled) appear in `[logging].dir`; paths are echoed in human success output.
- Bare query `elda foo` only works in human mode - scripts should call `elda search foo --json`.

---

## Quick command index

| Area | Commands |
| --- | --- |
| Lifecycle | `i`, `ig`, `ib`, `rm`, `u`, `downgrade`, `autoremove`, `sync` |
| Metadata | `a`, `add`, `rc *`, `vendor *`, `git tags`, `git releases` |
| Policy | `pin`, `unpin`, `hold`, `unhold`, `fl check`, `fl diff` |
| Inspection | `ls`, `info`, `files`, `why`, `rdeps`, `diff`, `search` |
| Health | `doctor`, `check`, `verify`, `recover`, `rollback`, `fix-triggers` |
| Config files | `config pending`, `config diff`, `config apply`, `config keep` |
| Profiles | `pf *`, `state show`, `state export`, `state import` |
| Remotes | `rmt *`, `cache *` |
| Migration | `adopt`, `mg from` |
| Review | `review ls`, `review info`, `review diff`, `review forget` |
| Version | `-V`, `version` |

For hosting and platform-specific index layout, continue with
[eldaforgehosting/README.md](./eldaforgehosting/README.md).
