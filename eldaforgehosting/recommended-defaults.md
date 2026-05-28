# Recommended Defaults

Start **source-only**, add binaries and caches when the review flow is stable.

1. Put package definitions in `https://github.com/Mjoyufull/Elda` or a dedicated `pkgs` repo.
2. Publish a signed index and detached signature.
3. Register clients with **pinned** trust and `packages_url`.
4. Add a cache only after payload flow exists.
5. Add full binary publication once package-definition review is stable.

This keeps metadata native from day one and lets binary hosting grow without changing the client contract.

For staging, use a second remote with `channel = staging`, TOFU or pinned keys, and `elda rmt disable` until testers opt in.
