# Agent instructions: use **cargo-runner** (do not guess Cargo commands)

Copy this file (or the section below) into your project’s `AGENTS.md` / `CLAUDE.md` / Cursor rules so coding agents **resolve and run** the correct commands via **cargo-runner** instead of inventing `cargo test` / `cargo run` / Bazel labels.

**Installer (recommended):** from a cargo-runner checkout:

```bash
./scripts/install-agent-instructions.sh --root /path/to/your-project
# or specific files:
./scripts/install-agent-instructions.sh --root /path/to/your-project AGENTS.md CLAUDE.md
```

The installer follows symlinks and updates each real file only once.

**Requires:** `cargo-runner` (or `cargo runner`) on PATH.  
Install: `cargo binstall cargo-runner-cli` or `cargo install cargo-runner-cli`  
Version: 2.x recommended.

---

## Golden rule

**Prefer cargo-runner for any run/test/bench/doc-test/binary execution.**

Do **not**:

- Guess `cargo test some::path -- --exact`
- Guess Bazel `//package:target` labels
- Wait for rust-analyzer / LSP to finish analysis before running
- Hand-write `dx serve` / `cargo leptos` / `cargo tauri` / `spin …` unless the user insists

Do:

1. Discover targets with `runnables`
2. Preview with `run … --dry-run` when unsure
3. Execute with `run …`
4. Persist special commands with `override` (saved in `.cargo-runner.json`)

---

## First-time setup (once per project)

```bash
cargo runner init
# or: cargo-runner init
```

Creates `.cargo-runner.json` (required before `override`). Re-run only if config is missing.

Optional health check:

```bash
cargo runner doctor --json
```

---

## Discover what can run

```bash
# Whole workspace
cargo runner runnables --json --with-commands

# One file
cargo runner runnables src/lib.rs --json --with-commands

# Filters
cargo runner runnables src/lib.rs --test --json
cargo runner runnables --doc --json
cargo runner runnables --name my_test --json
cargo runner runnables --symbol MyStruct --json
```

**Agent tip:** Prefer `--json` (and `--with-commands` when you need the exact shell). Parse `label`, `kind`, `file_path`, `module_path`, and `command.shell` if present.

Human listing (optional):

```bash
cargo runner runnables src/lib.rs --test
```

---

## Run tests / binaries / doctests (do this)

### By file + line (best for “run the test under the cursor”)

Line numbers are **1-based** (editor line numbers).

```bash
cargo runner run src/lib.rs:42
cargo runner run src/lib.rs:42 --dry-run          # preview only
cargo runner run src/lib.rs:42 --dry-run --json   # machine-readable preview
```

### By module path or function name

```bash
cargo runner run my_module::tests::test_add
cargo runner run test_add
cargo runner run runners::unified_runner::tests
```

### By file (file-level default: bin → run, lib → often test, etc.)

```bash
cargo runner run src/main.rs
cargo runner run src/lib.rs
```

### Default entrypoint (cwd)

```bash
cargo runner run
# honors Cargo default-run, then src/main.rs / src/lib.rs, …
```

### Pass-through args (test binary flags)

```bash
cargo runner run src/lib.rs:42 -- --nocapture
cargo runner run src/lib.rs:42 -- --exact   # only if the resolved command already targets that test
```

### Cargo flags

```bash
cargo runner run src/lib.rs:42 --features foo,bar
cargo runner run src/main.rs --release
cargo runner run src/lib.rs --package my-crate
cargo runner run src/lib.rs:42 --nextest    # if cargo-nextest installed
```

### Preview first (recommended for agents)

```bash
cargo runner run src/lib.rs:42 --dry-run --json
```

Check `shell` / `args` before executing for real. On JSON mode failures, stdout may be:

```json
{ "protocol_version": 1, "error": true, "message": "..." }
```

---

## Overrides — persist “always use this command” (no re-typing)

Overrides are stored in **`.cargo-runner.json`** and apply automatically on later `run` / VS Code Cmd+R.  
**You do not need to pass the override tokens every time.**

### When to override

- Framework / tool CLI: Spin, Dioxus, custom scripts  
- Always `+nightly`, always `RUST_LOG=debug`  
- Always `--nocapture` for a specific test  
- Replace default `cargo leptos serve` with something else  

### Create (once)

```bash
# Custom tool: spin build --up
cargo runner override src/main.rs -- @spin.build --up

# Dioxus
cargo runner override src/main.rs -- @dx.serve

# Nightly for one test location
cargo runner override src/lib.rs:42 -- +nightly

# Env for a binary
cargo runner override src/main.rs -- RUST_LOG=debug RUST_BACKTRACE=1

# Test binary args (cargo test -- …)
cargo runner override src/lib.rs:42 -- /--nocapture

# Named flags
cargo runner override src/main.rs --command spin --subcommand "build --up"
```

**Token cheat-sheet**

| Token | Meaning |
|--------|---------|
| `@cmd.sub` | program `cmd`, subcommand `sub` (e.g. `@spin.serve`) |
| `@spin.build --up` | `spin` + `build` + extra arg `--up` |
| `+nightly` | toolchain channel |
| `KEY=value` | env |
| `/args` or `#args` | test-binary args after cargo’s `--` |
| `@` first alone | append/merge into existing override |
| `-` or `!!` | **remove** this override |

### Use later (no override flags)

```bash
cargo runner run src/main.rs
# or VS Code: Cmd+R on that file
# → uses saved override automatically
```

### Inspect

```bash
cargo runner override --list
cargo runner override --list --json
cargo runner override --show src/main.rs
cargo runner override --show src/lib.rs:42 --json
```

### Remove

```bash
cargo runner override src/main.rs -- -
```

### Cookbook

```bash
cargo runner override --examples
```

---

## Frameworks (auto + override)

If the project has **Leptos / Dioxus / Tauri**, cargo-runner may auto-pick their CLI for binaries.

To force a **different** program (e.g. Spin on a Leptos app):

```bash
cargo runner override src/main.rs -- @spin.serve
# or
cargo runner override src/main.rs -- @spin.build --up
```

Custom `command` (not `cargo`) wins over framework overlays.

---

## Bazel

If `MODULE.bazel` / Bazel targets exist, `run` maps to `bazel run` / `bazel test` automatically. Prefer:

```bash
cargo runner run path/to/file.rs:LINE --dry-run
cargo runner run path/to/file.rs:LINE
```

over inventing labels. Doc-tests under Bazel may run the whole crate’s `rust_doc_test` target (see limitations).

---

## Watch (re-run on save)

```bash
# Replays the same command as `run file:line` when .rs files change
cargo runner watch src/lib.rs:42

# Project-level build/test/run only
cargo runner watch
cargo runner watch --test
```

---

## What NOT to do

| Avoid | Prefer |
|--------|--------|
| `cargo test foo::bar -- --exact` (guessed) | `cargo runner run path:line` or name selector |
| Assuming RA/LSP indexes before run | Just `cargo runner run …` |
| Re-passing `@spin.serve` every run | `override` once, then plain `run` |
| Editing shell history for permanent flags | `override` → `.cargo-runner.json` |
| `run --json` without `--dry-run` | `run --dry-run --json` |

---

## Minimal agent workflow (copy this)

```text
1. Ensure init: if .cargo-runner.json missing → cargo runner init
2. Discover: cargo runner runnables <file> --json --with-commands
3. Preview:  cargo runner run <file>:<line> --dry-run --json
4. Execute:  cargo runner run <file>:<line>
5. If user wants a permanent special command (spin/dx/env/nightly):
     cargo runner override <file> -- @tool.sub … 
     then only cargo runner run <file> thereafter
6. If wrong command sticky: cargo runner override --show <file>
     remove with: cargo runner override <file> -- -
```

---

## VS Code (humans / agents driving the editor)

| Action | Shortcut / command |
|--------|---------------------|
| Run at cursor | **Cmd+R** / Ctrl+R → `Cargo Runner: Run at Cursor` |
| Override at cursor | **Cmd+Shift+R** → type tokens e.g. `@spin.build --up` |
| Extension | Marketplace: `masterustacean.cargo-runner` |

Agents editing the repo should still prefer the **CLI** in the terminal for reproducibility.

---

## Links

- Repo: https://github.com/cargo-runner/cargo-runner  
- Changelog: https://github.com/cargo-runner/cargo-runner/blob/main/CHANGELOG.md  
- IDE JSON: https://github.com/cargo-runner/cargo-runner/blob/main/docs/ide-protocol.md  
- Limitations: https://github.com/cargo-runner/cargo-runner/blob/main/docs/limitations.md  
- crates.io: `cargo install cargo-runner-cli` · binary: `cargo binstall cargo-runner-cli`
