# `pkgit` reference fixtures

These files are `pkgit` reference input for Elda. They model upstream pkgit
behavior and formats; they are not a checkout of pkgit and they are not used as
local runtime code.

They are not runtime assets for Elda yet.
They exist so the rewrite has concrete samples for the `pkgit` behavior we are
preserving at the UX layer or importing into normalized metadata.

Fixture groups:

- `pkgdeps/`: dependency hints in the legacy `pkgit` line format
- `bldit/`: legacy shell build entrypoint shape
- `direct-install/`: direct git install targets and optional tag selection
- `repo-flow/`: flat repo-list inputs for search/list flows
