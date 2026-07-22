# Release cycle

**cargo-runner** keeps one shared semver for:

| Artifact | Version source |
|----------|----------------|
| `cargo-runner-core` / `cargo-runner-cli` (crates.io) | `Cargo.toml` workspace `version` |
| GitHub CLI binaries | tag `cargo-runner-cli-v{VERSION}` |
| VS Code extension — Marketplace **and** Open VSX | `extensions/vscode/package.json` → `version` |

The extension downloads **`cargo-runner-cli-v{extensionVersion}`**, so the numbers must stay aligned when you ship both.

The extension ships to **two** registries under publisher/namespace `masterustacean`:
Marketplace (`masterustacean.cargo-runner`) and [Open VSX](https://open-vsx.org/extension/masterustacean/cargo-runner)
(namespace is **verified**). Open VSX is what Cursor, VSCodium, Windsurf, and Gitpod install from.

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

- **CLI-only patch:** ship GitHub tag + crates.io. The extension publish step self-skips
  when the version is already live on a registry, so a CLI-only release is a clean no-op there.
- **Docs / README visibility on the registries:** bump the shared version — the VSIX embeds the new README.
- Users download `cargo-runner-cli-v{extensionVersion}` — keep tag and extension version in lockstep when shipping both.

---

## Automated publishing

Pushing a `cargo-runner-cli-v*` tag runs [`release.yml`](../.github/workflows/release.yml):

```text
build (5 targets) → create-release (attaches CLI binaries)
                  → publish-extension  (Marketplace + Open VSX + attach VSIX)
```

`publish-extension` needs `create-release`, so the CLI binaries are always on the release
**before** the extension goes live — otherwise a fresh install's first "Download CLI" would 404.

It also:

- fails fast if the tag version and `package.json` version disagree;
- queries both registries first and skips whichever already has that version.

**Required repo secrets** — Settings → Secrets and variables → Actions:

| Secret | Where it comes from |
|--------|---------------------|
| `VSCE_PAT` | [dev.azure.com](https://dev.azure.com) → User settings → Personal access tokens. Organization **must** be *All accessible organizations*; scope **Custom defined → Marketplace → Manage**. |
| `OVSX_PAT` | [open-vsx.org](https://open-vsx.org) → user settings → Access Tokens (requires a signed Eclipse Publisher Agreement). |

> ⚠️ **Azure DevOps global PATs retire 2026-12-01.** The replacement is Entra ID workload
> identity federation (`vsce publish --azure-credential`), but it currently errors with
> "corporate credentials required" for personally-owned publishers. Revisit before December.
> Open VSX has no equivalent deadline.

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
6. Optionally `vsce publish` (`--marketplace`) — **normally unnecessary**: pushing the tag
   already publishes to both registries via CI. Keep this as a manual escape hatch only.

**Makefile:** `make release-cli` · `make release-vscode` · `make release VERSION=2.1.1`

---

## Checklist

### Patch (`release.sh cli`)

- [ ] CHANGELOG entry
- [ ] Tests green
- [ ] Tag + GitHub Actions release green
- [ ] crates.io shows new version
- [ ] Smoke: `cargo binstall cargo-runner-cli` or Download CLI

### Minor (`release.sh vscode`)

- [ ] CHANGELOG entry
- [ ] Extension builds: `make vscode-package`
- [ ] CLI tag + crates.io
- [ ] `publish-extension` job green
- [ ] Marketplace shows version; id `masterustacean.cargo-runner`
- [ ] Open VSX shows version: `curl -s https://open-vsx.org/api/masterustacean/cargo-runner | grep -o '"version":"[^"]*"' | head -1`

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
