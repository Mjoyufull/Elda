# Gitea / Forgejo Hosting

Self-hosted forge with Git review. Keep the **client-facing index and cache** on boring static HTTP when possible so sync does not depend on forge API uptime.

## Topology

```text
gitea.example.com/yoka/pkgs.git           recipes
packages.example.com/elda/              signed index + sigs (static)
cache.example.com/elda/               optional payloads
```

## Remote Registration

```sh
elda rmt add yoka-main=https://packages.example.com/elda/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://packages.example.com/elda/index-v1.json.zst.sig \
  --metadata-url https://packages.example.com/elda/remote-metadata-v1.toml \
  --packages-url https://gitea.example.com/yoka/pkgs.git
```

## Submission Config

```toml
[submission]
mode = "pr"
auto_open = true
auto_assign = false
auth = "token"
token_env = "ELDA_GITEA_TOKEN"
api_base = "https://gitea.example.com/api/v1"
remote_name = "origin"
base_branch = "main"
```

Forgejo uses the same Gitea-compatible API v1 surface for PR creation in the current Elda slice.

## Releases

Gitea/Forgejo release assets are supported for `release_asset` binary lanes when metadata names the host. Publish payloads via forge releases **or** static URLs in the native index—both work if URLs are stable.

## Why Split Index From Forge

- Forge upgrades do not block `elda sync`.
- You can CDN-cache `index-v1.json.zst` aggressively.
- CI uploads index to object storage while MRs stay on Gitea.

See [../patterns/full-forge-with-ci.md](../patterns/full-forge-with-ci.md).
