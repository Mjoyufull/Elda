# Elda Usage Guide

Quick reference for common Elda workflows.

## Global Conventions

```sh
# Read-only commands support machine-readable output
elda ls --json
elda info ripgrep --json

# Mutating commands support dry runs
elda i ripgrep --dry-run
elda u --dry-run --json

# Request live host system mode explicitly
elda -S ls
elda -S u
```

Rules:

- `--json` is for read-only output and structured scripting.
- `--dry-run` shows the planned mutation without applying it.
- `-S` is the one-shot override for live host system mode.

## First Setup

### Bootstrap the machine profile

```sh
elda pf apply yoka-core --init dinit --foreign-arch i386
```

This installs the base machine shape, records the selected init-provider, and makes foreign-arch policy explicit.

### Register remotes and caches

```sh
elda rmt add yoka-main --forge github --owner yoka-ci --index index
elda cache add kyokai-cache https://cache.kyokai.lan/elda --priority 20
elda sync
```

This gives Elda:

- a metadata source
- one or more payload mirrors
- a synced package snapshot for search, install, and upgrade

## Search And Discovery

### Search packaged software

```sh
elda search ripgrep
elda search hypr --regex
```

`search` answers "is this already packaged in my synced remotes?"

### Search upstream forge software

```sh
elda forge search ripgrep
elda forge browse BurntSushi/ripgrep
```

`forge` commands are for discovery and import workflows, not solver state.

## Install Packages

### Normal install

```sh
elda i ripgrep
elda i gnome
elda i kde-plasma
```

`elda i` follows the package definition plus user config and prefers the binary lane when one exists.

### Force the source or binary lane

```sh
elda ig ripgrep
elda ib ripgrep

elda i ripgrep --prefer-source
elda i ripgrep --prefer-binary
```

Lane rules:

- `ig` forces the maintained source lane
- `ib` forces the maintained binary lane
- `--prefer-source` and `--prefer-binary` are one-shot overrides for `elda i`
- package identity stays the same; provenance is surfaced in UI instead of renaming the package

### Install from git directly

```sh
elda i https://github.com/Mjoyufull/bfetch
elda i github:Mjoyufull/bfetch
elda ig github:Mjoyufull/bfetch
```

Direct git installs go through the same staged payload and transaction model as maintained packages.

### Install with flags or build variants

```sh
elda i github:Mjoyufull/bfetch --use=+wayland,-x11
```

Variant choices become part of build identity and should remain inspectable and reproducible.

## Vendor Binary Workflows

### Import a GitHub release

```sh
elda vendor add rg-bin BurntSushi/ripgrep@14.1.0 --binary rg
elda i rg-bin
```

### Import a direct URL

```sh
elda vendor add gh-bin https://github.com/cli/cli/releases/download/v2.4.0/gh_2.4.0_linux_amd64.tar.gz --binary gh
elda i gh-bin
```

### Import or export a vendor bundle

```sh
elda vendor import vendor.lock.json
elda vendor export rg-bin
```

`vendor add` and `vendor import` create normal local package definitions under `/etc/elda/recipes/`.

## Package State And Inspection

### List installed packages

```sh
elda ls
```

Recommended output includes package name, version, reason, origin, remote, and current state membership.

### Inspect one package

```sh
elda info ripgrep
```

`info` should expose identity, version, deps, weak deps, provides, conflicts, replaces, origin, confidence, URLs, license, installed-file summary, and provider-specific assets.

### Show owned files

```sh
elda files ripgrep
elda files owner /usr/bin/rg
```

### Explain why a package is present

```sh
elda why ripgrep
elda rdeps openssl
elda rdeps openssl --all
elda rdeps openssl --weak
```

## Remove, Upgrade, Downgrade

### Remove packages

```sh
elda rm ripgrep
elda rm gnome --cascade
elda rm somepkg --purge-conffiles
```

Rules:

- `--cascade` removes reverse dependencies that become invalid
- `--purge-conffiles` drops preserved `*.eldasave` state

### Upgrade packages

```sh
elda sync
elda u
elda u ripgrep
```

`u` upgrades the whole machine or the named package plus the required closure from one synced snapshot.

### Downgrade

```sh
elda downgrade hyprland
elda downgrade hyprland 0:0.44.0-1
```

Downgrades come from cache or archive sources and still respect hold and pin policy unless explicitly overridden.

## Pin, Hold, And Orphan Control

```sh
elda pin discord-bin 0:0.0.99-1
elda unpin discord-bin

elda hold mesa
elda hold mesa --source yoka-main
elda unhold mesa

elda autoremove --dry-run
elda autoremove
```

Policy rules:

- `pin` records an exact-version constraint
- `hold` blocks upgrades
- `autoremove` only removes packages that still qualify as orphans at transaction time

## Verification, Repair, And Recovery

```sh
elda verify
elda verify ripgrep
elda reverify ripgrep
elda diff ripgrep
elda diff ripgrep --candidate
elda check
elda recover
elda rollback
elda rollback prefix-1710000000000
elda fix-triggers
```

Use these when you need to:

- verify files against manifests
- compare live state or candidate state
- inspect health warnings
- recover interrupted transactions
- reactivate archived state
- rerun pending trigger work

## Profiles And Machine Shape

### Apply profiles

```sh
elda pf apply yoka-core
elda pf apply yoka-core yoka-desktop-hyprland --init dinit --foreign-arch i386
```

### Inspect current machine shape

```sh
elda pf show
elda state show
```

These commands should expose:

- active profile anchors
- provider families
- active init choice
- foreign-arch policy
- pending system-change handlers
- current activation class

## Desired State Export And Import

```sh
elda state export > workstation.eldastate
elda state import workstation.eldastate
```

Desired-state documents capture machine intent, not a raw filesystem image.

## Local Recipe Management

### Add or import a local recipe

```sh
elda rc add ripgrep https://github.com/BurntSushi/ripgrep
elda rc add mypkg ./local-source-tree
```

### Validate recipe trees

```sh
elda rc check
elda rc check ripgrep
```

Maintained package definitions live under `/etc/elda/recipes/<pkgname>/`.

## Foreign Repositories And Migration

### Install from interepo

```sh
elda i <artix> networkmanager
elda i <aur> fsel-bin
elda i <chimera> hyprland
elda i <gentoo> seatd
```

Without an explicit interepo tag, native remotes win first and enabled interepos are searched by priority.

### Adopt or migrate from another PM

```sh
elda adopt --from pacman firefox
elda mg from pacman
elda mg lock pacman
elda mg unlock pacman
```

Migration rules:

- `adopt` is single-package takeover
- `mg from` imports whole-system installed state
- adopted packages preserve provenance instead of being rewritten as fake native installs

## CI, Forge, And Publishing

### Submit or build maintained packages

```sh
elda ci sub https://github.com/rikona/fsel
elda ci run https://github.com/rikona/fsel
elda ci status fsel
elda ci pr fsel
elda ci logs fsel
```

### Work with batches or stacks

```sh
elda ci batch new wayland-stack
elda ci batch add wayland-stack wayland wayland-protocols libxkbcommon libinput
elda ci batch push wayland-stack
elda ci status wayland-stack
```

The CI model is PR/MR-first and publishes payloads, manifests, signatures, SBOMs, attestations, and index updates after merge.

## QA And Reproducibility

```sh
elda qa lint wayland
elda qa build wayland
elda qa smoke wayland --profile core --init dinit
elda qa stack wayland-stack
elda qa repro wayland
elda qa diff wayland
```

Use QA commands to validate metadata, build behavior, stack health, and reproducibility before publishing.

## Extensions

```sh
elda ext ls
```

Extension points are explicit and bounded. They are for activation backends, build backends, object analyzers, boot backends, interepo adapters, migration adapters, and provider migrators.

## Config Example

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

Install-lane rules:

- `install_preference = "binary"` means `elda i` prefers a declared binary lane when one exists
- `install_preference = "source"` flips the default lane preference
- `ig`, `ib`, `--prefer-source`, and `--prefer-binary` override the config default

Prefix rules:

- `prefix = "/usr"` is system mode
- live host system mode requires `defaults.allow_system_mode = true` or `elda -S`
- any other prefix gets its own DB, cache namespace, and state root

## Stable CLI Surface

### Root commands

```text
i ig ib rm u sync ls search info files verify reverify why rdeps pin unpin hold unhold adopt downgrade diff check recover rollback fix-triggers autoremove
```

### Namespaces

```text
rmt add
rc add edit check
ci sub run status pr retry logs batch
vendor add import export
forge search browse
pf apply show set-init
fl check diff
mg from lock unlock
state show export import
cache add ls
daemon run status refresh
ext ls
qa lint build smoke stack repro diff
```
