# Generic Static HTTP / Object Storage

Any host that serves stable HTTPS objects works for Elda **indexes**, **payloads**, and **caches**. Git forge choice is independent.

## Index Host

Requirements:

- `GET` returns exact bytes for `index-v1.json.zst`
- Same for detached `.sig` and optional `remote-metadata-v1.toml`

Examples:

- nginx / Caddy / Apache `file_server`
- S3 + CloudFront / R2 / B2 with public or signed URLs
- GitHub `raw.githubusercontent.com` or release assets
- IPFS gateway (only if URLs are stable for the remote document lifetime)

## Cache Host

Flat digest tree:

```text
https://cache.example.com/elda/<sha256>
```

See [../cache-server.md](../cache-server.md).

## Recipe Git

Recipes stay in **any** Git server (`git@`, `https://`, self-hosted). Only `packages_url` must be cloneable with your `[git].allowed_protocols` policy.

## Binhost Without a Forge

You can run **binhost-only**:

1. Build elsewhere (CI, local).
2. Upload payloads + signed index to static storage.
3. Clients never need a forge—only `rmt add` + `sync`.

Optional second remote with `packages_url` if you also ship source lanes.

## rclone / rsync Publish Sketch

```sh
# after local publish output in ./out/
rclone sync ./out/ remote:elda-index/
rclone sync ./payloads/ remote:elda-cache/
```

Record the public HTTPS bases in remote and cache documents.
