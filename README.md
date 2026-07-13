# Cargo Runner

<p align="center">
  <img
    src="https://raw.githubusercontent.com/cargo-runner/cargo-runner/main/images/agent-terminal-cargo-runner.png"
    alt="Coding agent using cargo-runner: scan runnables, dry-run, then run the correct cargo test"
    width="920"
  />
</p>

<p align="center"><em>Agent terminal: scan → dry-run → run the right test — no rust-analyzer wait, no guessed <code>cargo test</code>.</em></p>

**Run Rust without waiting for rust-analyzer** — and stop AI agents from inventing the wrong `cargo test` every chat.

Cargo Runner has its own **scope / runnables engine**. Point it at a file or `file:line` and it builds the right command for:

| Support | Examples |
|---------|----------|
| **Cargo** | run · test · bench · doc |
| **rustc** | standalone files |
| **Single-file scripts** | `cargo +nightly -Zscript`, `rust-script` |
| **Bazel** | run · test · bench (from source path, not hand-written labels) |
| **Frameworks** | Dioxus (`dx`), Leptos (`cargo leptos`), Tauri (`cargo tauri`) |
| **Custom tools** | Spin, make, anything — **override once**, then plain `run` forever |

CLI and VS Code extension share one version. Config lives in **`.cargo-runner.json`**.

| | |
|--|--|
| **CLI** | `cargo binstall cargo-runner-cli` / `cargo install cargo-runner-cli` |
| **VS Code** | [masterustacean.cargo-runner](https://marketplace.visualstudio.com/items?itemName=masterustacean.cargo-runner) |
| **Neovim** | `cargo runner nvim install` · [extensions/nvim](extensions/nvim/) |
| **Changelog** | [CHANGELOG.md](CHANGELOG.md) · release process [docs/release.md](docs/release.md) |
| **Agent playbook** | [docs/AGENTS.cargo-runner.md](docs/AGENTS.cargo-runner.md) |
| **IDE JSON** | [docs/ide-protocol.md](docs/ide-protocol.md) |
| **Limits** | [docs/limitations.md](docs/limitations.md) |

---

## Quick start

```bash
cargo binstall cargo-runner-cli          # or: cargo install cargo-runner-cli
cd your-rust-project
cargo runner init                        # once → .cargo-runner.json
cargo runner runnables --json --with-commands   # scan what can run
cargo runner run src/lib.rs:42           # test/doc under that line
cargo runner run src/main.rs             # binary / app entry
```

**VS Code:** install the extension → **Cmd+R** / **Ctrl+R** run at cursor · **Cmd+Shift+R** override.

**Neovim:** `cargo runner nvim install` → **`<leader>r`** run · **`<leader>R`** override (optional **Cmd+R** in Neovide/GUI).

### Custom tools (Spin, make, …)

When the default is wrong (e.g. Leptos overlay but you need Spin):

```bash
# once — bind to the entry you will keep running
cargo runner override src/main.rs -- @spin.build --up
# no main.rs? use the lib or bin path:
# cargo runner override src/lib.rs -- @make.test

# forever after — no tokens, override is automatic
cargo runner run src/main.rs
```

---

## AI / coding agents

Agents should **scan → run if supported → override once if not → plain `run` forever**. Full instructions: [docs/AGENTS.cargo-runner.md](docs/AGENTS.cargo-runner.md).

**Install those instructions into a project** (no need to copy-paste by hand):

| Method | Command |
|--------|---------|
| **VS Code** | Palette → **Cargo Runner: Agent Init** |
| **CLI** | `cargo runner agent-init` · `cargo runner agent-init --dry-run` |
| **Script** (optional) | `./scripts/install-agent-instructions.sh --root /path/to/app` |

```bash
cargo runner agent-init --root /path/to/your-rust-app
cargo runner agent-init AGENTS.md CLAUDE.md
```

Installers find `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, Cursor/Copilot paths, etc.; **follow symlinks** and **dedupe** real files; upsert a managed HTML-comment block (safe to re-run).

---

## Installation details

```bash
# Prebuilt (fast)
cargo binstall cargo-runner-cli

# From source
cargo install cargo-runner-cli
```

**VS Code** auto-downloads CLI tag `cargo-runner-cli-v{extensionVersion}` from GitHub Releases (or use PATH / `cargoRunner.path`).

**Neovim / Vim plugin** (packpath — no `init.lua` edit required):

```bash
cargo runner nvim install
cargo runner nvim status
cargo runner nvim uninstall

# if `vim` is aliased to nvim in your shell:
cargo runner vim install --follow-shell-alias

# custom config / data (NVIM_APPNAME, LazyVim, dotfiles, …)
cargo runner nvim install --app-name nvim-lazy
cargo runner nvim install --config-dir ~/dotfiles/nvim --data-home ~/dotfiles/.local/share
cargo runner nvim install --pack-dir ~/dotfiles/nvim/pack/cargo-runner/start/cargo-runner
```

| Flag | Use when |
|------|----------|
| `--config-dir` | Your `init.lua` is not `~/.config/nvim` (hints for optional `setup()`) |
| `--data-home` / `--app-name` | Non-default Neovim data / `$NVIM_APPNAME` |
| `--pack-dir` | Exact packpath directory |
| `--vim-dir` | Classic Vim root ≠ `~/.vim` |

Docs: **[docs/nvim.md](docs/nvim.md)** · status panel UX **[docs/nvim-status-panel.md](docs/nvim-status-panel.md)** · plugin sources [`extensions/nvim/`](extensions/nvim/).

### Extension development

```bash
make vscode
# F5 with extensions/vscode open, or:
make vscode-package

# Neovim plugin (packpath symlink for live edit)
cargo runner nvim install   # symlinks extensions/nvim when run from this repo
```

---

## Build System & Framework Detection

`UnifiedRunner` uses a Plugin Registry to allow overrides (framework overlays) before falling back to generic build-system detection:

```
┌─ Framework Overlays (highest priority) ───────────────────────┐
│  Dioxus.toml in ancestor dirs    →  DioxusOverlayPlugin       │
│  "leptos" in Cargo.toml          →  LeptosOverlayPlugin       │
└───────────────────────────────────────────────────────────────┘
         │ (no plugin claimed the path)
         ▼
┌─ Build system detection ──────────────────────────────────────┐
│  MODULE.bazel present            →  BazelRunner               │
│  Cargo.toml present              →  CargoRunner               │
│  (none)                          →  RustcPrimaryPlugin        │
└───────────────────────────────────────────────────────────────┘
```

### Framework vs Bazel — design boundary

Framework-managed projects (Dioxus, Leptos, Tauri) **always use their native CLI**, never Bazel. These frameworks orchestrate WASM compilation, asset bundling, hot-reload dev servers, and platform-specific builds internally — capabilities that Bazel cannot replicate.

Bazel support targets **pure Rust projects**: API servers, CLI tools, libraries, and monorepos with shared dependency graphs.

| Framework | CLI | Bazel support? |
|-----------|-----|----------------|
| Dioxus | `dx serve / dx build` | ❌ Not supported — use `dx` |
| Leptos | `cargo leptos watch / build` | ❌ Not supported — use `cargo-leptos` |
| Tauri | `cargo tauri dev / build` | ❌ Not supported — use Tauri CLI |
| Pure Rust (lib, bin, tests) | `cargo` or `bazel` | ✅ Fully supported |

---

## Implicit Execution & Target Inference

When running `cargo runner run` without explicit file arguments, the runner intelligently infers what to execute based on Cargo's `default-run` settings and standard Rust project conventions.

It uses these exact filesystem layout patterns to automatically detect binaries, tests, benchmarks, and libraries—both for resolving implicitly executed targets and for generating underlying Bazel rules:

| Rust Source Path | Inferred Target Kind | Notes / Bazel Mapping |
|------------------|----------------------|-----------------------|
| `src/main.rs` | Project Binary | The default entry point, maps to `rust_binary` |
| `src/lib.rs` | Library & Doc Tests | Maps to `rust_library` and `rust_doc_test` |
| `src/bin/*.rs` | Additional Binary | Scaffolded/detected only if `fn main()` is present |
| `src/bin/*/main.rs` | Directory Binary | Scaffolded/detected only if `fn main()` is present |
| `tests/*.rs` | Integration Test | Automatically maps to `rust_test_suite` |
| `examples/*.rs` | Example Binary | Scaffolded/detected only if `fn main()` is present |
| `benches/*.rs` | Benchmark | Scaffolded/detected only if `fn main()` is present |
| `build.rs` | Build Script | Maps to `cargo_build_script` internally |

**Priority:** Explicit `Cargo.toml` definitions (`[[bin]]`, `[[test]]`, `[[bench]]`, `[[example]]`) always take priority over the filesystem conventions above.

**`fn main()` heuristic**: Files in `src/bin/` and `examples/` are only detected as runnable binaries if they actually contain a `fn main()` function — helper modules are silently skipped.

---

## Scoped Execution

`cargo runner run <path>` detects the surrounding project context and automatically builds the correct compilation and execution command. To preview any command without executing it, simply append `--dry-run`.

```bash
cargo runner run src/lib.rs:12 --dry-run          # print command
cargo runner run src/lib.rs:12 --dry-run --json   # IDE JSON (requires --dry-run)
cargo runner run src/lib.rs:12 -- --nocapture     # forward test binary args
cargo runner run src/lib.rs:12 --features foo --release --dry-run
cargo runner run --quiet                          # real run without “Using: …” notice
cargo runner --no-emoji runnables                 # human output without decorative emoji
cargo runner watch src/lib.rs:12                  # re-run resolved command on save
cargo runner doctor                               # project + toolchain health
cargo runner override --examples                  # override cookbook
cargo runner completions zsh                      # shell completions
```

### Shell completions

```bash
# zsh (example)
cargo runner completions zsh > ~/.zfunc/_cargo-runner
# bash
cargo runner completions bash | sudo tee /etc/bash_completion.d/cargo-runner
# fish
cargo runner completions fish > ~/.config/fish/completions/cargo-runner.fish
```

Completions target the `cargo-runner` binary name. When using `cargo runner …`, Cargo’s own completion applies to the `runner` subcommand name.

See also [docs/limitations.md](docs/limitations.md) for intentional hold-offs (macro doctests, Bazel per-example docs, etc.).

It evaluates the environment in this specific priority:

### 1. Standalone Rust Files (`rustc`)

If a Rust file sits outside of any framework or build system (no `Cargo.toml`, no Bazel), Cargo Runner transparently falls back to `rustc`.

```bash
cargo runner run standalone.rs
# Generates: rustc standalone.rs -o /tmp/... && /tmp/...
```

### 2. Single-file Scripts

Cargo Runner recognizes single-file Rust scripts explicitly via their shebangs and a `fn main()` entry point.

**Cargo Nightly Script Example (`-Zscript`):**
```rust
#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2021"
---
fn main() { println!("nightly cargo script!"); }
```
```bash
cargo runner run my_script.rs
# Generates: cargo +nightly -Zscript my_script.rs
```

**Rust-Script Example:**
```rust
#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! ```
fn main() { println!("rust-script execution!"); }
```
```bash
cargo runner run my_rust_script.rs
# Generates: rust-script my_rust_script.rs
```

### 3. Cargo Projects & Workspaces

Inside a standard Cargo project or massively scaled virtual Workspace, execution paths are mapped intelligently:

```bash
cargo runner run src/main.rs
# Generates: cargo run --bin my_app

cargo runner run tests/integration.rs
# Generates: cargo test --test integration
```

| What you cursor into | Cargo generates |
|---------------------|-----------------|
| `#[test] fn test_add()` | `cargo test test_add --exact` |
| `mod tests { }` block | `cargo test tests::` |
| fenced doctest on item | `cargo test --doc <Item>` |
| `fn main()` binary | `cargo run --bin name` |
| Benchmark function | `cargo bench name` |

**Scoped doctests** run when the cursor is on a markdown-fenced example in `///`, `//!`, or `/** */` docs attached to a named item (fn, struct, enum, mod, union, impl method). Fence tags `ignore`, `no_run`, and `compile_fail` are not offered as scoped runs. Macro-generated or `#[doc = include_str!(…)]` docs are not source-scanned — use crate-level `cargo test --doc` for those.

### 4. Bazel Projects

In a Bazel workspace, the runner maps standard Rust layout conventions to their exact Bazel targets so you don't have to think about `//:labels`:

```bash
cargo runner run src/main.rs
# Generates: bazel run //:my_app

cargo runner run src/lib.rs:25 
# Generates: bazel test //:unit_tests --test_arg="tests::add"
```

| What you cursor into | Bazel generates |
|---------------------|-----------------|
| `#[test] fn test_add()` | `bazel test //:unit_tests --test_arg="test_add"` |
| `mod tests { }` block | `bazel test //:unit_tests --test_arg="tests::"` |
| fenced doctest on item | `bazel test //:doc_tests` (all crate docs) |
| `fn main()` binary | `bazel run //:name` |
| Benchmark function | `bazel run //:bench_name -c opt` |

### 5. Dioxus and Leptos (Custom Frameworks)

When working inside a frontend framework, `cargo-runner` hands off execution to the native CLI orchestrator required to run the WebAssembly bundling and hot-reloading dev servers.

```bash
cargo runner run src/main.rs
```

- **Dioxus:** Automatically invokes `dx serve` or `dx build`.
- **Leptos:** Automatically invokes `cargo leptos watch` or `cargo leptos build`.
- **Tauri:** Detects `tauri.conf.json` (or a `tauri` crate dependency) and runs `cargo tauri dev` by default. Override subcommand with e.g. `cargo runner override src/main.rs --subcommand build` → `cargo tauri build`.

---

## Tooling & Debugging Commands

Beyond executing files, Cargo Runner provides powerful introspection commands. You can pass raw module paths (like `runners::unified_runner::tests`) or standard file paths to these utilities.

### Previewing Commands (`--dry-run`)
If you aren't sure what command Cargo Runner will synthesize for a specific file or framework, use `--dry-run`. It will print the exact internal `Command` structure without spawning it:
```bash
cargo runner run src/main.rs --dry-run
```

### Exploring Targets (`runnables`)
Use `runnables` to list all valid execution targets. You can filter by `--bin`, `--test`, `--bench`, or substring matches:
```bash
# List all targets in a specific file
cargo runner runnables src/lib.rs

# Search the entire workspace for binaries
cargo runner runnables --bin

# Find a specific module block or test
cargo runner runnables runners::unified_runner::tests --exact
```

### Inspecting Context (`context`)
If you need deep JSON introspection for IDE integration (or debugging why a file resolved a certain way), the `context` command reveals exactly how Cargo Runner interpreted the environment:
```bash
cargo runner context src/main.rs --json
```
When the input is not an existing file, `cargo runner` scans the current workspace members, matches the runnable `module_path`, and resolves the owning file automatically.

---

## Bazel — One-Command Workflow

> **Goal**: Use a Bazel-managed Rust workspace as if it were plain Cargo — no manual Bazel bookkeeping.

### `cargo runner init --bazel`

This is the **single entry point** for all Bazel scaffolding. It handles both initial setup and subsequent syncs.

#### First run — scaffolds the workspace

```bash
cargo runner init --bazel
```

Generates:
- `MODULE.bazel` — bzlmod dependency graph via `crate.from_cargo()`
- `.bazelversion` — pins Bazel 7.4.1
- `.bazelrc` — build flags + shared disk/repo caches
- `BUILD.bazel` — targets for each crate (see below)
- `Cargo.lock` — required by `crate_universe`
- `.cargo-runner.json` — framework defaults

Then runs:
1. `bazel sync` — downloads toolchain + resolves crate deps
2. `bazel build --nobuild //...` — validates all BUILD files without compiling

#### Re-run — idempotent sync

```bash
cargo runner init --bazel   # safe to re-run anytime
```

Re-scans source files, adds missing targets, skips existing ones. The single command replaces the old `build-sync` workflow.

#### Workspace support

For Cargo workspaces, `init --bazel` automatically:
- Parses `[workspace] members` (supports explicit lists and globs)
- Generates per-member `BUILD.bazel` files
- Creates a unified `MODULE.bazel` at the root

### Doctests

Library crates (`src/lib.rs`) automatically get a `rust_doc_test` target:

```python
rust_doc_test(
    name = "doc_tests",
    crate = ":my_lib",
)
```

Doctests use Bazel natively. There is **no cargo fallback** — if it's a Bazel project, everything goes through Bazel.

### BUILD.bazel safety model

`build-sync` only modifies lines inside a **managed block** — anything outside the fences is left untouched:

```python
# Hand-authored rules above are NEVER touched

# BEGIN cargo-runner-managed — do not edit this block manually
rust_library(...)
rust_test(...)
rust_doc_test(...)
# END cargo-runner-managed
```

Deduplication is name-aware and content-aware:
- Exact name matches are skipped
- Any existing `rust_doc_test(` rule (regardless of name) prevents duplicate doc test targets

### Other commands

| Command | What it does |
|---------|-------------|
| `cargo runner add <crate> [--features f] [--dev]` | `cargo add` + `cargo update` + `bazel sync` + `gen_rust_project` in one shot |
| `cargo runner sync [--crate <name>] [--skip-ide]` | Sync Bazel crate-universe after any `Cargo.toml` edit |
| `cargo runner build-sync [--crate <name>] [--dry-run]` | Update `BUILD.bazel` targets (also runs as part of `init --bazel`) |
| `cargo runner clean` | Context-aware clean: `bazel clean` (Bazel) or `cargo clean` (Cargo) |
| `cargo runner watch` | Context-aware file watcher: notify + bazel run/test/build (Bazel), or cargo-watch / notify fallback (Cargo) |
| `cargo runner run <file\|module::path>[:<line>]` | Scope-based execution: detects build system and runs the target at the given line or module path |
| `cargo runner runnables [file\|module::path[:line]] [--bin] [--test] [--bench] [--doc] [--name QUERY] [--symbol SYMBOL] [--exact]` | List runnable items for a file, module path, or entire workspace |
| `cargo runner context [file\|module::path[:line]] --json` | Emit machine-readable project/file context for IDEs and agents |
| `cargo runner agent-init [PATH...]` | Install agent instructions into AGENTS.md / CLAUDE.md / Cursor / Copilot |
| `cargo runner nvim install` / `uninstall` / `status` | Neovim packpath plugin; path flags `--config-dir`, `--app-name`, `--pack-dir`, … |
| `cargo runner doctor [--json]` | Project + toolchain health checks |
| `cargo runner override …` | Persist custom commands (spin, make, env, …) in `.cargo-runner.json` |

---

## Configuration Reference

Configuration lives in `.cargo-runner.json` at your crate root.

### Top-level shape

```json
{
  "bazel": { ... },
  "cargo": { ... },
  "overrides": [ ... ]
}
```

### Bazel project config (`"bazel"`)

Used at the project level to configure how Bazel commands are built.

```json
{
  "bazel": {
    "workspace": "my_workspace",
    "test_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed"],
      "test_args": ["--nocapture", "{test_filter}"]
    },
    "binary_framework": {
      "command": "bazel",
      "subcommand": "run",
      "target": "{target}"
    },
    "benchmark_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed", "--test_arg", "--bench"],
      "test_args": ["{bench_filter}"]
    }
  }
}
```

Supported template placeholders: `{target}`, `{test_filter}`, `{bench_filter}`, `{file_name}`.

### Per-function overrides (`"overrides"`)

The `overrides` array lets you customize commands for specific functions, tests, or files. Each entry has a `"match"` key and a command-type block.

#### Cargo override

```json
{
  "match": {
    "function_name": "my_slow_test",
    "package": "server"
  },
  "cargo": {
    "extra_args": ["--test-threads=1"],
    "extra_env": { "RUST_BACKTRACE": "1", "RUSTFLAGS": "-Awarnings" }
  }
}
```

**Note on Cargo Environments**: Overrides correctly inject environment variables into `cargo` commands contextually. Due to accurate `FileType` contextual propagation, `RUSTFLAGS` or other systemic flags apply perfectly onto `cargo run`/`cargo test` invocations without mistakenly triggering standalone `rustc` fallbacks.

#### Bazel override

The `"bazel"` block inside an override is a **flat `BazelOverride`** — fields are promoted from the framework level directly to the override. You no longer need to nest `test_framework.test_args`.

```json
{
  "match": {
    "function_name": "it_works_too",
    "module_path": "tests",
    "package": "frontend"
  },
  "bazel": {
    "test_args": ["--nocapture"]
  }
}
```

All `BazelOverride` fields:

| Field | Type | Description |
|-------|------|-------------|
| `command` | `string` | Override the Bazel binary (e.g. `"bazelisk"`). Display/tooling only — not yet applied at runtime. |
| `subcommand` | `string` | Override the subcommand (`"test"`, `"run"`, `"build"`). |
| `target` | `string` | Override the target label. |
| `args` | `string[]` | Replace the base args block (after subcommand + target). |
| `extra_args` | `string[]` | Append verbatim args after the base args. |
| `test_args` | `string[]` | Inject as `--test_arg <value>` pairs. |
| `exec_args` | `string[]` | Append after `--` separator (for `bazel run`). |
| `extra_env` | `object` | Merge extra environment variables. |

> **Migration note**: The previous nested form `"bazel": { "test_framework": { "test_args": [...] } }` inside overrides **is no longer valid**. Update to the flat shape above.

### Override CLI

Use `cargo runner override` to create overrides from the command line instead of editing JSON manually.

#### Named flags

```bash
cargo runner override <filepath> --command <cmd> --subcommand <sub> --channel <ch>
```

| Flag | What it sets |
|------|-------------|
| `--command` | The command binary (`dx`, `cargo`, `bazel`) |
| `--subcommand` | The subcommand (`serve`, `run`, `build`, `watch`) |
| `--channel` | Rust toolchain channel (`nightly`, `stable`) |

#### Token syntax (after `--`)

```bash
cargo runner override <filepath> -- <tokens...>
```

| Token | Effect |
|-------|--------|
| `@cmd.sub` | Set command + subcommand (e.g., `@dx.run`) |
| `+channel` | Set Rust toolchain channel (e.g., `+nightly`) |
| `KEY=value` | Set environment variable (e.g., `RUST_LOG=debug`, `RUSTFLAGS="-Awarnings"`) |
| `/args...` | Test binary args (like `--` in `cargo test`) |
| `-command` | Remove the command override |
| `-env` | Remove all env overrides |
| `-` | Remove the entire override |
| other | Appended as `extra_args` |

> **Environment Variables**: Overrides like `KEY=value` reliably populate the `extra_env` record. For Bazel targets, these are accurately transformed strictly into `--action_env=KEY=value` (and `--test_env=KEY=value` for valid subcommands). For Cargo, these correctly attach to the `Command` execution environment across nested workspace crates.

#### Dioxus examples

```bash
# Default: dx serve — override to dx run
cargo runner override src/main.rs --command dx --subcommand run

# Same using token syntax
cargo runner override src/main.rs -- @dx.run

# Add --release flag
cargo runner override src/main.rs -- @dx.serve --release

# Add env vars
cargo runner override src/main.rs -- @dx.serve RUST_LOG=debug
```

#### Leptos examples

```bash
# Default: cargo leptos serve — override to cargo leptos watch
cargo runner override src/main.rs --subcommand watch

# Same using token syntax (all three forms are equivalent)
cargo runner override src/main.rs -- @cargo.watch
cargo runner override src/main.rs -- @cargo.leptos.watch

# Add --release
cargo runner override src/main.rs --subcommand build -- --release
```

> **Note**: For Leptos, you only set `--subcommand` (not `--command`) because the command is always `cargo` — the runner constructs `cargo leptos <subcommand>` automatically.

#### Test binary args (ignored tests, etc.)

The `/` token passes arguments directly to the test binary (after `--` in `cargo test`):

```bash
# Run all tests including ignored ones
cargo runner override src/lib.rs -- /--include-ignored

# Run only ignored tests
cargo runner override src/lib.rs -- /--ignored

# Add multiple test binary args
cargo runner override src/lib.rs -- /--include-ignored --nocapture
```

Key distinction:
- `-- extra_args` → cargo-level flags (before `--`)
- `/ test_args` → test binary flags (after `--`)

#### Workspace-wide test binary args

To apply test binary args to **all tests in the workspace**, set `extra_test_binary_args` at the top-level `cargo` config in `.cargo-runner.json`:

```json
{
  "cargo": {
    "extra_test_binary_args": ["--include-ignored"]
  }
}
```

This applies globally without needing per-file overrides.

#### Remove an override

```bash
cargo runner override src/main.rs -- -
```

---

## Selectors & filters

`cargo runner run` accepts:

- file / line: `cargo runner run src/lib.rs:42`
- bare function or method name: `cargo runner run test_helper`
- full module path: `cargo runner run runners::unified_runner::tests::test_helper`
- doc-test symbol: `cargo runner run Users`

`runnables` filters:

| Flag | Meaning |
|------|---------|
| `--bin` / `--test` / `--bench` / `--doc` | Kind filters |
| `--name QUERY` | Case- and punctuation-insensitive substring on label / module / function |
| `--exact` | Exact normalized name match (not substring) |
| `--symbol SYMBOL` | Symbol-like targets (structs, enums, bins, module groups, …) |

---

## License

MIT or Apache-2.0, at your option.
