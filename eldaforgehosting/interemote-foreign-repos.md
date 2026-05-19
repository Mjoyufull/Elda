# Interemote (Foreign Source Repos)

Use an **interemote** when the upstream source tree is itself a supported foreign package metadata repository (Gentoo overlay, Void-style `srcpkgs`, etc.).

## Gentoo Overlay Example

```sh
elda rmt add heather-overlay=https://github.com/heather7283/heather7283-overlay \
  --exclude firefox --exclude vlc
elda rmt preview heather-overlay
elda sync heather-overlay
```

## Void-Style `srcpkgs` Example

```sh
elda rmt add blackhole-vl=https://github.com/Event-Horizon-VL/blackhole-vl \
  --exclude firefox --exclude vlc
elda rmt preview blackhole-vl
elda sync blackhole-vl
```

## Remote Documents

Gentoo overlay:

```toml
name = "heather-overlay"
index_url = "https://github.com/heather7283/heather7283-overlay"
channel = "stable"
priority = 120
enabled = true
trust = "tofu"
trusted_keys = []
allow_stale = false
exclude = ["firefox", "vlc"]
```

Void-style:

```toml
name = "blackhole-vl"
index_url = "https://github.com/Event-Horizon-VL/blackhole-vl"
channel = "stable"
priority = 130
enabled = false
trust = "tofu"
trusted_keys = []
allow_stale = false
exclude = ["firefox", "vlc"]
```

## Notes

- **`rmt preview`** clones and reports catalog/parse issues without writing a synced snapshot.
- **`--exclude`** is persistent remote policy.
- Output distinguishes parser failures from excluded packages.
- Interemotes are **source metadata**, not binary repos.
- Remove with `elda rmt rm <name>` then `elda sync`.

Examples in-repo: `examples/config/remotes.d/heather-overlay.toml`, `examples/config/remotes.d/blackhole-vl.toml`.
