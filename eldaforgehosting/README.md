# Elda Native Forge Hosting

Operator documentation for hosting Elda package metadata, recipe Git repositories, binary payloads, caches, and forge workflows. Behavior contracts live in [SPEC.md](../SPEC.md).

Example maintainer used throughout:

- developer: Rikona
- GitHub user: `Mjoyufull`
- project repo: `https://github.com/Mjoyufull/Elda`

## What Elda Hosts (and What You Still Operate)

Elda ships the client/runtime pieces for native hosting:

| Surface | You provide | Elda commands |
| --- | --- | --- |
| **Forge** | Git host, review, tokens, CI runners | `elda ci`, `elda forge` |
| **Remote** | Signed index, optional `packages_url`, trust keys | `elda rmt *`, `elda sync` |
| **Cache** | Static HTTP by content digest | `elda cache *`, `elda-populate` |
| **Interemote** | Upstream overlay / `srcpkgs` Git repo | `elda rmt preview`, `elda sync` |
| **Snapshot import** | One URL of foreign metadata | `elda a <url>` (one-time local recipes) |

Elda does **not** run a turnkey hosted binary service. Your CI or upload job still publishes to GitHub/GitLab/Gitea Releases, object storage, a static HTTP host, or a LAN mirror.

## Choose Your Hosting Pattern

| Goal | Start here |
| --- | --- |
| AUR-like: recipes in Git, users build from source | [patterns/aur-style-source-only.md](./patterns/aur-style-source-only.md) |
| Binhost-style: prebuilt payloads + signed index | [patterns/binhost-style-binary.md](./patterns/binhost-style-binary.md) |
| Full forge: Git review + CI + publish + clients | [patterns/full-forge-with-ci.md](./patterns/full-forge-with-ci.md) |
| Mirror someone else's payloads locally | [patterns/lan-cache-mirror-only.md](./patterns/lan-cache-mirror-only.md) |
| Consume Gentoo overlay or Void `srcpkgs` as a remote | [interemote-foreign-repos.md](./interemote-foreign-repos.md) |
| One-time import, not a living remote | [snapshot-import-vs-interemote.md](./snapshot-import-vs-interemote.md) |

## Pick a Git or HTTP Platform

| Platform | Guide |
| --- | --- |
| GitHub | [platforms/github.md](./platforms/github.md) |
| GitLab (hosted or self-hosted) | [platforms/gitlab.md](./platforms/gitlab.md) |
| Gitea / Forgejo | [platforms/gitea-forgejo.md](./platforms/gitea-forgejo.md) |
| SourceHut | [platforms/sourcehut.md](./platforms/sourcehut.md) |
| Any static HTTP / S3 / nginx / Caddy | [platforms/generic-static-http.md](./platforms/generic-static-http.md) |
| Bitbucket, Codeberg, plain `git://` | [platforms/bitbucket-and-others.md](./platforms/bitbucket-and-others.md) |

Platform guides cover **where to put** the recipe repo, signed index, payloads, and submission tokens. The pattern guides explain **what fields** each deployment style needs.

## Core Topics (Platform-Agnostic)

1. [Core model](./core-model.md) — forge vs remote vs cache vs interemote
2. [Repository layout](./repository-layout.md) — `packages/<pkgname>/`, published index tree, cache digests
3. [Getting recipes into Git](./getting-recipes-into-git.md) — `rc add`, `elda a`, `ci sub`, vendor, bulk import
4. [Source-only native remote](./source-only-native-remote.md)
5. [Binary / binhost native remote](./binary-binhost-remote.md)
6. [Trust and signing](./trust-and-signing.md)
7. [Cache server](./cache-server.md)
8. [Interemote foreign repos](./interemote-foreign-repos.md)
9. [Snapshot import vs interemote](./snapshot-import-vs-interemote.md)
10. [Host maintainer tools](./host-maintainer-tools.md) — `host *`, `publish *`, `/etc/elda/host.d/`
11. [Maintainer workflow](./maintainer-workflow.md) — `ci sub`, `ci run`, publish steps
12. [Client setup recipes](./client-setup.md)
13. [Recommended defaults](./recommended-defaults.md)
14. [Reference files in this repo](./reference-files.md)

## Quick Rules

- `elda sync` reads **remotes**, not caches.
- Remotes carry package names, versions, deps, `packages_url`, `repo_commit`, and optional binary `asset_url` / `sha256` / `payload_sig`.
- Caches only answer `GET <cache base>/<sha256>`.
- Source-only remotes are real remotes when the signed index includes `pkg_lua`, `repo_commit`, and `packages_url`.
- Interemotes re-sync from upstream Git; snapshot imports write editable local recipes once.

## Related Operator Docs

- [USAGE.md](../USAGE.md) — day-to-day commands
- [README.md](../README.md) — project entrypoint
- [phase.md](../phase.md) — implementation status for hosting and runtime slices
- [examples/config/](../examples/config/) — sample `remotes.d`, `caches.d`, `config.toml`, host profiles
- [examples/recipes/](../examples/recipes/) — recipe authoring examples
