# Release cycle

**cargo-runner** keeps one shared semver for:

| Artifact | Version source |
|----------|----------------|
| `cargo-runner-core` / `cargo-runner-cli` (crates.io) | `Cargo.toml` workspace `version` |
| GitHub CLI binaries | tag `cargo-runner-cli-v{VERSION}` |
| VS Code extension | `extensions/vscode/package.json` → `version` |

The extension downloads **`cargo-runner-cli-v{extensionVersion}`**, so the numbers must stay aligned when you ship both.

---

## Bump policy

Use **semver** with this product rule:

| What changed | Bump | Example |
|--------------|------|---------|
| **VS Code extension** features/UX/UI (or any user-facing product release that includes the extension) | **minor** (patch → `0`) | `1.6.3` → **`1.7.0`** |
| **CLI / core only** (engine, plugins, overrides, doctests, doctor, …) | **patch** | `1.7.0` → **`1.7.1`** |
| Breaking API / protocol / remove features | **major** | `1.7.1` → **`2.0.0`** |

Examples (same idea as `0.1` → `0.2` for product, then `0.2.1` for CLI fixes):

```text
1.6.3   current
1.7.0   new VS Code UI + maybe CLI  → publish extension + CLI tag + crates.io
1.7.1   CLI-only Spin/Leptos fix    → CLI tag + crates.io  (extension marketplace optional)
1.7.2   another CLI-only fix
1.8.0   next VS Code feature drop
```

### CLI-only vs Marketplace

- **CLI-only patch:** always ship GitHub tag + crates.io. You do **not** have to publish a new VSIX if the extension code did not change.
- **Caveat:** Marketplace users still have extension version `X.Y.Z` and download `cargo-runner-cli-vX.Y.Z`.  
  - Prefer **bumping the shared version (patch)** so the next “Download CLI” after a git-synced build matches, **or** re-upload binaries under the same tag only when necessary (crates.io cannot overwrite a version).  
  - Cleanest CLI-only path: **`./scripts/release.sh cli`** (patch) → users update via `cargo binstall` / PATH, and the extension can re-download when its version is bumped or when Download CLI falls back to **latest** if the exact tag is missing.

---

## Script: `./scripts/release.sh`

```bash
# CLI / core only → patch (1.6.3 → 1.6.4)
./scripts/release.sh cli

# VS Code / product surface → minor (1.6.3 → 1.7.0)
./scripts/release.sh vscode

# Breaking → major (1.6.3 → 2.0.0)
./scripts/release.sh major

# Explicit version
./scripts/release.sh 1.7.0

# Options
./scripts/release.sh cli --no-crates      # skip crates.io
./scripts/release.sh vscode --marketplace # also vsce publish
./scripts/release.sh cli --dry-run        # print plan only
./scripts/release.sh cli --no-push        # commit/tag locally only
```

What the script does (unless dry-run):

1. Bumps `Cargo.toml` workspace version, `crates/cli` core dep, `extensions/vscode/package.json`
2. Appends a stub section to `CHANGELOG.md` if missing for that version
3. Commits + pushes `main` (unless `--no-push`)
4. Creates annotated tag `cargo-runner-cli-v{VERSION}` and pushes it (triggers multi-arch GitHub Release)
5. Publishes `cargo-runner-core` then `cargo-runner-cli` to crates.io (unless `--no-crates` or version already published)
6. Optionally `vsce publish` for the extension (`--marketplace`)

**Makefile:**

```bash
make release-cli
make release-vscode
make release VERSION=1.7.0
```

---

## Checklist (humans)

### CLI-only (`release.sh cli`)

- [ ] CHANGELOG entry under the new patch
- [ ] Tests / clippy green
- [ ] Tag + GitHub Actions release green
- [ ] crates.io `cargo-runner-cli` shows new version
- [ ] Smoke: `cargo binstall cargo-runner-cli` or Download CLI

### VS Code / product (`release.sh vscode`)

- [ ] CHANGELOG entry under the new minor
- [ ] Extension builds: `make vscode-package`
- [ ] CLI tag + crates.io as above
- [ ] `vsce publish` (or `./scripts/release.sh vscode --marketplace`)
- [ ] Marketplace page shows new version; extension id `masterustacean.cargo-runner`

### Never

- Do not re-publish the same version on **crates.io**
- Do not bump major for routine CLI fixes
- Do not ship extension-only version that cannot resolve a matching `cargo-runner-cli-v*` tag (unless Download CLI is intentionally set to “latest”)

---

## Version sources of truth

```text
Cargo.toml                    [workspace.package] version = "X.Y.Z"
crates/cli/Cargo.toml         cargo-runner-core = { version = "X.Y.Z", path = "..." }
extensions/vscode/package.json  "version": "X.Y.Z"
GitHub tag                    cargo-runner-cli-vX.Y.Z
```
