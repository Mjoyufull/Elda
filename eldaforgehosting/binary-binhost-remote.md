# Binary / Binhost Native Remote

Use this when you publish **prebuilt payloads** (binhost-style) in addition to—or instead of—expecting every client to build from source.

## Everything From Source-Only, Plus

- Payload archives (per arch/channel)
- Payload signatures (`.minisig` or contract-defined sidecars)
- Manifests, optional SBOM and attestation URLs
- Optional cache mirrors for digest reuse

## Extra Indexed Package Fields

- `asset_url`
- `sha256`
- `payload_sig`
- Optional `sbom_url`, `attestation_url`

Clients resolve from the signed index, try configured caches by digest, then fall back to `asset_url`.

## Publish Flow (Maintainer-Owned)

1. Run Elda CI/build/publish locally or in CI (`elda ci run`, publish helpers).
2. Collect payloads, manifests, signatures, SBOMs, attestations, and index output.
3. Upload payloads and sidecars to your origin (Releases, static HTTP, object storage).
4. Upload the **signed index** and detached signature to `index_url`.
5. Optionally mirror payloads into cache hosts (`elda-populate cache mirror-remote`).
6. Optionally update `remote-metadata-v1.toml` for key rotation.

If step 4 is missing, clients have no usable remote.

## Client With Cache

```sh
elda rmt add yoka-main=https://packages.example.com/elda/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://packages.example.com/elda/index-v1.json.zst.sig \
  --metadata-url https://packages.example.com/elda/remote-metadata-v1.toml \
  --packages-url https://github.com/Mjoyufull/Elda.git
elda cache add lan-mirror=https://cache.example.com/elda --priority 20
elda sync
```

See [cache-server.md](./cache-server.md) and [patterns/binhost-style-binary.md](./patterns/binhost-style-binary.md).
