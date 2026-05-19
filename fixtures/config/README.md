# `config` fixtures

Small runtime-shaped examples for `/etc/elda`.

Use these files when you need lean test inputs instead of the heavily annotated
`examples/config/` tree. All files are valid against the current runtime config
or document models.

## Primary configs

- `system-default.toml`: system-mode `/usr` config with binary install preference.
- `prefix-source.toml`: non-system `/opt/elda` prefix config with source install preference.
- `profile-defaults.toml`: profile and flag-layer fixture, including an atom-versioned package flag override.
- `su-system.toml`: system-mode config that selects `su` as the privilege provider.

## Drop-ins

- `remotes.d/yoka-main.toml`: pinned native remote with `packages_url`, signature URL, metadata URL, and channel.
- `remotes.d/yoka-staging.toml`: disabled TOFU staging native remote.
- `remotes.d/heather-overlay.toml`: enabled Gentoo overlay interemote with package excludes.
- `remotes.d/blackhole-vl.toml`: disabled Void-style `srcpkgs` interemote with package excludes.
- `caches.d/lan-mirror.toml`: content-addressed payload cache document.
- `extensions.d/nix-flake-adapter.toml`: capability-scoped extension registration.

Useful commands:

```sh
elda rmt ls
elda rmt preview heather-overlay
elda rmt info yoka-main
elda rmt trust yoka-main
elda sync yoka-main
elda sync heather-overlay
```

Metadata write rule: generated/imported recipe metadata is preserved unless the
operator passes `--replace` on the command that writes it.
