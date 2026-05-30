# Pattern: AUR-Style Source-Only Remote

**Goal:** Community or team maintains `pkg.lua` in Git; users install with `elda i` and build locally - no published binaries required.

## Architecture

```text
[Maintainers] -> Git push -> packages/<pkg>/pkg.lua
                ↓
         CI signs index-v1.json.zst
                ↓
[Clients] elda sync -> elda i pkg (source lane)
```

## Maintainer Steps

1. Create `pkgs` repository with `packages/<pkgname>/`.
2. Add recipes (`elda rc add`, hand-edit, or `elda ci sub`).
3. On each release:
   - Pin `repo_commit` per package in the index.
   - Embed `pkg_lua` snapshot or reference per [SPEC.md](../../SPEC.md).
   - Sign index; upload index + `.sig`.
4. Publish `remote-metadata-v1.toml` when rotating keys.

## Client Steps

```sh
elda rmt add community=https://example.com/index-v1.json.zst \
  --trust pinned --trusted-key ed25519:... \
  --packages-url https://github.com/org/pkgs.git
elda sync community
elda i fsel
```

## Compared to Arch AUR

| AUR | Elda source-only remote |
| --- | --- |
| PKGBUILD in Git | `pkg.lua` + optional `build.lua` |
| `makepkg` on client | Elda build/stage/install |
| No official binary repo | Optional later binhost on same remote |

## Optional: Interemote Instead

If upstream is already a Gentoo overlay, you may **consume** it as an interemote instead of mirroring recipes - see [../interemote-foreign-repos.md](../interemote-foreign-repos.md).

## Docs

- [../source-only-native-remote.md](../source-only-native-remote.md)
- [../getting-recipes-into-git.md](../getting-recipes-into-git.md)
