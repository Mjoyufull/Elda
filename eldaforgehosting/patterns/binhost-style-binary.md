# Pattern: Binhost-Style Binary Remote

**Goal:** Clients download verified payloads by digest; builds happen in CI, not on every laptop.

## Architecture

```text
[CI] build -> payload + minisig -> upload origin
              ↓
         sign index (asset_url, sha256, payload_sig)
              ↓
[Clients] sync -> cache try -> asset_url fallback -> install
```

## Maintainer Steps

1. Maintain recipe Git (`packages_url`) for source lane and metadata.
2. CI runs `elda ci run` / publish pipeline per arch/channel.
3. Upload each payload + sidecars to Releases or static host.
4. Regenerate index with binary fields; sign and publish.
5. Optional: `elda-populate cache mirror-remote` to seed LAN cache.

## Client Steps

```sh
elda rmt add yoka-main=... --packages-url ... --trust pinned ...
elda cache add lan=https://cache.lan/elda --priority 10
elda sync
elda i fsel --prefer-binary   # when binary lane exists
```

## Trust

- Index signature validates catalog.
- `payload_sig` / `[trust].release_keys` govern release assets when configured.

## Docs

- [../binary-binhost-remote.md](../binary-binhost-remote.md)
- [../cache-server.md](../cache-server.md)
