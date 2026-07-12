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
- **Palette commands** — init, **Agent Init**, clean, watch, context dump, select runnable
- **Agent Init** — install cargo-runner instructions into `AGENTS.md` / `CLAUDE.md` / Cursor / Copilot files so coding agents use `cargo runner` (no shell script required)

## Requirements

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- Rust toolchain (`cargo` / `rustc`) for your projects
- **cargo-runner CLI** — same version as this extension (e.g. both `1.6.2`)

### CLI install prompt

On first activate, if the CLI is missing you get **Download CLI** / **Later**.

If you press **Cmd+R** or **Cmd+Shift+R** without a CLI:

1. A status-bar toast appears for **5 seconds**
2. An error notification offers **Download CLI**
3. Clicking **Download CLI** fetches `cargo-runner-cli-v{extensionVersion}` for your platform, extracts it, runs `chmod +x` (and clears macOS quarantine when possible), then verifies with `--version`

You can also run **Cargo Runner: Download CLI** from the command palette.

### CLI update prompts (extension stays put)

If a **newer CLI** is published on GitHub while the VS Code extension is still an older version, on activate you get:

- Status-bar toast (~5s)
- Notification: *CLI vX is available (you have vY)* → **Download Update** / **Later** / **Skip this version**

Settings: `cargoRunner.checkCliUpdates` (default on), `cargoRunner.cliUpdateCheckIntervalHours` (default 24).  
Manual check: **Cargo Runner: Check for CLI Updates**.

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
