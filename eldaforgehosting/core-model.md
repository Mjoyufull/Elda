# Core Hosting Model

## Surfaces

| Surface | Purpose | Client commands |
| --- | --- | --- |
| Forge | Git review, PR/MR submission, auth, CI trigger | `elda ci`, `elda forge` |
| Remote | Signed metadata and package-definition authority | `elda rmt *`, `elda sync` |
| Cache | Content-addressed payload mirror only | `elda cache *` |
| Interemote | Dynamic source repo translated during sync | `elda rmt preview`, `elda sync` |
| Snapshot import | One-time local recipe import | `elda a <url>` |

## Rules

- **`elda sync` reads remotes, not caches.** Caches accelerate payload fetch after the signed index names a digest.
- **Remotes** define package names, versions, dependencies, providers, source lanes, binary payload URLs, channels, and trust policy.
- **Caches** mirror blobs at `GET <cache base>/<sha256>` only.
- **Source-only remote:** signed index carries package records with `pkg_lua`, `repo_commit`, and `packages_url`; no `asset_url` / `sha256` / `payload_sig`.
- **Binary remote:** same as source-only plus payload fields and optional SBOM/attestation URLs.
- **Interemote:** `index_url` points at a foreign metadata Git tree (Gentoo overlay, Void `srcpkgs`). Elda translates on sync; not a binary repo.
- **Snapshot import:** `elda a <url>` writes local editable recipes once; unlike interemotes, it does not keep updating from upstream on sync.

## Deployment Styles (Summary)

| Style | Index | Recipe Git | Binaries |
| --- | --- | --- | --- |
| Source-only native | Signed `index-v1.json.zst` | Required via `packages_url` | Clients build |
| Full binary native | Signed index | Required for source lane | `asset_url` + sigs |
| Interemote | Generated at sync from upstream Git | Upstream repo is the source | None from interemote |
| Cache mirror | N/A | N/A | Digest files only |

See [source-only-native-remote.md](./source-only-native-remote.md), [binary-binhost-remote.md](./binary-binhost-remote.md), and [interemote-foreign-repos.md](./interemote-foreign-repos.md).
