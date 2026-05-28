# Recommended Repository Layout

## Package-definition Git repo

```text
pkgs/
  packages/
    fsel/
      pkg.lua
      build.lua
      ci.toml
      patches/
    yoka-core/
      pkg.lua
  README.md
```

Each indexed package should have a stable path `packages/<pkgname>/` with `pkg.lua` and any `build.lua`, patches, and CI config the build needs.

## Published index / static host

```text
published-index/
  index-v1.json.zst
  index-v1.json.zst.sig
  remote-metadata-v1.toml
  remote-metadata-v1.toml.sig
```

The **signed compressed index** is what clients fetch on `elda sync`. Detached signatures and optional `remote-metadata-v1.toml` support trust rotation.

## Payload / cache host

```text
cache/
  <sha256>
  <sha256>
```

Caches are flat digest-addressed trees. They can live on a different hostname than the index or recipe repo.

## Splitting services

These three trees can be separate services:

| Service | Example |
| --- | --- |
| Development + recipes | `github.com/Mjoyufull/Elda` |
| Signed index + release assets | GitHub Releases or raw branch |
| LAN / CDN cache | `cache.example.com/elda` |

A practical GitHub setup keeps packages in the main repo, publishes the signed index as release assets or a static branch, and optionally mirrors large payloads to a dedicated cache host.
