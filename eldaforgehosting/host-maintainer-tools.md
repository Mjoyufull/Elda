# Host Maintainer Tools

Landscaped commands for recipe monorepos, signed indexes, and client onboarding. Behavior
contracts are in [SPEC.md](../SPEC.md); day-to-day operator examples are in [USAGE.md](../USAGE.md).

## Host profile (`/etc/elda/host.d/`)

A host profile names:

- the recipe Git remote and default branch
- index and metadata publication URLs per channel
- signing keys and upload targets (GitHub Releases, static HTTP, object storage)
- optional cache mirror bases

```sh
elda host doctor --profile yoka-main
elda host status --profile yoka-main
elda host client-bundle yoka-main
```

`client-bundle` emits `rmt add` snippets, trust hints, and config fragments clients can paste.

## Recipe tree workflow

```sh
elda host scan-tree ./packages
elda host test-tree ./packages
elda host test-tree ./packages --install
elda host diff-tree ./packages --since origin/main
elda host push-recipes --profile yoka-main
elda host link ./packages
```

- `scan-tree` — parse and `rc publish-ready` status for every package under the tree.
- `test-tree` — dry-run validation by default; `--install` opt-in disposable-root install smoke.
- `diff-tree` — packages changed since a git ref (for publish planning).
- `push-recipes` — push recipe commits to the configured forge remote.
- `link` — sync a maintainer tree into the local CI workspace.

## Publish pipeline

```sh
elda publish plan ./packages --channel stable
elda publish run ./packages --channel stable
elda publish finalize --channel stable
elda publish diff --channel stable
elda publish promote --from staging --to stable
elda publish sign --channel stable
```

| Step | Purpose |
| --- | --- |
| `plan` | Show what would be built or indexed; blockers per package |
| `run` | Build/stage payloads and write channel-local index artifacts |
| `finalize` | Rewrite `asset_url` / cache URLs for production after upload |
| `diff` | Compare planned index against the last published index |
| `promote` | Copy index rows or channel metadata between channels |
| `sign` | Sign index and refresh detached signature sidecars |

Production URL rewrite belongs in **`publish finalize`**, not in ad hoc index edits.

## CI template

```sh
elda host init-ci --forge github
```

Writes a starter workflow under `.github/workflows/` (or the matching forge layout) that runs
Elda CI/publish steps. Customize secrets and runner labels for your forge.

## Cache front-end helper

```sh
elda host print-cache-config lan-mirror
```

Prints example nginx/Caddy/static-server rules for content-addressed `GET /<sha256>` cache layout.

## Channel model

Use **one recipe monorepo** with **separate branches and signed indexes per channel** (`main` →
`stable`, `staging` → `staging`, …). Clients register separate remotes (or URLs) with matching
`channel =` values. See [patterns/hybrid-staging-stable.md](./patterns/hybrid-staging-stable.md).
