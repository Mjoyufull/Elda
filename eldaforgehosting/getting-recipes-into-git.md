# Getting Recipes Into Git

Regardless of forge platform, package definitions live in Git as `packages/<pkgname>/pkg.lua` (plus `build.lua`, patches, `ci.toml` as needed). This guide lists every common path into that tree.

## 1. Author Locally, Push to Your Forge

```sh
mkdir -p packages/myapp
elda rc add ./upstream-tree --pkgname myapp   # or hand-write pkg.lua
elda rc check packages/myapp/pkg.lua
elda rc format packages/myapp/pkg.lua       # normalize formatting
git add packages/myapp && git commit && git push
```

Use your host's normal PR/MR flow, or Elda submission:

```sh
elda ci sub https://github.com/you/myapp-recipe
elda ci pr myapp
```

## 2. Link From Upstream Git (`elda a` / `elda add`)

Metadata-first import from a URL (branch, tag, or raw repo):

```sh
elda a https://github.com/upstream/project
# review gate -> local packages/<pkgname>/ with generated or merged metadata
```

One-time snapshot: edit locally, commit to **your** recipe repo, publish index pointing at your `repo_commit`.

## 3. CI Submission Pipeline

Maintainers submit package repos; your forge integration clones and opens review:

```sh
elda ci sub https://github.com/Mjoyufull/fsel
elda ci run https://github.com/Mjoyufull/fsel
elda ci status fsel
elda ci pr fsel
```

Configure `[submission]` in `config.toml` (see platform guides under [platforms/](./platforms/)).

## 4. Vendor / Import Workflows

- `elda vendor add` / `vendor import` - vendor trees with metadata replacement policy
- Bulk snapshot import - one-time many-package import with review gates (not a living remote)

## 5. Interemote -> Local Recipe (Advanced)

Sync an overlay interemote, then copy or adapt generated metadata into your native repo if you want a **frozen** native index instead of dynamic translation. Most teams either stay on interemotes or maintain a native repo - not both for the same package name without priority policy.

## 6. Fork an Existing Forge Repo

When using GitHub and `gh` is available:

```sh
elda forge fork https://github.com/Mjoyufull/Elda
```

Then clone your fork, add packages, publish signed index from your namespace.

## After Recipes Are in Git

1. Tag or record the commit you will pin in the index (`repo_commit` per package).
2. Build/publish CI artifacts if you ship binaries.
3. Publish signed `index-v1.json.zst` + `.sig`.
4. Register clients with `packages_url` pointing at the same Git remote.

See [maintainer-workflow.md](./maintainer-workflow.md) and [repository-layout.md](./repository-layout.md).
