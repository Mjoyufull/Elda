# GitHub Hosting

## Recommended Topology

```text
github.com/Mjoyufull/Elda              package definitions and development
github.com/Mjoyufull/Elda/releases     signed index and/or payload origins
cache.example.com/elda                 optional digest-addressed cache
```

You can split recipe repo and index repo (`Mjoyufull/elda-index`) if you prefer smaller clone URLs for clients.

## Source-Only Client Setup

```sh
elda rmt add yoka-main=https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst.sig \
  --metadata-url https://raw.githubusercontent.com/Mjoyufull/Elda/dev/remote-metadata-v1.toml \
  --packages-url https://github.com/Mjoyufull/Elda.git
```

Alternative index hosting: commit index files to a `gh-pages` branch or `raw.githubusercontent.com` (ensure CDN caching headers suit your rollout policy).

## Submission Config

```toml
[submission]
mode = "pr"
auto_open = true
auto_assign = false
auth = "token"
token_env = "ELDA_GITHUB_TOKEN"
api_base = "https://api.github.com"
remote_name = "origin"
base_branch = "dev"
```

## Maintainer Commands

```sh
elda ci sub https://github.com/Mjoyufull/fsel
elda ci run https://github.com/Mjoyufull/fsel
elda ci status fsel
elda ci pr fsel
elda forge fork https://github.com/Mjoyufull/Elda   # requires `gh` on PATH
```

## Binary Payloads

Upload payloads and sidecars to **GitHub Releases** (per-tag assets) or another stable URL. Republish the signed index with `asset_url`, `sha256`, and `payload_sig` for each package/arch.

Release asset discovery for installs uses the GitHub releases API; self-hosted GitHub Enterprise uses the same API base pattern with your host preserved in metadata.

## CI Notes

- Use `ELDA_GITHUB_TOKEN` with `repo` + PR scope for `ci pr`.
- GitHub Actions can run `elda ci run` and upload `index-v1.json.zst` + `.sig` as release assets on tag push.
- Large payloads: prefer an external cache or release assets outside the main repo blob limit.

See [../patterns/full-forge-with-ci.md](../patterns/full-forge-with-ci.md).
