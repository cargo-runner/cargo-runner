# Changelog

All notable changes to **cargo-runner** (CLI, core library, and VS Code extension) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

CLI and VS Code extension share the same version. Prebuilt CLI binaries are
published on GitHub as tag `cargo-runner-cli-v{VERSION}`.

---

## [1.6.3] — 2026-07-12

### Added

- **VS Code extension** (`masterustacean.cargo-runner`): run/override UX, CodeLens, trees, auto-download of matching CLI from GitHub Releases.
- **IDE protocol** (`docs/ide-protocol.md`): `runnables --json`, `run --dry-run --json`, `context --json`, override list/show JSON; structured error objects for IDE modes.
- **`cargo runner doctor [--json]`** — project + toolchain health checks (Cargo, Bazel, nextest, frameworks).
- **`cargo runner completions {bash,zsh,fish,…}`** — shell completions via clap_complete.
- **`cargo runner override --examples`** — override token cookbook.
- **`run` flags:** `--features`, `--all-features`, `--no-default-features`, `--release`, `--package`, `--nextest` / `--no-nextest`.
- **`run … -- <args>`** — passthrough (e.g. `--nocapture` after cargo’s `--` for tests).
- **`run --quiet`** and global **`--quiet` / `--no-emoji`** (plus env `CARGO_RUNNER_QUIET`, `CARGO_RUNNER_NO_EMOJI` / `NO_EMOJI`).
- **Watch replay:** `watch path.rs:LINE` re-runs the same command `run` would resolve (not only project-level build).
- **Scoped doctests:** fenced examples in `///`, `//!`, `/** */` on fn/struct/enum/mod/union/**trait**/impl; skip `ignore` / `no_run` / `compile_fail`; crate-relative `cargo test --doc` filters.
- **Framework overlays:** Dioxus, Leptos, Tauri detection and native CLI handoff.
- **Docs:** `docs/limitations.md`, IDE protocol warnings for Bazel doctest limits.

### Fixed

- Override application / identity matching (including Windows absolute path tests).
- Doctest filters no longer prefix the Cargo package name (which matched **zero** rustdoc tests).
- `--json` without `--dry-run` on `run` is rejected (no silent no-op).

### Changed

- Marketplace publisher: **`masterustacean`** (extension id `masterustacean.cargo-runner`).
- `runnables` / analyze implementation split into `filters` / `print_json` / `print_human` modules.

### Packaging

- GitHub Release `cargo-runner-cli-v1.6.3` (linux/mac/windows multi-arch).
- crates.io: `cargo-runner-core` + `cargo-runner-cli` 1.6.3.
- VS Code Marketplace: 1.6.3.

---

## [1.6.2] — 2026-07-12

### Added

- Initial multi-arch GitHub CLI release under the modern monorepo layout.
- VS Code extension scaffolding and IDE-facing CLI contracts (landed with the extension PR).

### Fixed

- Windows path identity matching for relative vs absolute override paths.

---

## [1.0.0] — 2026-04-15

### Added

- First crates.io release of `cargo-runner-core` and `cargo-runner-cli`.
- Cargo / Bazel / rustc / single-file script runners and override configuration.

---

## Links

- Repo: https://github.com/cargo-runner/cargo-runner
- CLI releases: https://github.com/cargo-runner/cargo-runner/releases
- Marketplace: https://marketplace.visualstudio.com/items?itemName=masterustacean.cargo-runner
- crates.io: https://crates.io/crates/cargo-runner-cli
