# Pattern: Full Elda Forge (Git + CI + Publish + Clients)

**Goal:** Contributors open PRs/MRs; CI builds and publishes signed indexes; users add your remote.

## Components

| Layer | Technology |
| --- | --- |
| Recipe Git | GitHub / GitLab / Gitea / Forgejo / etc. |
| Review | Host PR/MR + `elda ci pr` |
| Build | CI runner with Elda + toolchains |
| Index | Signed `index-v1.json.zst` on Releases or static HTTP |
| Optional cache | LAN or CDN digest mirror |
| Clients | `rmt add` + `sync` + `i` |

## Workflow

```mermaid
flowchart LR
  subgraph forge [Forge]
    A[Contributor fork/branch]
    B[elda ci sub / git push]
    C[Review MR]
  end
  subgraph ci [CI]
    D[elda ci run]
    E[Build payloads]
    F[Sign index]
  end
  subgraph host [Publication]
    G[Upload index + binaries]
  end
  subgraph clients [Clients]
    H[elda sync]
    I[elda i]
  end
  A --> B --> C --> D --> E --> F --> G --> H --> I
```

## Minimal `config.toml` (Maintainer Machine)

```toml
[submission]
mode = "pr"
auth = "token"
token_env = "ELDA_GITHUB_TOKEN"
api_base = "https://api.github.com"
base_branch = "main"
```

## Platform Guides

- [../platforms/github.md](../platforms/github.md)
- [../platforms/gitlab.md](../platforms/gitlab.md)
- [../platforms/gitea-forgejo.md](../platforms/gitea-forgejo.md)

## Start Small

Ship **source-only** index first; turn on binary fields when CI artifacts are reliable. See [../recommended-defaults.md](../recommended-defaults.md).
