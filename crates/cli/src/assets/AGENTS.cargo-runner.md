# Agent instructions: use **cargo-runner** (do not guess Cargo commands)

Coding agents must **resolve and run** tests, binaries, benches, and doc-tests via **cargo-runner** — not invent `cargo test` / `cargo run` / Bazel labels.

**Installer (once per project):**

- **VS Code:** Command Palette → **Cargo Runner: Agent Init**
- **CLI:** `cargo runner agent-init` (or `cargo-runner agent-init`)
- **Script:** `./scripts/install-agent-instructions.sh --root /path/to/your-project`

**Requires:** `cargo-runner` / `cargo runner` on PATH (`cargo binstall cargo-runner-cli`). Version 2.x recommended.

---

## Decision loop (always follow this)

```text
1. INIT (once)
   if .cargo-runner.json missing → cargo runner init

2. SCAN — what does cargo-runner think this project can run?
   cargo runner runnables --json --with-commands
   # and/or for the file you care about:
   cargo runner runnables src/main.rs --json --with-commands
   cargo runner runnables src/lib.rs --json --with-commands
   cargo runner run <entry> --dry-run --json

3. SUPPORTED?
   Dry-run / runnables show a correct shell
   (cargo run|test|bench|doc, bazel run|test, dx, cargo leptos, cargo tauri, rustc, …)
   → EXECUTE with plain run (no override tokens):
        cargo runner run src/main.rs
        cargo runner run src/lib.rs
        cargo runner run path/to/file.rs:LINE

4. NOT SUPPORTED / WRONG TOOL?
   Project really needs make, spin, a custom script, or a different CLI
   than the default (e.g. Leptos overlay but you need Spin)
   → OVERRIDE ONCE on the right entry file, then stop passing custom tokens:
        # App binary:
        cargo runner override src/main.rs -- @spin.build --up
        cargo runner override src/main.rs -- @make.run
        # Library-only crate (no main.rs) — bind to lib or the bin you run:
        cargo runner override src/lib.rs -- @make.test
        cargo runner override src/bin/app.rs -- @spin.serve

5. FOREVER AFTER (same entry)
   cargo runner run src/main.rs
   # NO @spin, NO make args, NO --command …
   # Override is loaded automatically from .cargo-runner.json
```

**Critical:** After step 4, every later run is **plain** `cargo runner run <same path>`.  
Do **not** re-type `@spin…` / `make …` on every agent turn. That wastes tokens and fights the config.

---

## Golden rules

**Do**

1. **Scan first** (`runnables` / `run --dry-run --json`) before inventing a shell command  
2. **Run** with cargo-runner when the preview is correct  
3. **Override once** when the real tool is custom (make, spin, …) or the default is wrong  
4. **Bind the override to the executable entry** you will keep running:
   - Prefer `src/main.rs` when it exists  
   - Else `src/lib.rs`, or `src/bin/*.rs` / the path the user runs  
   - For one test/doc scope: `src/lib.rs:LINE` (1-based)  
5. Later: **only** `cargo runner run <that entry>` (override applies automatically)

**Do not**

- Guess `cargo test some::path -- --exact`  
- Guess Bazel `//package:target` labels  
- Wait for rust-analyzer / LSP before running  
- Hand-write `spin` / `make` / `dx` / `cargo leptos` **every** run — set `override` once instead  
- Re-pass override tokens after they are saved  

---

## First-time setup

```bash
cargo runner init          # creates .cargo-runner.json (required before override)
cargo runner doctor --json # optional health check
```

---

## Scan (discover what can run)

```bash
# Whole workspace
cargo runner runnables --json --with-commands

# One file
cargo runner runnables src/lib.rs --json --with-commands
cargo runner runnables src/main.rs --json --with-commands

# Filters
cargo runner runnables src/lib.rs --test --json
cargo runner runnables --doc --json
cargo runner runnables --name my_test --json
cargo runner runnables --symbol MyStruct --json
```

Parse `label`, `kind`, `file_path`, `module_path`, and especially `command.shell`.

Preview before executing:

```bash
cargo runner run src/main.rs --dry-run --json
cargo runner run src/lib.rs:42 --dry-run --json
```

JSON errors look like:

```json
{ "protocol_version": 1, "error": true, "message": "..." }
```

---

## Execute (supported path)

Line numbers are **1-based**.

```bash
# File-level (binary / lib defaults)
cargo runner run src/main.rs
cargo runner run src/lib.rs

# Test / doc / fn under a line
cargo runner run src/lib.rs:42

# By module path or name
cargo runner run my_module::tests::test_add
cargo runner run test_add

# Project default (cwd)
cargo runner run

# Optional Cargo flags (only when needed)
cargo runner run src/lib.rs:42 --features foo,bar
cargo runner run src/main.rs --release
cargo runner run src/lib.rs --package my-crate
cargo runner run src/lib.rs:42 --nextest

# Pass-through to test binary
cargo runner run src/lib.rs:42 -- --nocapture
```

---

## Override (unsupported / custom tools) — once, then plain run

Stored in **`.cargo-runner.json`**. Applied automatically on later `run` and VS Code **Cmd+R** for that identity.

### When to override

| Situation | Action |
|-----------|--------|
| Need **spin**, **make**, custom script | `override` with `@spin…` / `@make…` / `--command` |
| Default is Leptos/Dioxus/Tauri but real tool differs | `override` with the real program |
| Always nightly / env / `--nocapture` for a scope | `override` with `+nightly`, `KEY=val`, `/--nocapture` |
| Preview shell is wrong for “run the app” | Fix with `override` on the **app entry** |

### Choose the path you override (important)

| Project shape | Override target | Later run (no extra params) |
|---------------|-----------------|-------------------------------|
| Has `src/main.rs` | `src/main.rs` | `cargo runner run src/main.rs` |
| Lib-only (no main) | `src/lib.rs` | `cargo runner run src/lib.rs` |
| Extra binary | `src/bin/foo.rs` | `cargo runner run src/bin/foo.rs` |
| One test forever | `src/lib.rs:42` | `cargo runner run src/lib.rs:42` |

An override on `src/main.rs` does **not** apply to a different test on `src/lib.rs:10`. Match the entry you will keep running.

### Create (once)

```bash
# Spin instead of default cargo / leptos
cargo runner override src/main.rs -- @spin.build --up

# Make
cargo runner override src/main.rs -- @make.run
cargo runner override src/lib.rs -- @make.test

# Dioxus explicit
cargo runner override src/main.rs -- @dx.serve

# Named flags form
cargo runner override src/main.rs --command spin --subcommand "build --up"

# Env / channel / test args
cargo runner override src/main.rs -- RUST_LOG=debug
cargo runner override src/lib.rs:42 -- +nightly
cargo runner override src/lib.rs:42 -- /--nocapture
```

### Forever after (no tokens)

```bash
cargo runner run src/main.rs
# → uses .cargo-runner.json override automatically
```

### Inspect / remove

```bash
cargo runner override --list
cargo runner override --list --json
cargo runner override --show src/main.rs
cargo runner override src/main.rs -- -          # remove
cargo runner override --examples
```

**Token cheat-sheet**

| Token | Meaning |
|--------|---------|
| `@cmd.sub` | program + subcommand (e.g. `@spin.serve`, `@make.test`) |
| `@spin.build --up` | `spin` + `build` + extra `--up` |
| `+nightly` | toolchain channel |
| `KEY=value` | environment |
| `/args` or `#args` | test-binary args after cargo’s `--` |
| `@` alone first | merge into existing override |
| `-` or `!!` | remove this override |

Custom `command` (not `cargo`) **wins over** Leptos/Dioxus/Tauri overlays.

---

## Frameworks (auto)

Leptos / Dioxus / Tauri projects may auto-select their CLI for binaries.  
If that is wrong for this repo → **override once** on the entry file (see above), then plain `run`.

---

## Bazel

With Bazel targets, `run` maps to `bazel run` / `bazel test` from the file location. Prefer:

```bash
cargo runner run path/to/file.rs:LINE --dry-run
cargo runner run path/to/file.rs:LINE
```

Do not invent labels. Doc-tests under Bazel may be coarser (whole crate) — see limitations.

---

## Watch

```bash
cargo runner watch src/lib.rs:42   # re-runs same command as `run` on that scope
cargo runner watch
cargo runner watch --test
```

---

## What NOT to do

| Avoid | Prefer |
|--------|--------|
| Guessed `cargo test foo::bar -- --exact` | Scan → `cargo runner run path:line` |
| Wait for RA/LSP | `cargo runner run …` immediately |
| `spin build --up` every chat turn | `override` once → plain `run src/main.rs` |
| Override on wrong file | Bind to `main.rs` / `lib.rs` / bin you actually run |
| `run --json` without `--dry-run` | `run --dry-run --json` |

---

## Minimal copy-paste checklist

```text
1. .cargo-runner.json missing? → cargo runner init
2. SCAN:  cargo runner runnables --json --with-commands
          cargo runner run <entry> --dry-run --json
3. OK shell? → cargo runner run <entry>
4. Need make/spin/custom or wrong default?
     → cargo runner override <entry> -- @tool.sub …
     → thereafter ONLY: cargo runner run <entry>
5. Sticky wrong? → cargo runner override --show <entry>
                 → cargo runner override <entry> -- -
```

`<entry>` = `src/main.rs` if present, else `src/lib.rs` (or the bin/test path you mean).

---

## VS Code

| Action | How |
|--------|-----|
| Run at cursor | **Cmd+R** / Ctrl+R |
| Override at cursor | **Cmd+Shift+R** (e.g. `@spin.build --up`) |
| Install these instructions | **Cargo Runner: Agent Init** |
| Extension | `masterustacean.cargo-runner` |

Prefer the **CLI** in the terminal for reproducible agent runs.

---

## Links

- Repo: https://github.com/cargo-runner/cargo-runner  
- Changelog: https://github.com/cargo-runner/cargo-runner/blob/main/CHANGELOG.md  
- IDE JSON: https://github.com/cargo-runner/cargo-runner/blob/main/docs/ide-protocol.md  
- Limitations: https://github.com/cargo-runner/cargo-runner/blob/main/docs/limitations.md  
- Install: `cargo binstall cargo-runner-cli` / `cargo install cargo-runner-cli`
