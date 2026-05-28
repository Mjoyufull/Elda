# Hosting Patterns

End-to-end recipes combining index shape, Git layout, and operator workflow.

| Pattern | Description |
| --- | --- |
| [aur-style-source-only.md](./aur-style-source-only.md) | Recipes in Git; clients build; signed metadata remote |
| [binhost-style-binary.md](./binhost-style-binary.md) | Prebuilt payloads + signed index (+ optional cache) |
| [full-forge-with-ci.md](./full-forge-with-ci.md) | Git review, CI, publish, client remotes |
| [hybrid-staging-stable.md](./hybrid-staging-stable.md) | Two channels, two remotes, shared `packages_url` |
| [lan-cache-mirror-only.md](./lan-cache-mirror-only.md) | Mirror upstream binaries; no recipe hosting |

Platform-specific URL examples: [../platforms/](../platforms/).
