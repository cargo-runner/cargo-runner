# IDE Protocol

Machine-readable CLI contracts for the VS Code (and future) adapters.

**Protocol version:** 1  
**Version parity:** CLI and VS Code extension share the same semver (e.g. both `1.6.2`).  
Release tags: `cargo-runner-cli-v{version}`. The extension prefers downloading that exact tag.

All JSON modes print **only** JSON on stdout (no emoji banners). Errors still go to stderr and non-zero exit codes.

---

## `runnables --json`

```bash
cargo runner runnables [path[:line]] --json
cargo runner runnables [path[:line]] --json --with-commands
cargo runner runnables --json                 # whole workspace
```

### Output

Array of entries. With `--json`, each item is a runnable plus optional command:

```json
[
  {
    "label": "test tests::it_works",
    "scope": { "start": { "line": 10, "character": 0 }, "end": { "line": 14, "character": 1 } },
    "kind": { "Test": { "test_name": "it_works", "is_async": false } },
    "module_path": "tests",
    "file_path": "/abs/path/src/lib.rs",
    "command": {
      "program": "cargo",
      "args": ["test", "tests::it_works", "--", "--exact"],
      "cwd": "/abs/path",
      "shell": "cargo test tests::it_works -- --exact"
    }
  }
]
```

`command` is present only when `--with-commands` is set.

Legacy: `--verbose` still dumps a bare `Runnable[]` JSON array (no command previews). Prefer `--json` for new IDE code.

---

## `run --dry-run --json`

```bash
cargo runner run path/to/file.rs:12 --dry-run --json
```

**`--json` requires `--dry-run`.** Using `--json` alone is an error (avoids a silent no-op).

Extra args after a trailing `--` on the CLI are forwarded into the generated command
(for cargo test/doc: after cargo’s `--` as test-binary args).

### Output (`DryRunOutput`)

```json
{
  "protocol_version": 1,
  "program": "cargo",
  "args": ["test", "tests::it_works", "--", "--exact"],
  "cwd": "/abs/path",
  "env": {},
  "shell": "cargo test tests::it_works -- --exact",
  "strategy": "cargo",
  "runnable": { "...": "optional matched Runnable" },
  "warnings": []
}
```

`strategy` is one of: `cargo`, `cargo_script`, `rustc`, `shell`, `bazel`.

Internal env keys starting with `_` are stripped from `env`. Known markers such as
`_BAZEL_DOC_TEST_LIMITATION` are promoted into `warnings` (string array) for IDE UI.

---

## `context --json`

```bash
cargo runner context [path[:line]] --json
```

See `RunnerContext` in `crates/cli/src/commands/context.rs` (`context_version: 1`).

---

## `override --list|--show --json`

```bash
cargo runner override --list --json
cargo runner override --list --file src/lib.rs --json
cargo runner override --show src/lib.rs:12 --json
```

### List entry

```json
[
  {
    "config_path": "/abs/path/.cargo-runner.json",
    "override": {
      "match": {
        "file_path": "/abs/path/src/lib.rs",
        "function_name": "it_works",
        "module_path": "tests"
      },
      "cargo": {
        "extra_args": ["--release"],
        "extra_env": { "RUST_LOG": "debug" }
      }
    }
  }
]
```

Create / remove remains:

```bash
cargo runner override src/lib.rs:12 -- @dx.serve --release RUST_LOG=debug
cargo runner override src/lib.rs:12 -- -
cargo runner override src/lib.rs:12 -- !!
```

---

## Override tokens (unified)

| Token | Effect |
|-------|--------|
| `@cmd.sub` | Set command + subcommand (`@dx.serve`, `@cargo.watch`) |
| `@` (first, alone) | Append/merge mode (legacy UX flag) |
| `+nightly` | Toolchain channel |
| `KEY=value` | Environment variable |
| `/args…` or `# args…` | Test binary args |
| bare flags | `extra_args` |
| `-command`, `-env`, `-arg`, `-test`, `-/` | Remove fields |
| `!env`, `!#`, `!/`, `!args`, `!features` | Legacy field resets |
| `-` or `!!` | Remove entire matching override |

Parser: `OverrideManager::parse_override_args` in `crates/core/src/config/override_manager.rs`.

---

## Binary releases (extension download)

Tags: `cargo-runner-cli-v*` — **must match** the VS Code extension `package.json` version.

Example: extension `1.6.2` downloads tag `cargo-runner-cli-v1.6.2`.

Asset pattern:

```text
cargo-runner-cli-{rustc-target}-v{version}.tar.gz
```

Contains `cargo-runner` (or `cargo-runner.exe`). After extract the extension:

1. `chmod 0o755` (Unix)
2. Clears macOS `com.apple.quarantine` when present
3. Runs `cargo-runner --version` to verify executability

| Host | Rust target |
|------|-------------|
| darwin arm64 | `aarch64-apple-darwin` |
| darwin x64 | `x86_64-apple-darwin` |
| linux x64 | `x86_64-unknown-linux-gnu` |
| linux arm64 | `aarch64-unknown-linux-gnu` |
| win32 x64 | `x86_64-pc-windows-msvc` |

Multi-platform builds: `.github/workflows/release.yml` (triggered by tag push).

---

## Extension settings (planned)

```jsonc
{
  "cargoRunner.path": "",
  "cargoRunner.useTaskRunner": true,
  "cargoRunner.enableCodeLens": true,
  "cargoRunner.enableBreakpointDetection": true,
  "cargoRunner.cliMinVersion": "1.0.0",
  "cargoRunner.releaseRepo": "cargo-runner/cargo-runner"
}
```

Project config remains **`.cargo-runner.json`** only.
