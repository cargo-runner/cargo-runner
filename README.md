# Cargo Runner

The core build engine for the `cargo-runner` project. Handles command generation, build-system detection, framework dispatch, and per-function override resolution for Cargo, Bazel, Rustc, single-file-script targets, and custom frameworks like Dioxus, Leptos, and Tauri.

---

## Installation

The recommended way to install is via `cargo-binstall` to download pre-compiled binaries directly from GitHub Releases:

```bash
cargo binstall cargo-runner-cli
```

Alternatively, you can build from source:

```bash
cargo install cargo-runner-cli
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

## Scoped Execution

`cargo runner run path/to/file.rs:25` detects the surrounding project context and automatically builds the correct compilation and execution command.

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

**Rust-Script Example:**
```rust
#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! ```
fn main() { println!("rust-script execution!"); }
```

### 3. Cargo Projects

Inside a standard Cargo workspace, execution paths are mapped intelligently:

| What you cursor into | Cargo generates |
|---------------------|-----------------|
| `#[test] fn test_add()` | `cargo test test_add --exact` |
| `mod tests { }` block | `cargo test tests::` |
| `/// ``` doctest` | `cargo test --doc add` |
| `fn main()` binary | `cargo run --bin name` |
| Benchmark function | `cargo bench name` |

### 4. Bazel Projects

In a Bazel workspace, the runner maps standard Rust layout conventions to their exact Bazel targets so you don't have to think about `//:labels`:

| What you cursor into | Bazel generates |
|---------------------|-----------------|
| `#[test] fn test_add()` | `bazel test //:unit_tests --test_arg="test_add"` |
| `mod tests { }` block | `bazel test //:unit_tests --test_arg="tests::"` |
| `/// ``` doctest` | `bazel test //:doc_tests` |
| `fn main()` binary | `bazel run //:name` |
| Benchmark function | `bazel run //:bench_name -c opt` |

### 5. Dioxus and Leptos (Custom Frameworks)

When working inside a frontend framework, `cargo-runner` hands off execution to the native CLI orchestrator required to run the WebAssembly bundling and hot-reloading dev servers.

- **Dioxus:** Automatically invokes `dx serve` or `dx build`.
- **Leptos:** Automatically invokes `cargo leptos watch` or `cargo leptos build`.
- **Tauri:** Automatically invokes the `cargo tauri` developer environment.

---

### Advanced Routing

The runner also accepts a raw module path when you already know the Rust module name instead of the file path:

```bash
cargo runner run runners::unified_runner::tests
cargo runner runnables runners::unified_runner::tests
cargo runner context runners::unified_runner::tests --json
```

When the input is not an existing file, `cargo runner` scans the current workspace members, matches the runnable `module_path`, and resolves the owning file automatically.

---

## Target Inference

`init --bazel` uses a **combined** strategy for discovering Bazel targets:

| Source | Strategy | Target generated |
|--------|----------|-----------------|
| `src/lib.rs` | Always | `rust_library` + `rust_test` (unit tests) + `rust_doc_test` |
| `src/main.rs` | Always | `rust_binary` |
| `src/bin/*.rs` | Only if `fn main()` present | `rust_binary` per file |
| `src/bin/*/main.rs` | Subdirectory binaries | `rust_binary` per dir |
| `tests/*.rs` | Always (harness provides entry) | `rust_test_suite` |
| `examples/*.rs` | Only if `fn main()` present | `rust_binary` |
| `benches/*.rs` | Only if `fn main()` present | `rust_binary` |
| `build.rs` | Always | `cargo_build_script` + warning |
| `Cargo.toml` `[[bin]]` | Explicit definitions win | `rust_binary` per entry |
| `Cargo.toml` `[[test]]` | Explicit definitions | `rust_test_suite` |
| `Cargo.toml` `[[bench]]` | Explicit definitions | `rust_binary` per entry |
| `Cargo.toml` `[[example]]` | Explicit definitions | `rust_binary` per entry |

**Priority**: Explicit `Cargo.toml` definitions always win over filesystem convention.

**`fn main()` heuristic**: Files in `src/bin/` and `examples/` are only scaffolded as binaries if they contain `fn main()` — helper modules are silently skipped.

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
| `cargo runner watch` | Context-aware file watcher: `ibazel` (Bazel) or `cargo watch` (Cargo) |
| `cargo runner run <file\|module::path>[:<line>]` | Scope-based execution: detects build system and runs the target at the given line or module path |
| `cargo runner runnables [file\|module::path[:line]] [--bin] [--test] [--bench] [--doc] [--name QUERY] [--symbol SYMBOL] [--exact]` | List runnable items for a file, module path, or entire workspace |
| `cargo runner context [file\|module::path[:line]] --json` | Emit machine-readable project/file context for TMP and other tooling |

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

## Waz Integration

If you use `waz`, the same lookup model is available there too:

```bash
waz run src/main.rs:25
waz run runners::unified_runner::tests
waz runnables
waz runnables runners::unified_runner::tests
```

`waz run` is the non-interactive path; it reuses the same project and
module-path resolution so you can skip the TUI when you already know what you
want to run.

`waz runnables` is the companion listing command when you want to inspect the
available run targets first, either for the whole workspace or for a specific
module path.

`--bin`, `--test`, `--bench`, and `--doc` narrow the result set by runnable
kind. `--name` does a case-insensitive, punctuation-insensitive substring
match against the label, module path, and function name. Add `--exact` to
require the normalized name to match exactly instead of by substring, so
`foo bar`, `foo_bar`, and `FooBar` still collapse to the same search key but
`foo` will no longer match `foobar` when `--exact` is present.

`--symbol` filters symbol-like targets, such as doc-tested structs, enums,
unions, module test groups, and binary names. It can be combined with `--name`
and the kind filters.

`cargo runner run` accepts the same selector styles:

- bare function or method name: `cargo runner run test_helper`
- full module path plus function: `cargo runner run runners::unified_runner::tests::test_helper`
- doc-test symbol: `cargo runner run Users`

---

## License

MIT or Apache-2.0, at your option.
