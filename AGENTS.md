# Agent notes (cargo-runner repository)

## For contributors working **in this repo**

- Prefer `cargo test` / Bazel CI targets when changing the engine itself.
- CLI package: `cargo-runner-cli`; library: `cargo-runner-core`.
- Release process: [docs/release.md](docs/release.md).
- Do not commit secrets (see `.gitignore` / `user.bazelrc`).

## For LLMs / coding agents in **any Rust project**

Use **cargo-runner** so you do not invent `cargo test` / Bazel / framework commands.

**Full copy-paste instructions:** → **[docs/AGENTS.cargo-runner.md](docs/AGENTS.cargo-runner.md)**  
**Install into a user project:** `cargo runner agent-init` or VS Code **Cargo Runner: Agent Init**

### Decision loop (always)

```text
1. INIT once:  .cargo-runner.json missing? → cargo runner init
2. SCAN:       cargo runner runnables --json --with-commands
               cargo runner run <entry> --dry-run --json
3. SUPPORTED?  → cargo runner run <entry>
4. NOT OK?     (spin / make / wrong framework default)
               → cargo runner override <entry> -- @tool.sub …
5. FOREVER:    cargo runner run <entry>   # NO override tokens again
```

`<entry>` = `src/main.rs` if present, else `src/lib.rs` (or the bin/test path you mean).

### Install (user machines)

```bash
cargo binstall cargo-runner-cli
# or
cargo install cargo-runner-cli
```

VS Code: marketplace **`masterustacean.cargo-runner`** (Cmd+R / Cmd+Shift+R / Agent Init).
