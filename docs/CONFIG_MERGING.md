# Configuration Merging in cargo-runner

## Overview

cargo-runner supports a hierarchical configuration system that allows you to define settings at different levels:
- **Root level** (at PROJECT_ROOT)
- **Workspace level** (where `[workspace]` is defined in Cargo.toml)
- **Package level** (individual package directories)

## Merging Hierarchy

Configurations are merged in the following order:
1. Root config (from PROJECT_ROOT)
2. Workspace config (if applicable)
3. Package config

Later configurations override earlier ones, but by default, arrays and maps are **merged** rather than replaced.

## Configuration Structure

All config files use snake_case naming and only include fields with actual values:

```json
{
  "package": "my-package",
  "channel": "nightly",
  "extra_args": ["--release"],
  "env": {
    "RUST_LOG": "debug"
  },
  "extra_test_binary_args": ["--nocapture"],
  "test_frameworks": {
    "command": "cargo",
    "subcommand": "nextest run"
  },
  "overrides": []
}
```

## Special Fields

### linked_projects
- **Only allowed at root level** (PROJECT_ROOT)
- Lists all Cargo.toml files in the workspace
- Automatically populated by `cargo runner init`

### test_frameworks
- Allows customizing the test runner
- Supports commands like `cargo miri nextest run`
- Example:
  ```json
  "test_frameworks": {
    "command": "cargo",
    "subcommand": "miri nextest run",
    "channel": "nightly",
    "extra_args": ["-j10"],
    "env": {
      "MIRIFLAGS": "-Zmiri-disable-isolation"
    }
  }
  ```

## Merge vs Replace

By default, arrays and maps are merged:
- Arrays: Items from both configs are combined
- Maps: Keys from both configs are merged, with later values overriding

### Force Replace

Overrides support `force_replace` flags to replace instead of merge:

```json
{
  "overrides": [{
    "match": {
      "function_name": "test_foo"
    },
    "extra_args": ["--nocapture"],
    "force_replace_args": true,  // Replace args instead of merge
    "env": {
      "TEST_VAR": "value"
    },
    "force_replace_env": false   // Merge env vars (default)
  }]
}
```

## Example Hierarchy

```
PROJECT_ROOT/
├── .cargo-runner.json          # Root config with linked_projects
├── Cargo.toml                  # [workspace] definition
├── my-crate/
│   ├── .cargo-runner.json      # Package-specific config
│   └── Cargo.toml
└── another-crate/
    ├── .cargo-runner.json      # Different package config
    └── Cargo.toml
```

When running from `my-crate/src/lib.rs`:
1. Load root config (if PROJECT_ROOT is set)
2. Load workspace config (if different from root)
3. Load my-crate's config
4. Merge all configs in order

## Best Practices

1. **Use root config for** workspace-wide settings like:
   - Common environment variables
   - Default channels
   - linked_projects list

2. **Use package config for** package-specific settings:
   - Package-specific features
   - Test configurations
   - Function-specific overrides

3. **Use force_replace sparingly** - prefer merging for flexibility

4. **Keep configs minimal** - only include fields you need to override