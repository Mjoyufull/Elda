# Elda Examples

This directory contains copyable, annotated examples for the files operators and
maintainers hand-author around Elda. It is the long-form companion to
`fixtures/`, which stays small for tests.

Source of truth order remains: `SPEC.md`, then `USAGE.md`, then runtime code.

## Relationship to root `config.toml`

| File | Role |
| --- | --- |
| [`../config.toml`](../config.toml) | Practical sample you can copy to `/etc/elda/config.toml` on a host |
| [`su/config.toml`](../su/config.toml) | Same shape with `provider = "su"` and system mode enabled |
| `config/config.toml` | Heavily commented tour of every section and commented optional blocks |

Use the repository root files for a quick install; use `examples/config/` when you
need inline explanation of each field.

## Layout

```text
examples/
  config/                 annotated /etc/elda documents
  recipes/                complete pkg.lua/build.lua examples
  ci/                     package CI policy example
  import-inputs/          legacy and raw-link inputs for add/import flows
```

## Config Examples

- `config/config.toml`: annotated `/etc/elda/config.toml` with defaults,
  privilege, profile, flags, metadata strategy order, git policy, submission,
  display, capabilities, and trust sections.
- `config/remotes.d/yoka-main.toml`: pinned native remote using the project host
  `https://github.com/Mjoyufull/Elda` as the package-definition repo.
- `config/remotes.d/yoka-staging.toml`: disabled TOFU staging remote.
- `config/remotes.d/local-mirror.toml`: local `file://` signed index mirror.
- `config/remotes.d/heather-overlay.toml`: Gentoo overlay interemote.
- `config/remotes.d/blackhole-vl.toml`: Void-style `srcpkgs` interemote.
- `config/caches.d/*.toml`: content-addressed payload caches.
- `config/extensions.d/*.toml`: explicit capability-scoped extension records.
- `config/host.d/yoka.toml.example`: maintainer publish profile (index URLs, signing,
  upload targets); copy to `/etc/elda/host.d/<name>.toml` for `elda host *` and
  `elda publish *` (see [eldaforgehosting/host-maintainer-tools.md](../eldaforgehosting/host-maintainer-tools.md)).

Useful commands:

```sh
elda rmt ls
elda rmt info yoka-main
elda rmt trust yoka-main
elda rmt preview heather-overlay
elda sync yoka-main
elda sync heather-overlay
```

## Recipe Examples

Each `recipes/<NN>-.../<pkgname>/` directory can be copied to
`/etc/elda/recipes/<pkgname>/` or committed as `packages/<pkgname>/` in a native
package-definition repo.

| Directory | Shows |
| --- | --- |
| `01-binary-github-release/fd` | single binary release lane |
| `02-source-cargo/ripgrep` | source-only Cargo recipe |
| `03-source-with-build-lua/zig-toy` | custom `build.lua` companion |
| `04-dual-lane-source-and-binary/fd` | one package with source and binary lanes |
| `05-multi-arch-binary/elda-cli` | arch-specific release assets |
| `06-vendor-url-archive/yt-dlp` | direct vendor URL archive |
| `07-system-service-providers/example-daemon` | sysusers/tmpfiles/provider assets/hooks |
| `08-conffiles-and-state/example-config-pkg` | conffiles and state paths |
| `09-flag-suite-extended/flag-demo` | extended flags and conditional deps |
| `10-meta-anchor/yoka-desktop-meta` | payload-less meta package |
| `11-profile-machine-shape/yoka-laptop-profile` | profile package machine shape |
| `12-split-subpackages/llvm-suite` | split package metadata |
| `13-interbuild-nix-flake/hello-flake` | bounded Nix flake interbuild |
| `14-interbuild-gentoo-overlay/eselect` | bounded Gentoo overlay interbuild |
| `15-build-systems-*` | CMake, Go, and Meson declarative builds |
| `16-appimage-managed/demo-tool` | managed AppImage binary lane |

Interbuild lanes for AUR PKGBUILD and XBPS templates are supported at runtime; use
`elda a <url>` on those trees or author recipes after `elda git releases` /
metadata import. Numbered recipe folders for those parsers may be added later.

Metadata safety: generated/imported recipe files preserve existing local
metadata unless the command includes `--replace`.

## Try One Locally

```sh
elda rc check ./examples/recipes/01-binary-github-release/fd
sudo install -d /etc/elda/recipes/fd
sudo cp -r examples/recipes/01-binary-github-release/fd/. /etc/elda/recipes/fd/
elda i fd --dry-run
elda rc rm fd
```

## Forge And CI

- `ci/ci.toml` is the package-side CI policy file used in native forge repos.
- `import-inputs/legacy.pkgdeps` and `legacy.bldit` document legacy import input.
- `import-inputs/git-targets.txt` is a plain list for raw git add/install tests.

For hosting a native package-definition repo, signed index, and payload cache,
read [eldaforgehosting/README.md](../eldaforgehosting/README.md).
