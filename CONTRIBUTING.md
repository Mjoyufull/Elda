# Contributing to Elda

Thank you for your interest in contributing to Elda. This project welcomes contributions from the community, whether you are fixing bugs, improving documentation, or proposing new features.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [How to Contribute](#how-to-contribute)
- [Branching Strategy](#branching-strategy)
- [Commit Standards](#commit-standards)
- [Pull Request Process](#pull-request-process)
- [Code Review](#code-review)
- [Testing](#testing)
- [Coding Standards](#coding-standards)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Release Process](#release-process)
- [What Not To Do](#what-not-to-do)
- [Getting Help](#getting-help)

---

## Getting Started

Before contributing, please:

1. Read [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for Git workflow, branching, and releases
2. Read [CODE_STANDARDS.md](./CODE_STANDARDS.md) for Rust structure, testing, and implementation style
3. Check existing [issues](https://github.com/Mjoyufull/Elda/issues) and [pull requests](https://github.com/Mjoyufull/Elda/pulls) to avoid duplicating work
4. Understand that **all code changes go through pull requests** — no exceptions
5. **Fork the repository** if you do not have write access (most contributors)

For Project and CLI behavior questions, [SPEC.md](./SPEC.md) is the written contract. If something is not specified there, do not guess in code or docs — note the gap and ask in an issue or discussion.

### Key Resources

- **Issue Tracker**: [GitHub Issues](https://github.com/Mjoyufull/Elda/issues)
- **Project Standards**: [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md)
- **Code Standards**: [CODE_STANDARDS.md](./CODE_STANDARDS.md)
- **Project Spec**: [SPEC.md](./SPEC.md)
- **Operator Guide**: [USAGE.md](./USAGE.md)
- **Project README**: [README.md](./README.md)

---

## Development Setup

### Prerequisites

Elda is a Rust workspace. You will need:

- **Rust 1.94+ stable** (see `rust-version` in the workspace `Cargo.toml`)
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustc --version  # Verify stable, not nightly
  ```
- **Cargo** (comes with Rust)
- **Git** on `PATH` (required for many workspace tests and git-source flows)

### Optional Host Tools

The full workspace test suite may invoke host build tools when integration tests exercise those lanes. You do not need every tool unless you are working on or running tests for that area:

- **zig**, **nimble**, **make**, **sh** — build-system integration tests
- **pacman**, **portage**, **xbps**, or other distro tooling — only when working on interbuild or migration adapters for that backend
- **GPG** / signing material — publish and trust workflows

For day-to-day CLI work, building `elda-cli` and running disposable-root tests in `elda-core` is usually enough.

### Fork and Clone

**For external contributors (most people):**

1. **Fork the repository** on GitHub:
   - Go to https://github.com/Mjoyufull/Elda
   - Click **Fork**

2. **Clone your fork**:
   ```sh
   git clone https://github.com/YOUR_USERNAME/Elda.git
   cd Elda
   git remote add upstream https://github.com/Mjoyufull/Elda.git
   ```

3. **Keep your fork up to date** (before starting new work):
   ```sh
   git fetch upstream
   git checkout dev
   git merge upstream/dev
   git push origin dev
   ```

**For maintainers with write access:**

```sh
git clone https://github.com/Mjoyufull/Elda.git
cd Elda
```

### Build

```sh
# Debug build (faster iteration)
cargo build -p elda-cli

# Release build
cargo build -p elda-cli --release

# Run from the workspace
cargo run -p elda-cli -- --help
cargo run -p elda-cli -- version
```

Installed binary name is `elda` (from `elda-cli`).

### Development Build

```sh
cargo run -p elda-cli -- <subcommand> [args]

# Optional: rebuild on change
cargo install cargo-watch
cargo watch -x 'run -p elda-cli -- --help'
```

---

## Project Structure

Elda is a Cargo workspace. The CLI entry point is `elda-cli`; most project logic lives in `elda-core`.

```
crates/
├── elda-cli/          # CLI parsing, dispatch, help rendering
├── elda-core/         # App orchestration, install/upgrade, render, tests
├── elda-recipe/       # Recipe parse, check, import
├── elda-install/      # Install transactions, system backend, triggers
├── elda-build/        # Builds, archives, interbuild backends
├── elda-git/          # Git sources, tags, release assets
├── elda-repo/         # Repository and index handling
├── elda-db/           # SQLite store
├── elda-fetch/        # Remote fetch
├── elda-populate/     # Populate helpers
├── elda-appimage/     # AppImage inspect/integration
├── elda-linux/        # Linux-specific helpers
├── elda-unix/         # Unix portability
├── elda-ext/          # Extension hooks
└── elda-types/        # Shared types
xtask/                 # Maintainer tasks
examples/              # Recipes, config samples, fixtures
man/elda.1             # Man page source
config.toml            # Annotated example configuration
```

When adding behavior, put types and shared logic in the smallest crate that owns the domain, keep `elda-cli` thin, and prefer disposable-root tests in `elda-core` for install/state/repo flows.

---

## How to Contribute

### Code Contributions

- Fix bugs listed in [issues](https://github.com/Mjoyufull/Elda/issues)
- Implement features that fit [SPEC.md](./SPEC.md)
- Improve performance, error messages, or operator output
- Refactor for clarity while keeping behavior stable

### Non-Code Contributions

- Improve documentation (see [Documentation Changes](#documentation-changes))
- Add or fix examples under [examples/](./examples/)
- Test releases and report issues
- Improve packaging or distro integration notes

### Documentation Changes

**Docs go to `main`.** Documentation-only changes (typos, grammar, operator clarifications, example fixes) use a branch from **`main`** and a PR **targeting `main`** — not `dev`. After merge to `main`, a maintainer merges `main` into `dev` so `dev` stays in sync.

**Criteria for docs-only:**

- Changes only to public operator doc paths (`README.md`, `USAGE.md`, `SPEC.md`, `CONTRIBUTING.md`, `PROJECT_STANDARDS.md`, `CODE_STANDARDS.md`, `checklist.md`, `phase.md`, `examples/`, `eldaforgehosting/`, `man/`, `fixtures/`, `assets/`, and related root files listed in the repo) or other doc assets
- No source or config changes that affect runtime behavior

**Process (contributor):**

```bash
git fetch upstream
git checkout main
git merge upstream/main
git checkout -b docs/fix-usage-install-loop
# Make documentation changes
git add -A && git commit -m "docs: clarify install review flow in USAGE"
git push origin docs/fix-usage-install-loop
# Open PR targeting main (not dev)
```

**User-visible code changes** (`feat/*`, `fix/*`) must include matching operator docs in the **same PR to `dev`** (typically `USAGE.md`, `man/elda.1`, and `SPEC.md` when the contract changes). See [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md).

---

## Branching Strategy

**IMPORTANT**: Never push code directly to `main` or `dev`. Code goes through PRs to `dev`; docs go through PRs to `main` (or maintainer push to `main` for trivial docs).

| Branch | Purpose |
|--------|---------|
| **main** | Tagged releases and living docs. Code reaches `main` only via release or hotfix branches. |
| **dev** | Integration branch for all code work. |

| Type | Naming | Base → Target |
|------|--------|----------------|
| Feature | `feat/name` | `dev` → `dev` |
| Fix | `fix/name` | `dev` → `dev` |
| Refactor | `refactor/name` | `dev` → `dev` |
| Docs | `docs/name` | `main` → `main` |
| Chore | `chore/name` | `dev` → `dev` |
| Release | `release/0.1.50-Sumomo` | maintainer only |

Full workflow detail: [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md).

### Standard Workflow (code)

```sh
git fetch upstream
git checkout dev
git merge upstream/dev
git checkout -b feat/your-feature-name
# develop, commit
git fetch upstream
git rebase upstream/dev
git push origin feat/your-feature-name
# Open PR: base Mjoyufull/Elda dev
# Enable "Allow edits by maintainers"
```

---

## Commit Standards

Follow **Conventional Commits**:

```
type(optional-scope): short description

[optional body]

[optional footer]
```

| Type | Meaning |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `chore` | Build, deps, tooling |
| `test` | Testing only |
| `style` | Whitespace, formatting |
| `revert` | Undo a commit |

Examples:

```sh
feat(install): add interbuild review stamp persistence
fix(solver): reject cyclic replaces edges
docs(usage): document elda sync remote selection
chore: bump workspace to 0.1.50-Sumomo
```

During development, local `wip:` commits are fine. Clean up with `git rebase -i` before opening a PR.

---

## Pull Request Process

### Before Submitting (code PRs)

```sh
git fetch upstream
git rebase upstream/dev

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p elda-cli --release
```

For documentation PRs to `main`, rebase on `upstream/main`; cargo checks are not required for docs-only changes.

### Opening a PR

- **Code**: base **`dev`**, compare your feature branch
- **Docs**: base **`main`**, compare your docs branch
- Enable **Allow edits by maintainers**
- Use a clear conventional-commit title

### PR Template

**Title**: `feat: short description`

**Body**:

```markdown
## Summary
Brief description of what this PR does and why.

- [ ] I ran fmt, clippy, and relevant tests
- [ ] User-facing impact assessed (if yes, docs updated in this PR)

## Changes
- ...

## Testing
1. cargo test -p elda-core -- <test_name>   # or full workspace
2. cargo run -p elda-cli -- <command> ...
3. Disposable-root / install path exercised (if applicable)

## Breaking Changes
None

## Related Issues
Closes #42
```

### PR Guidelines

- One focused change per PR when possible
- Respond to review feedback promptly
- Draft PRs are welcome for early feedback (still target `dev` or `main` correctly)

---

## Code Review

### What to Expect

- Initial response: hours to a few days
- Full review: typically within a week
- Merge after approval: usually within a few days

### Review Criteria

| Aspect | Expectation |
|--------|-------------|
| Correctness | Matches SPEC and stated behavior |
| Clarity | Another contributor can follow the change |
| Impact | No regressions in covered paths |
| Style | Matches [CODE_STANDARDS.md](./CODE_STANDARDS.md) |
| Documentation | Updated when operators would notice |

### Stale PRs

Inactive PRs may be marked stale after **30 days** and closed after **14** more days unless marked WIP by a maintainer. Closed PRs can be reopened.

---

## Testing

### Running Tests

```sh
# Full workspace (git required on PATH)
cargo test --workspace

# Single crate
cargo test -p elda-core

# One test with output
cargo test -p elda-core test_name -- --nocapture

RUST_BACKTRACE=1 cargo test -p elda-core test_name
```

Install, upgrade, profile, state, and repo behavior should use disposable-root tests in `elda-core` when possible (see [CODE_STANDARDS.md](./CODE_STANDARDS.md)).

### Manual CLI Checks

Before submitting operator-facing changes:

1. `cargo run -p elda-cli -- version` / `elda -V`
2. Help for the subcommand you touched (`--help`)
3. Human output (default) and `--json` if the command supports machine output
4. Dry-run / review paths for install or upgrade changes
5. `elda doctor` when changing bootstrap, remotes, or backend health

---

## Coding Standards

Rust style, module size, error handling, and testing depth are defined in **[CODE_STANDARDS.md](./CODE_STANDARDS.md)**.

Minimum before push:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

The workspace forbids `unsafe` and denies several Clippy lints (see root `Cargo.toml` `[workspace.lints]`).

---

## Reporting Bugs

Open an [issue](https://github.com/Mjoyufull/Elda/issues/new) with:

```markdown
**Description**
What went wrong.

**To Reproduce**
1. ...
2. ...

**Expected / Actual**
What you expected vs what happened.

**Environment**
- Elda version: (`elda version` or `elda -V`)
- OS / libc:
- Rust version: (`rustc --version`)
- Privilege mode used (user / sudo / etc.)

**Config**
Relevant `/etc/elda` or recipe snippets (redact secrets).

**Logs / output**
Paste framed CLI output or `--json` payload if applicable.
```

---

## Suggesting Features

1. Search existing [issues](https://github.com/Mjoyufull/Elda/issues)
2. Confirm the idea fits [SPEC.md](./SPEC.md) scope
3. Open a feature issue with use case, proposed UX, and alternatives considered

Large features should be discussed before a large PR lands.

---

## Release Process

**Maintainers only.** Contributors do not bump release versions.

Summary (detail in [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md)):

1. Merge `main` into `dev` so docs are current
2. Create `release/<version>-<codename>` from `dev`
3. Bump version in workspace `Cargo.toml`, `man/elda.1`, `phase.md` (header + §9 Changelog), and README if needed
4. Run `cargo test --workspace` and release build on the release branch
5. Merge release branch to `main`, tag **version number only** (e.g. `0.1.50`), publish GitHub release with title `[0.1.50-Sumomo]`
6. Merge release branch back to `dev`, delete release branch

**Codename (current line):** `Sumomo` applies to all `0.1.x` releases until a MAJOR bump chooses a new codename.

---

## What Not To Do

### Forbidden

- Push code directly to `main` or `dev`
- Merge code without a PR
- Ship user-visible behavior without docs in the same PR (for `feat`/`fix` to `dev`)
- Skip `cargo fmt` / `cargo clippy` before code PRs
- Reference internal planning docs (audits, gap logs, fork notes) from public operator markdown

### Strongly Discouraged

- Giant drive-by refactors mixed with feature work
- Merging with failing tests
- Ignoring Clippy warnings
- Silent SPEC drift (change SPEC when the contract changes)

---

## Getting Help

- **Issues** — bugs and feature requests
- **PR comments** — questions about a specific change

**Q: Typo in README only?**  
A: Branch from `main`, PR to `main`.

**Q: New CLI flag?**  
A: Branch from `dev`, PR to `dev`, update `USAGE.md` and usually `man/elda.1`; update `SPEC.md` if the contract changes.

**Q: Who picks release codenames?**  
A: Maintainers, on MAJOR version bumps. Current `0.1.x` line uses **Sumomo**.

---

## License

By contributing to Elda, you agree that your contributions will be licensed under the **AGPL-3.0-or-later** license, the same license as the project.

See [LICENSE](./LICENSE).

---

## Additional Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) — branching, releases, review policy
- [CODE_STANDARDS.md](./CODE_STANDARDS.md) — Rust implementation standards
- [SPEC.md](./SPEC.md) — product contract
- [USAGE.md](./USAGE.md) — operator workflows

**Questions?** Open an [issue](https://github.com/Mjoyufull/Elda/issues) 
Thank you for contributing to Elda.
