# Cargo Runner Configuration Reference

This document describes all available configuration fields for `.cargo-runner.json` files.

## Configuration Hierarchy

Cargo Runner supports a hierarchical configuration system:

1. **Root Config** - At `PROJECT_ROOT/.cargo-runner.json` (set via `PROJECT_ROOT` env var)
2. **Workspace Config** - In workspace root containing `Cargo.toml`
3. **Package Config** - In individual package directories

Configurations are merged with package-specific settings overriding workspace settings, which override root settings.

## Configuration Structure

Starting from version 0.2.0, Cargo Runner uses a nested configuration structure to isolate settings for different command types:

```json
{
  "cargo": {
    // Cargo-specific settings
  },
  "rustc": {
    // Rustc-specific settings for standalone files
  },
  "single_file_script": {
    // Settings for cargo script files (RFC 3424)
  },
  "overrides": [
    // Function-specific overrides
  ]
}
```

## Cargo Configuration

Settings for Cargo projects (files within a project with Cargo.toml):

```json
{
  "cargo": {
    "package": "my-package",
    "command": "cargo",
    "subcommand": "run",
    "channel": "nightly",
    "features": ["core", "logging"],
    "extra_args": ["--release"],
    "extra_env": {
      "RUST_BACKTRACE": "1",
      "CARGO_TARGET_DIR": "target/custom"
    },
    "extra_test_binary_args": ["--nocapture", "--test-threads=1"],
    "linked_projects": [
      "/path/to/project1/Cargo.toml",
      "/path/to/project2/Cargo.toml"
    ],
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest run",
      "channel": "nightly",
      "features": ["test-utils"],
      "extra_args": ["-j10"],
      "extra_env": {
        "RUST_BACKTRACE": "full",
        "NEXTEST_RETRIES": "2"
      }
    },
    "binary_framework": {
      "command": "dx",
      "subcommand": "serve",
      "features": ["web"],
      "extra_args": ["--hot-reload"],
      "extra_env": {
        "DIOXUS_LOG": "debug"
      }
    }
  }
}
```

### Cargo Fields

- **package** (string): Package name for identification
- **command** (string): Base command (default: "cargo")
- **subcommand** (string): Subcommand to run (e.g., "run", "test")
- **channel** (string): Rust toolchain channel (e.g., "nightly", "stable")
- **features** (string | array): Feature flags to enable
  - `"all"`: Enables all features with `--all-features`
  - `["web", "desktop"]`: Enables specific features with `--features=web,desktop`
- **extra_args** (array): Additional arguments for cargo commands
- **extra_env** (object): Environment variables to set
- **extra_test_binary_args** (array): Arguments passed after `--` to test binaries
- **linked_projects** (array): Paths to linked Cargo.toml files for monorepo setups
- **test_framework** (object): Custom test runner configuration
- **binary_framework** (object): Custom binary runner configuration

## Rustc Configuration

Settings for standalone Rust files (not part of a Cargo project):

```json
{
  "rustc": {
    "extra_args": ["--edition=2021", "-O", "--target=wasm32-unknown-unknown"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    }
  }
}
```

### Rustc Fields

- **extra_args** (array): Additional arguments for rustc commands
- **extra_env** (object): Environment variables to set

## Single File Script Configuration

Settings for Cargo script files (RFC 3424 - files with `#!/usr/bin/env -S cargo +nightly -Zscript`):

```json
{
  "single_file_script": {
    "extra_args": ["-Zscript"],
    "extra_env": {
      "CARGO_SCRIPT_DEBUG": "1"
    }
  }
}
```

### Single File Script Fields

- **extra_args** (array): Additional arguments for cargo script commands
- **extra_env** (object): Environment variables to set

## Test Framework Configuration

Configure custom test runners:

```json
{
  "cargo": {
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest run",
      "channel": "nightly",
      "features": ["test-utils"],
      "extra_args": ["-j10"],
      "extra_env": {
        "RUST_BACKTRACE": "full",
        "NEXTEST_RETRIES": "2"
      }
    }
  }
}
```

## Binary Framework Configuration

Configure custom binary runners:

```json
{
  "cargo": {
    "binary_framework": {
      "command": "dx",
      "subcommand": "serve",
      "features": ["web"],
      "extra_args": ["--hot-reload"],
      "extra_env": {
        "DIOXUS_LOG": "debug"
      }
    }
  }
}
```

Common binary frameworks:
- Dioxus: `{"command": "dx", "subcommand": "serve"}`
- Leptos: `{"command": "cargo", "subcommand": "leptos watch"}`
- Trunk: `{"command": "trunk", "subcommand": "serve"}`

## Features Configuration

The `features` field can be used at any configuration level to control Cargo feature flags:

### String Format
```json
{
  "cargo": {
    "features": "all"
  }
}
```
Results in: `cargo test --all-features`

### Array Format
```json
{
  "cargo": {
    "features": ["web", "desktop", "logging"]
  }
}
```
Results in: `cargo test --features=web,desktop,logging`

### Feature Merging

Features are merged across configuration levels:
- Root: `["core", "logging"]`
- Package: `["web"]`
- Result: `--features=core,logging,web`

Use `force_replace_features` in overrides to replace instead of merge.

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
        "function_name": "test_specific",
        "file_type": "cargo_project"
      },
      "cargo": {
        "command": "cargo",
        "subcommand": "test",
        "channel": "nightly",
        "features": ["integration-tests"],
        "extra_args": ["--release"],
        "extra_test_binary_args": ["--nocapture"],
        "extra_env": {
          "TEST_LOG": "debug"
        },
        "test_framework": {
          "command": "cargo",
          "subcommand": "nextest run"
        }
      },
      "force_replace_args": false,
      "force_replace_env": false,
      "force_replace_features": false
    },
    {
      "match": {
        "file_type": "standalone"
      },
      "rustc": {
        "extra_args": ["--edition=2024", "--crate-type=bin"]
      }
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
- **file_type** (string): One of "cargo_project", "standalone", or "single_file_script"

All specified fields must match for the override to apply. Omitted fields match any value.

### Override Fields

- Configuration can be specified in `cargo`, `rustc`, or `single_file_script` sections
- **force_replace_args** (bool): Replace args instead of appending
- **force_replace_features** (bool): Replace features instead of merging
- **force_replace_env** (bool): Replace env vars instead of merging

## Complete Example

```json
{
  "cargo": {
    "package": "my-app",
    "features": ["default", "logging"],
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
    }
  },
  "rustc": {
    "extra_args": ["--edition=2021", "-O"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    }
  },
  "single_file_script": {
    "extra_args": ["-Zscript"],
    "extra_env": {
      "CARGO_SCRIPT_DEBUG": "1"
    }
  },
  "overrides": [
    {
      "match": {
        "package": "my-app",
        "module_path": "my_app"
      },
      "cargo": {
        "extra_args": ["--platform", "web"]
      }
    },
    {
      "match": {
        "function_name": "test_integration"
      },
      "cargo": {
        "extra_env": {
          "TEST_DATABASE_URL": "postgres://test"
        }
      }
    },
    {
      "match": {
        "file_path": "benches/performance.rs"
      },
      "cargo": {
        "channel": "nightly",
        "extra_args": ["--features", "bench"]
      }
    },
    {
      "match": {
        "file_type": "standalone"
      },
      "rustc": {
        "extra_args": ["--edition=2024"]
      }
    }
  ]
}
```

## Migration from Old Format

If you have configurations in the old flat format, they need to be updated to the new nested structure:

### Old Format:
```json
{
  "package": "my-package",
  "features": "all",
  "extra_args": ["--release"]
}
```

### New Format:
```json
{
  "cargo": {
    "package": "my-package",
    "features": "all",
    "extra_args": ["--release"]
  }
}
```

## Usage Tips

1. **Command Type Isolation**: Cargo-specific settings (like `--all-features`) won't affect rustc commands
2. **Environment Variables**: Use `extra_env` at any level to set environment variables
3. **Test Configuration**: Use `test_framework` for custom test runners like nextest
4. **Binary Runners**: Use `binary_framework` for web frameworks like Dioxus, Leptos
5. **Overrides**: Target specific functions with precise `match` patterns
6. **Merging**: Configs merge by default; use `force_replace_*` to override completely

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

# Run a standalone file (uses rustc config)
cargo runner run test.rs

# Run a cargo script file (uses single_file_script config)
cargo runner run script.rs
```