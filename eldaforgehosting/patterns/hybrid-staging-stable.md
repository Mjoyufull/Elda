# Pattern: Hybrid Staging + Stable Channels

**Goal:** Same recipe Git monorepo, **separate branches and signed indexes** per channel (`main` / `staging` / ...), different trust posture. This is the default channel model for native Elda hosting - not "one branch only."

## Two Remotes, One `packages_url`

```sh
# Production
elda rmt add yoka-main=https://example.com/stable/index-v1.json.zst \
  --trust pinned --trusted-key ed25519:... \
  --packages-url https://github.com/org/pkgs.git --channel stable

# Staging
elda rmt add yoka-staging=https://example.com/staging/index-v1.json.zst \
  --trust tofu \
  --packages-url https://github.com/org/pkgs.git --channel staging
elda rmt disable yoka-staging
```

Clients with both enabled resolve packages per channel and remote priority rules in [SPEC.md](../../SPEC.md).

## CI Layout

| Branch | Index artifact | Channel |
| --- | --- | --- |
| `main` | `stable/index-v1.json.zst` | `stable` |
| `staging` | `staging/index-v1.json.zst` | `staging` |

## Binary Promotion

1. Build and test on staging index.
2. Copy payload digests (unchanged) into stable index generation, or rebuild from same `repo_commit` on `main`.
3. Sign stable index; clients `sync` production remote only.

## Docs

- [../trust-and-signing.md](../trust-and-signing.md)
- `examples/config/remotes.d/yoka-staging.toml`
