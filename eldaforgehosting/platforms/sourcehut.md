# SourceHut Hosting

SourceHut fits **Git-first recipe hosting** with static index publication. Release discovery in Elda treats SourceHut tags as git artifacts (`git.sr.ht`), not a GitHub-style releases API.

## Recipe Repo

Host `packages/<pkgname>/` on `git.sr.ht/~you/pkgs` (or self-hosted equivalent).

```sh
git clone https://git.sr.ht/~you/pkgs
# author recipes, push with normal sr.ht workflow
```

## Index and Payloads

Publish the signed index and signatures on any HTTPS static host (pages, your own nginx, object storage). Point `packages_url` at the sr.ht Git URL.

```sh
elda rmt add yoka-main=https://static.example.com/elda/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:... \
  --packages-url https://git.sr.ht/~you/pkgs
```

## Binary Lanes

- Prefer **native index** `asset_url` fields for prebuilt payloads.
- For upstream projects on SourceHut, `release_asset` metadata can target tag artifacts when checksum sidecars exist.
- Richer SourceHut ecosystem metadata beyond tag artifacts remains bounded; see [gitstuffeld.md](../../gitstuffeld.md).

## CI / Builds

Use sr.ht builds to run `elda ci run` and upload artifacts to your static index host. Submission PR flow depends on forge API support in your Elda version; when unavailable, use Git email/patch workflow and merge recipes manually, then republish the index.
