# Cargo Runner Override Configuration Guide

This guide explains how to configure overrides in cargo-runner for different build systems and scenarios.

## Table of Contents
- [Overview](#overview)
- [Configuration File Location](#configuration-file-location)
- [Basic Structure](#basic-structure)
- [Build System Configuration](#build-system-configuration)
- [Override System](#override-system)
- [Examples](#examples)

## Overview

Cargo-runner supports flexible configuration through JSON files that allow you to:
- Switch between different build systems (Cargo, Bazel)
- Override command arguments for specific tests or binaries
- Set environment variables
- Configure test filtering and execution

## Configuration File Location

Configuration files can be placed at:
- `.cargo-runner.json` or `cargo-runner.json` in your project root
- `.cargo-runner.json` in any subdirectory (for package-specific config)
- `~/.config/cargo-runner/config.json` for global configuration

## Basic Structure

```json
{
  "cargo": {
    "command": "cargo",  // or "bazel" for Bazel projects
    "package": "my_package",
    "extra_args": [],
    "extra_env": {},
    "extra_test_binary_args": []
  },
  "bazel": {
    // Bazel-specific configuration
  },
  "overrides": [
    // Override rules
  ]
}
```

## Build System Configuration

### Cargo Projects (Default)

```json
{
  "cargo": {
    "command": "cargo",
    "channel": "stable",  // or "nightly", "beta"
    "package": "my_crate",
    "extra_args": ["--features", "test-feature"],
    "extra_env": {
      "RUST_LOG": "debug"
    },
    "extra_test_binary_args": ["--nocapture", "--test-threads=1"]
  }
}
```

### Bazel Projects

```json
{
  "cargo": {
    "command": "bazel",  // This switches to Bazel mode
    "package": "my_package"
  },
  "bazel": {
    "workspace": "my_workspace",
    "default_test_target": "//src:all_tests",
    "default_binary_target": "//src:main"
  }
}
```

## Override System

Overrides allow you to customize behavior for specific functions, files, or modules.

### Override Structure

```json
{
  "overrides": [
    {
      "match": {
        // Match criteria (all optional)
        "package": "my_crate",
        "module_path": "my_crate::tests",
        "file_path": "/absolute/path/to/file.rs",
        "function_name": "test_specific_function",
        "file_type": "CargoProject"  // or "SingleFile"
      },
      // Build system specific overrides
      "cargo": { /* cargo-specific */ },
      "bazel": { /* bazel-specific */ },
      "rustc": { /* rustc-specific */ },
      "single_file_script": { /* single-file-specific */ }
    }
  ]
}
```

### Match Criteria

All match fields are optional. The more specific your match, the higher priority it has:

- `package`: The package/crate name
- `module_path`: Full module path (e.g., "my_crate::tests::unit")
- `file_path`: Absolute path to the file
- `function_name`: Specific function/test name
- `file_type`: "CargoProject" or "SingleFile"

### Cargo Overrides

```json
{
  "cargo": {
    "channel": "nightly",
    "subcommand": "nextest",  // Use cargo-nextest
    "extra_args": ["--features", "experimental"],
    "extra_test_binary_args": ["--nocapture"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    }
  }
}
```

### Bazel Overrides

```json
{
  "bazel": {
    "extra_test_args": ["--nocapture", "--test-threads=1"],
    "extra_env": {
      "RUST_LOG": "debug"
    },
    "test_target": "//src:special_test",
    "binary_target": "//src:special_binary"
  }
}
```

## Examples

### Example 1: Cargo Project with Test-Specific Overrides

```json
{
  "cargo": {
    "package": "my_app",
    "extra_test_binary_args": ["--nocapture"]
  },
  "overrides": [
    {
      "match": {
        "module_path": "my_app::integration_tests"
      },
      "cargo": {
        "extra_test_binary_args": ["--test-threads=1"],
        "extra_env": {
          "TEST_DATABASE_URL": "postgresql://localhost/test"
        }
      }
    },
    {
      "match": {
        "function_name": "flaky_test"
      },
      "cargo": {
        "extra_args": ["--", "--test-threads=1", "--nocapture"]
      }
    }
  ]
}
```

### Example 2: Bazel Project with Multiple Targets

```json
{
  "cargo": {
    "command": "bazel",
    "package": "my_workspace"
  },
  "overrides": [
    {
      "match": {
        "file_path": "/Users/me/project/frontend/src/lib.rs"
      },
      "bazel": {
        "test_target": "//frontend:unit_tests",
        "extra_test_args": ["--nocapture"]
      }
    },
    {
      "match": {
        "module_path": "backend::integration"
      },
      "bazel": {
        "test_target": "//backend:integration_tests",
        "extra_test_args": ["--test-threads=1"],
        "extra_env": {
          "TEST_ENV": "integration"
        }
      }
    }
  ]
}
```

### Example 3: Mixed Cargo/Bazel Monorepo

Root `.cargo-runner.json`:
```json
{
  "cargo": {
    "linked_projects": [
      "/Users/me/monorepo/rust-service/Cargo.toml",
      "/Users/me/monorepo/wasm-app/Cargo.toml"
    ]
  }
}
```

Bazel subproject (`/Users/me/monorepo/cpp-service/.cargo-runner.json`):
```json
{
  "cargo": {
    "command": "bazel",
    "package": "cpp_service"
  },
  "bazel": {
    "workspace": "monorepo"
  }
}
```

### Example 4: Using Command Syntax for One-Off Overrides

You can also specify overrides directly in the command:

```bash
# Run with custom test filter
cargo runner run src/lib.rs:42.test_name

# Run all tests in a module
cargo runner run src/lib.rs:42.module_name.

# Run with specific binary
cargo runner run src/main.rs:1.binary_name
```

### Example 5: Advanced Bazel Framework Configuration

```json
{
  "cargo": {
    "command": "bazel"
  },
  "bazel": {
    "test_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output=streamed"],
      "test_args": ["--exact", "{test_filter}"],
      "extra_env": {
        "RUST_LOG": "debug"
      }
    },
    "binary_framework": {
      "command": "bazel",
      "subcommand": "run",
      "target": "{target}",
      "exec_args": ["{binary_args}"]
    }
  }
}
```

## Placeholder System (Bazel)

The Bazel configuration supports placeholders in framework configurations:

- `{target}`: The full Bazel target (e.g., "//src:test")
- `{target_name}`: Just the target name (e.g., "test")
- `{package}`: The package path (e.g., "//src")
- `{file_path}`: Full file path
- `{file_name}`: File name without extension
- `{test_filter}`: Test name or filter
- `{module_path}`: Rust module path
- `{binary_name}`: Binary name

## Best Practices

1. **Start Simple**: Begin with basic configuration and add overrides as needed
2. **Use Specific Matches**: More specific matches have higher priority
3. **Test Your Config**: Use `RUST_LOG=debug` to see which overrides are being applied
4. **Version Control**: Commit `.cargo-runner.json` files to share configuration with your team
5. **Package-Level Config**: Place configs in subdirectories for package-specific settings

## Debugging Configuration

To see which configuration is being used:

```bash
# Enable debug logging
RUST_LOG=debug cargo runner run src/lib.rs:42

# Check which overrides match
RUST_LOG=debug cargo runner run src/lib.rs:42 2>&1 | grep "override"
```

## Migration from Old Format

If you have old-style configurations, here's how to migrate:

Old:
```json
{
  "overrides": [
    {
      "cargo": {
        "extra_args": ["--nocapture"]
      },
      "matcher": {  // Note: "matcher" instead of "match"
        "function_name": "test_foo"
      }
    }
  ]
}
```

New:
```json
{
  "overrides": [
    {
      "match": {  // Note: "match" instead of "matcher"
        "function_name": "test_foo"
      },
      "cargo": {
        "extra_test_binary_args": ["--nocapture"]  // For test args
      }
    }
  ]
}
```

## Common Issues

1. **Overrides not applying**: Check that your match criteria are correct. File paths must be absolute.
2. **Bazel targets not found**: Ensure BUILD.bazel files are in the correct locations
3. **Wrong build system**: Make sure `"command": "bazel"` is set for Bazel projects
4. **Test args not working**: Use `extra_test_args` for Bazel, `extra_test_binary_args` for Cargo