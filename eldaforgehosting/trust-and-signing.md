# Trust And Signing

Decide trust policy **before** publishing a remote clients will use in production.

## Production (Pinned)

```sh
elda rmt add yoka-main=https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst.sig \
  --metadata-url https://raw.githubusercontent.com/Mjoyufull/Elda/dev/remote-metadata-v1.toml \
  --packages-url https://github.com/Mjoyufull/Elda.git \
  --channel stable
```

## Staging (TOFU, Disabled By Default)

```sh
elda rmt add yoka-staging=https://github.com/Mjoyufull/Elda/releases/download/staging/index-v1.json.zst \
  --trust tofu \
  --packages-url https://github.com/Mjoyufull/Elda.git \
  --channel staging
elda rmt disable yoka-staging
```

## Rules

- **Pinned** remotes need at least one trusted key or key file.
- **TOFU** is acceptable for human first-use staging; unattended jobs should use pinned keys.
- Key rotation goes through `metadata_url` plus `${metadata_url}.sig`; rotation still requires explicit operator acceptance during sync.
- **Insecure** remotes are only for throwaway local testing.

## Inspection

```sh
elda rmt ls
elda rmt info yoka-main
elda rmt trust yoka-main
elda sync yoka-main
```

Release payload trust when recipes declare `release_asset.signature` is governed by `[trust].release_keys` in config; missing keys fail closed when a signature sidecar exists. See [SPEC.md](../SPEC.md) for the trust contract.
