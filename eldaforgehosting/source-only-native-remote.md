# Source-Only Native Remote

Use this for an **AUR-like** Elda package-definition repository: clients sync metadata and build from source using pinned `repo_commit` and `packages_url`.

## What You Host

- Git package-definition repo with `packages/<pkgname>/pkg.lua` (and `build.lua`, patches, etc.)
- Signed index file (`index-v1.json.zst`)
- Detached signature for the index
- Optional signed `remote-metadata-v1.toml` for key rotation

## Indexed Package Fields (Required)

- Identity: `pkgname`, `pkgver`, `pkgrel`, `arch`, `channel`
- Dependency / provider / conflict metadata
- Exact `pkg_lua` and `repo_commit` for the recipe tree
- **No** `asset_url`, `sha256`, or `payload_sig`

## Remote Document

The remote must include **`packages_url`** so Elda can clone the package-definition repo and check out `repo_commit` for builds.

```toml
name = "yoka-main"
index_url = "https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst"
channel = "stable"
packages_url = "https://github.com/Mjoyufull/Elda.git"
metadata_url = "https://raw.githubusercontent.com/Mjoyufull/Elda/dev/remote-metadata-v1.toml"
signature_url = "https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst.sig"
priority = 100
enabled = true
allow_stale = false
trust = "pinned"
trusted_keys = [
  "ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344",
]
exclude = []
```

## Client Registration

```sh
elda rmt add yoka-main=https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://github.com/Mjoyufull/Elda/releases/download/index/index-v1.json.zst.sig \
  --metadata-url https://raw.githubusercontent.com/Mjoyufull/Elda/dev/remote-metadata-v1.toml \
  --packages-url https://github.com/Mjoyufull/Elda.git \
  --channel stable
elda sync yoka-main
```

Plan before writing: `elda rmt add ... --dry-run`.

See [trust-and-signing.md](./trust-and-signing.md) and [patterns/aur-style-source-only.md](./patterns/aur-style-source-only.md).
