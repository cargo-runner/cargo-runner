# Cargo Runner for VS Code

Run, test, build, and override Rust targets with the **cargo-runner** CLI — Cargo, Bazel, Dioxus, Leptos, Tauri, and standalone files.

## Features

- **`Cmd+R` / `Ctrl+R`** — run the best target at the cursor
- **`Cmd+Shift+R` / `Ctrl+Shift+R`** — set override tokens, then save & run
- **CodeLens** — ▶ Run · Debug · ⚙ Override above each detected runnable
- **Sidebar** — browse runnables and overrides; run/override/delete from context menus
- **Auto binary** — downloads a prebuilt `cargo-runner` from GitHub Releases (or uses PATH)
- **Task runner** — long-running commands (`dx serve`, `cargo leptos watch`) as VS Code tasks
- **Breakpoint awareness** — Cmd+R uses rust-analyzer Debug CodeLens when BPs are in the current function
- **Palette commands** — init, clean, watch, context dump, select runnable

## Requirements

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- Rust toolchain (`cargo` / `rustc`) for your projects

## Override tokens

```
@dx.serve --release RUST_LOG=debug /--nocapture
```

| Token | Meaning |
|-------|---------|
| `@cmd.sub` | command + subcommand |
| `@` (first) | append mode |
| `+nightly` | toolchain channel |
| `KEY=value` | environment |
| `/args` or `# args` | test binary args |
| `-` / `!!` | remove override |
| `!env` / `!#` | reset fields |

Config is stored in **`.cargo-runner.json`** (shared with the CLI).

See [IDE protocol](../../docs/ide-protocol.md) for JSON contracts.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `cargoRunner.path` | `""` | Binary path; empty = auto-download |
| `cargoRunner.useTaskRunner` | `true` | Use VS Code tasks |
| `cargoRunner.enableBreakpointDetection` | `true` | Debug when BPs present |
| `cargoRunner.releaseRepo` | `cargo-runner/cargo-runner` | GitHub release source |

## Development

```bash
cd extensions/vscode
npm install
npm run build
# Press F5 in VS Code with this folder open, or:
code --extensionDevelopmentPath=extensions/vscode
```

Use a local CLI during development:

```json
{
  "cargoRunner.path": "/path/to/target/debug/cargo-runner"
}
```

## License

MIT
