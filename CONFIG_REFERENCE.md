# Cargo Runner Configuration Reference

This document describes all available configuration fields for `.cargo-runner.json` files.

## Configuration Hierarchy

Cargo Runner supports a hierarchical configuration system:

1. **Root Config** - At `PROJECT_ROOT/.cargo-runner.json` (set via `PROJECT_ROOT` env var)
2. **Workspace Config** - In workspace root containing `Cargo.toml`
3. **Package Config** - In individual package directories

Configurations are merged with package-specific settings overriding workspace settings, which override root settings.

## Global Configuration Fields

These fields apply to all commands unless overridden:

### Basic Fields

```json
{
  "package": "my-package",
  "version": "1.0",
  "cache_enabled": true,
  "cache_ttl": 3600,
  "command": "cargo",
  "subcommand": "run",
  "channel": "nightly",
  "extra_args": ["--release"],
  "extra_env": {
    "RUST_BACKTRACE": "1",
    "CARGO_TARGET_DIR": "target/custom"
  },
  "extra_test_binary_args": ["--nocapture", "--test-threads=1"]
}
```

- **package** (string): Package name for identification
- **version** (string): Config version (currently "1.0")
- **cache_enabled** (bool): Enable caching of detected runnables
- **cache_ttl** (number): Cache time-to-live in seconds
- **command** (string): Base command (default: "cargo")
- **subcommand** (string): Subcommand to run (e.g., "run", "test")
- **channel** (string): Rust toolchain channel (e.g., "nightly", "stable")
- **extra_args** (array): Additional arguments for cargo commands
- **extra_env** (object): Environment variables to set
- **extra_test_binary_args** (array): Arguments passed after `--` to test binaries

### Linked Projects

For monorepo/workspace setups:

```json
{
  "linked_projects": [
    "/path/to/project1/Cargo.toml",
    "/path/to/project2/Cargo.toml"
  ]
}
```

## Framework Configuration

### Test Framework

Configure custom test runners:

```json
{
  "test_framework": {
    "command": "cargo",
    "subcommand": "nextest run",
    "channel": "nightly",
    "extra_args": ["-j10"],
    "extra_env": {
      "RUST_BACKTRACE": "full",
      "NEXTEST_RETRIES": "2"
    }
  }
}
```

### Binary Framework

Configure custom binary runners:

```json
{
  "binary_framework": {
    "command": "dx",
    "subcommand": "serve",
    "extra_args": ["--hot-reload"],
    "extra_env": {
      "DIOXUS_LOG": "debug"
    }
  }
}
```

Common binary frameworks:
- Dioxus: `{"command": "dx", "subcommand": "serve"}`
- Leptos: `{"command": "cargo", "subcommand": "leptos watch"}`
- Trunk: `{"command": "trunk", "subcommand": "serve"}`

## Override Configuration

Override settings for specific functions/modules:

```json
{
  "overrides": [
    {
      "match": {
        "package": "my-package",
        "module_path": "my_crate::tests",
        "file_path": "src/tests/unit.rs",
        "function_name": "test_specific"
      },
      "command": "cargo",
      "subcommand": "test",
      "channel": "nightly",
      "extra_args": ["--release"],
      "extra_test_binary_args": ["--nocapture"],
      "extra_env": {
        "TEST_LOG": "debug"
      },
      "test_framework": {
        "command": "cargo",
        "subcommand": "nextest run"
      },
      "force_replace_args": false,
      "force_replace_env": false
    }
  ]
}
```

### Match Fields

The `match` object identifies which runnables to override:

- **package** (string): Package name
- **module_path** (string): Module path (e.g., "my_crate::utils")
- **file_path** (string): Relative file path
- **function_name** (string): Function/test name

All specified fields must match for the override to apply. Omitted fields match any value.

### Override Fields

- All global configuration fields can be overridden
- **test_framework**: Override test framework for matched tests
- **force_replace_args** (bool): Replace args instead of appending
- **force_replace_env** (bool): Replace env vars instead of merging

## Complete Example

```json
{
  "package": "my-app",
  "version": "1.0",
  "cache_enabled": true,
  "cache_ttl": 3600,
  "extra_args": [],
  "extra_env": {
    "CARGO_TARGET_DIR": "target/rust-analyzer",
    "RUST_LOG": "debug"
  },
  "extra_test_binary_args": ["--nocapture"],
  
  "test_framework": {
    "command": "cargo",
    "subcommand": "nextest run",
    "channel": "nightly",
    "extra_args": ["-j10"],
    "extra_env": {
      "RUST_BACKTRACE": "full"
    }
  },
  
  "binary_framework": {
    "command": "dx",
    "subcommand": "serve",
    "extra_args": ["--hot-reload"],
    "extra_env": {
      "DIOXUS_LOG": "debug"
    }
  },
  
  "overrides": [
    {
      "match": {
        "package": "my-app",
        "module_path": "my_app"
      },
      "extra_args": ["--platform", "web"]
    },
    {
      "match": {
        "function_name": "test_integration"
      },
      "extra_env": {
        "TEST_DATABASE_URL": "postgres://test"
      }
    },
    {
      "match": {
        "file_path": "benches/performance.rs"
      },
      "channel": "nightly",
      "extra_args": ["--features", "bench"]
    }
  ]
}
```

## Usage Tips

1. **Environment Variables**: Use `extra_env` at any level to set environment variables
2. **Test Configuration**: Use `test_framework` for custom test runners like nextest
3. **Binary Runners**: Use `binary_framework` for web frameworks like Dioxus, Leptos
4. **Overrides**: Target specific functions with precise `match` patterns
5. **Merging**: Configs merge by default; use `force_replace_*` to override completely

## Command Examples

```bash
# Initialize configuration in workspace
cargo runner init

# Analyze runnables with config details
cargo runner analyze --config src/main.rs

# Run with configuration applied
cargo runner run src/main.rs:42

# Run without line number (file-level command)
cargo runner run src/main.rs
```