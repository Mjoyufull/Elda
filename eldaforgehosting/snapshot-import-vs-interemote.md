# Snapshot Import vs Interemote

| | Snapshot import (`elda a`) | Interemote (`rmt add` + `sync`) |
| --- | --- | --- |
| Updates from upstream | No - local editable copy | Yes - each `sync` refreshes |
| Signed Elda index | Not required | Native remotes need one; interemotes generate from Git |
| Use case | Try a foreign URL once, edit locally | Track Gentoo overlay / `srcpkgs` long-term |
| Remote document | N/A | `index_url` points at upstream Git |

**Snapshot import** writes recipes under your local recipe tree once through the metadata review gate. Good for bootstrapping or experimenting.

**Interemote** keeps Elda's view of the foreign catalog in sync with upstream commits. Use `rmt preview` before the first `sync`.

Do not register a one-time `elda a` URL as an interemote unless you intend ongoing sync from that Git tree.
