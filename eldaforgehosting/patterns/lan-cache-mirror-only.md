# Pattern: LAN Cache Mirror Only

**Goal:** You do not host recipes or indexes; you mirror **payloads** from an existing upstream remote to save bandwidth.

## You Do Not Need

- Your own `packages_url`
- Your own signed index (unless you also run a private remote)

## You Do Need

- Permission to fetch upstream payloads (public remote or internal mirror policy)
- Static HTTP cache layout
- `elda-populate` or rsync from a machine that already synced upstream

## Setup

```sh
# Upstream already configured
elda rmt add upstream=... --trust pinned ...

elda cache add lan=https://cache.lan/elda --priority 5
elda-populate cache mirror-remote --remote upstream --channel stable --cache lan
# rsync /var/cache/elda/... to nginx root if populate wrote locally
```

## Org-Wide Rollout

1. Admin registers `upstream` remote on golden image.
2. Admin registers `lan` cache pointing at internal mirror.
3. Workstations `sync`; installs hit LAN digest first.

## Docs

- [../cache-server.md](../cache-server.md)
