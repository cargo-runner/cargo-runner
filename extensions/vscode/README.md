# Cargo Runner for VS Code

Run, test, build, and override Rust targets with the **cargo-runner** CLI — **without waiting for rust-analyzer** to index.

Supports **Cargo**, **Bazel**, **Dioxus / Leptos / Tauri**, **rustc** / single-file scripts, and **custom tools** (Spin, make, …) via sticky overrides.

CLI and extension share the same version (e.g. both **2.1.x**).

## Features

- **`Cmd+R` / `Ctrl+R`** — run the best target at the cursor
- **`Cmd+Shift+R` / `Ctrl+Shift+R`** — set override tokens, then save & run
- **CodeLens** — ▶ Run · Debug · ⚙ Override above each detected runnable
- **Sidebar** — browse runnables and overrides
- **Auto binary** — downloads `cargo-runner` from GitHub Releases (or PATH)
- **Task runner** — long-running commands (`dx serve`, `cargo leptos watch`) as tasks
- **Breakpoint awareness** — when BPs are present, can use rust-analyzer Debug CodeLens
- **Agent Init** — palette command installs cargo-runner instructions into `AGENTS.md` / `CLAUDE.md` / Cursor / Copilot (no shell script)
- **Other palette commands** — init, clean, watch, context dump, select runnable, Download CLI

## Requirements

- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) (debug CodeLens / optional BP path)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
- Rust toolchain (`cargo` / `rustc`) for your projects
- **cargo-runner CLI** — same version as this extension

> **Run does not require rust-analyzer.** The extension shells out to the CLI’s own scope engine. RA is only needed for the optional debug/breakpoint path.

### CLI install / update

On first activate, if the CLI is missing: **Download CLI** / **Later**.

**Download CLI** fetches `cargo-runner-cli-v{extensionVersion}`, extracts it, `chmod +x`, clears macOS quarantine when possible, verifies with `--version`.

If a **newer CLI** exists on GitHub while this extension is older: toast + **Download Update** / **Later** / **Skip**.

| Setting | Default |
|---------|---------|
| `cargoRunner.checkCliUpdates` | on |
| `cargoRunner.cliUpdateCheckIntervalHours` | 24 |

Manual: **Cargo Runner: Check for CLI Updates** · **Cargo Runner: Download CLI**.

## Agent Init

Command Palette → **Cargo Runner: Agent Init**

1. Scans / installs agent instructions (managed HTML-comment block)
2. Agents then **scan** with `runnables`, **run** if supported, **override once** for Spin/make/custom, then plain `cargo runner run <entry>` forever

See [docs/AGENTS.cargo-runner.md](../../docs/AGENTS.cargo-runner.md).

## Override tokens

```
@dx.serve --release RUST_LOG=debug /--nocapture
@spin.build --up
```

| Token | Meaning |
|-------|---------|
| `@cmd.sub` | command + subcommand |
| `@` (first) | append mode |
| `+nightly` | toolchain channel |
| `KEY=value` | environment |
| `/args` or `# args` | test binary args |
| `-` / `!!` | remove override |

Stored in **`.cargo-runner.json`** (shared with CLI). After override, **Cmd+R** uses it with no extra tokens.

IDE JSON contracts: [docs/ide-protocol.md](../../docs/ide-protocol.md).

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
# F5, or:
code --extensionDevelopmentPath=extensions/vscode
```

```json
{
  "cargoRunner.path": "/path/to/target/debug/cargo-runner"
}
```

## License

MIT
