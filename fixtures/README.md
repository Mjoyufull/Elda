# Fixtures

Small, readable fixture inputs used by tests, docs, and quick manual checks.

- `config/`: lean `/etc/elda` snapshots plus drop-in `remotes.d/`, `caches.d/`, and
  `extensions.d/` documents (see [`config/README.md`](./config/README.md) for
  `system-default.toml`, `prefix-source.toml`, `profile-defaults.toml`, and
  `su-system.toml`).
- `recipes/`: compact `pkg.lua` fixtures for local recipe, profile, and flag-system checks.
- `pkgit/`: legacy `pkgit` inputs that Elda imports or preserves as behavior reference.

For annotated operator examples, use the top-level `examples/` tree. Fixtures
stay intentionally small so they are easy to copy into temporary roots during
runtime tests.
