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

| What changed | Bump | Example |
|--------------|------|---------|
| **VS Code** features / UX / product surface | **minor** (patch → `0`) | `2.1.0` → **`2.2.0`** |
| **CLI / core only**, or **docs-only** re-publish so crates.io + marketplace show new README | **patch** | `2.1.0` → **`2.1.1`** |
| Breaking API / protocol | **major** | `2.1.1` → **`3.0.0`** |

```text
2.1.0   product (Agent Init + agent-init CLI)
2.1.1   docs polish / small CLI fixes  → tag + crates.io + marketplace if README should update there
2.2.0   next VS Code feature drop
```

### CLI-only vs Marketplace

- **CLI-only patch:** ship GitHub tag + crates.io. Marketplace optional if extension code did not change.
- **Docs / README visibility on Marketplace:** bump shared version and publish with `--marketplace` so the VSIX embeds the new README.
- Marketplace users download `cargo-runner-cli-v{extensionVersion}` — keep tag and extension version in lockstep when shipping both.

---

## Script: `./scripts/release.sh`

```bash
# CLI / core / docs patch
./scripts/release.sh cli

# VS Code / product surface → minor
./scripts/release.sh vscode

# Breaking → major
./scripts/release.sh major

# Explicit version
./scripts/release.sh 2.1.1

# Options
./scripts/release.sh cli --no-crates
./scripts/release.sh cli --marketplace   # also vsce publish
./scripts/release.sh vscode --marketplace
./scripts/release.sh cli --dry-run
./scripts/release.sh cli --no-push
```

What the script does (unless dry-run):

1. Bumps workspace version, CLI core dep pin, VS Code `package.json`
2. Appends a CHANGELOG stub if missing for that version
3. Commits + pushes `main` (unless `--no-push`)
4. Creates annotated tag `cargo-runner-cli-v{VERSION}` and pushes it
5. Publishes `cargo-runner-core` then `cargo-runner-cli` to crates.io
6. Optionally `vsce publish` (`--marketplace`)

**Makefile:** `make release-cli` · `make release-vscode` · `make release VERSION=2.1.1`

---

## Checklist

### Patch (`release.sh cli`)

- [ ] CHANGELOG entry
- [ ] Tests green
- [ ] Tag + GitHub Actions release green
- [ ] crates.io shows new version
- [ ] Smoke: `cargo binstall cargo-runner-cli` or Download CLI
- [ ] If README should appear on Marketplace: pass `--marketplace`

### Minor (`release.sh vscode`)

- [ ] CHANGELOG entry
- [ ] Extension builds: `make vscode-package`
- [ ] CLI tag + crates.io
- [ ] `vsce publish` / `--marketplace`
- [ ] Marketplace shows version; id `masterustacean.cargo-runner`

### Never

- Do not re-publish the same version on **crates.io**
- Do not bump major for routine fixes
- Do not ship extension-only version without a matching `cargo-runner-cli-v*` tag (unless Download CLI intentionally uses latest)

---

## Agent instructions installer

```bash
# VS Code: Command Palette → "Cargo Runner: Agent Init"
cargo runner agent-init --root ~/Code/my-app
cargo runner agent-init --dry-run
cargo runner agent-init AGENTS.md CLAUDE.md

# Optional script (same behavior):
./scripts/install-agent-instructions.sh --root ~/Code/my-app
```

See [AGENTS.cargo-runner.md](./AGENTS.cargo-runner.md). Symlinks are followed and **deduped**.

---

## Version sources of truth

```text
Cargo.toml                      [workspace.package] version = "X.Y.Z"
crates/cli/Cargo.toml           cargo-runner-core = { version = "X.Y.Z", path = "..." }
extensions/vscode/package.json  "version": "X.Y.Z"
GitHub tag                      cargo-runner-cli-vX.Y.Z
```
