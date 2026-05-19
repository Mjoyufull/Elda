# Client Setup Recipes

## One Source-Only Native Remote

```sh
elda rmt add yoka-main=https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --packages-url https://github.com/Mjoyufull/Elda.git
elda sync yoka-main
```

## Binary Remote Plus Cache

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

## Dynamic Overlay Remote

```sh
elda rmt add heather-overlay=https://github.com/heather7283/heather7283-overlay \
  --exclude firefox --exclude vlc
elda rmt preview heather-overlay
elda sync heather-overlay
```

## Remove a Remote

```sh
elda rmt rm heather-overlay
elda sync
```

## Dry-Run Remote Registration

```sh
elda rmt add yoka-staging=https://example.com/index-v1.json.zst --dry-run
```
