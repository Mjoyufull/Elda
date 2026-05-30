# Reference Files In This Repo

| Path | Purpose |
| --- | --- |
| `config.toml` | Sample `/etc/elda/config.toml` (copy to `/etc/elda/`) |
| `su/config.toml` | Sample for hosts that escalate with `su` |
| `examples/config/config.toml` | Annotated field-by-field config reference |
| `examples/config/host.d/*.toml` | Maintainer publish profiles (`host` / `publish`) |
| `examples/config/` | Annotated remote, cache, and extension examples |
| `fixtures/config/` | Lean test fixtures |
| `examples/recipes/` | Recipe authoring examples |
| [USAGE.md](../USAGE.md) | Operator commands |
| `man/elda.1` | Man page source |

Remote examples:

- `examples/config/remotes.d/yoka-main.toml` - source-capable pinned remote
- `examples/config/remotes.d/yoka-staging.toml` - staging channel
- `examples/config/remotes.d/heather-overlay.toml` - Gentoo interemote
- `examples/config/remotes.d/blackhole-vl.toml` - Void-style interemote

Cache examples:

- `examples/config/caches.d/lan-mirror.toml`
- `examples/config/caches.d/readonly-archive.toml`

Host profile example:

- `examples/config/host.d/yoka.toml.example` - copy to `/etc/elda/host.d/yoka.toml`
