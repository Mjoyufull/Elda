# Bitbucket, Codeberg, and Other Git Hosts

Elda's contract is **Git URL + HTTPS index URLs**, not a specific forge brand.

## What Always Works

| Piece | Requirement |
| --- | --- |
| Recipe repo | Any cloneable Git remote (`packages_url`) |
| Index | HTTPS `index_url` + `signature_url` |
| Trust | Pinned keys or TOFU per policy |
| Binaries | Stable `asset_url` in signed index |

## Bitbucket

- Host recipes on `bitbucket.org/you/pkgs`.
- Publish index via Bitbucket Downloads, a static host, or object storage.
- PR-based `elda ci pr` requires a supported API in your Elda version; until then, merge via Bitbucket UI and republish index from CI.

## Codeberg

- Gitea-compatible API; follow [gitea-forgejo.md](./gitea-forgejo.md) with `api_base = https://codeberg.org/api/v1`.
- Use Codeberg Pages or external static host for the signed index.

## Plain Git Server (No Web UI)

```sh
packages_url = https://git.example.com/elda-pkgs.git
index_url    = https://mirror.example.com/elda/index-v1.json.zst
```

Operators submit recipes via email, internal GitLab, or `rsync` to a blessed branch; CI signs and uploads the index.

## Self-Hosted `git://` or SSH-Only

If clients cannot use the protocol, expose an HTTPS mirror for `packages_url` or adjust `[git].allowed_protocols` in config. Sync and install paths fail closed on disallowed transports.

## Choosing a Forge

| Need | Suggestion |
| --- | --- |
| Largest contributor familiarity | GitHub / GitLab |
| Self-hosted, lightweight | Gitea / Forgejo |
| Minimal web UI, Git email culture | SourceHut |
| Static binhost only | [generic-static-http.md](./generic-static-http.md) |
