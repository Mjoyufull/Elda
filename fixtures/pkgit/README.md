# `pkgit` reference fixtures

These files are legacy `pkgit` reference input for Elda.

They are not runtime assets for Elda yet.
They exist so the rewrite has concrete samples for the `pkgit` behavior we are preserving at the UX layer or importing into normalized metadata.

Source references:

- `pkgit/src/getDeps.nim`
- `pkgit/src/buildPkg.nim`
- `pkgit/src/installPkg.nim`
- `pkgit/src/searchPkgs.nim`
- `pkgit/src/listPkgs.nim`
- `pkgit/README.md`

Fixture groups:

- `pkgdeps/`: dependency hints in the legacy `pkgit` line format
- `bldit/`: legacy shell build entrypoint shape
- `direct-install/`: direct git install targets and optional tag selection
- `repo-flow/`: flat repo-list inputs for search/list flows
