# Maintainer Workflow

## Submit a Package

```sh
elda ci sub https://github.com/Mjoyufull/fsel
elda ci status fsel
elda ci pr fsel
```

## Run Processing Locally

```sh
elda ci run https://github.com/Mjoyufull/fsel
elda ci logs fsel
```

## Batch a Closure

```sh
elda ci batch new wayland-stack
elda ci batch add wayland-stack wayland wayland-protocols libxkbcommon libinput
elda ci batch push wayland-stack
elda ci status wayland-stack
```

## Publish Checklist

Your CI wrapper (or manual steps) should:

1. Run `elda host test-tree` (and optional `elda publish plan`) on the recipe monorepo.
2. Run Elda CI/build/publish (`elda publish run`) in the runner.
3. Upload payloads, manifests, signatures, SBOMs, and attestations to the chosen origin.
4. Run `elda publish finalize` so production `asset_url` values match the uploaded layout.
5. Upload the signed index and detached signature to `index_url`.
6. Optionally mirror payloads into cache hosts with `elda-populate` or your static host.
7. Optionally update remote metadata documents for key rotation.

Missing step 5 means clients cannot sync a usable remote. See [host-maintainer-tools.md](./host-maintainer-tools.md).

## Forge Browse / Search

```sh
elda forge search <term>
elda forge browse
elda qa check <pkg>
```

Exact flags and JSON shapes: [USAGE.md](../USAGE.md), [SPEC.md](../SPEC.md).
