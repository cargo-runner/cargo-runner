# Config Simplification Proposal

## Current Pain Points

1. **Too Much Nesting**: `cargo.test_framework.command` requires 3 levels of navigation
2. **Repetitive Structures**: test_framework, binary_framework, benchmark_framework all have identical fields
3. **Inconsistent Patterns**: Rustc uses build/exec phases, others don't
4. **Legacy Cruft**: Bazel has both old fields and new framework approach
5. **Verbose for Simple Cases**: Need lots of JSON for basic overrides

## Simplification Proposals

### Proposal 1: Minimal Config with Smart Defaults

Most users just want to:
- Use nextest instead of cargo test
- Add some env vars
- Maybe use nightly

```json
{
  "defaults": {
    "test": "nextest",
    "channel": "nightly",
    "env": {
      "RUST_LOG": "debug"
    }
  },
  "overrides": [{
    "match": "tests/*",
    "env": { "TEST_ENV": "integration" }
  }]
}
```

### Proposal 2: Unified Runner Config

Instead of separate sections per build system:

```json
{
  "runners": {
    "test": {
      "cargo": { "subcommand": "nextest run" },
      "bazel": { "args": ["--test_output=all"] },
      "rustc": { "build": "--test", "exec": "{output}" }
    },
    "binary": {
      "cargo": { "args": ["--release"] },
      "bazel": { "target": "//:server" }
    }
  }
}
```

### Proposal 3: Template-Based System

Use templates to reduce repetition:

```json
{
  "templates": {
    "nextest": {
      "command": "cargo",
      "subcommand": "nextest run",
      "env": { "NEXTEST_PROFILE": "ci" }
    }
  },
  "config": {
    "test": { "use": "nextest" },
    "overrides": [{
      "match": "integration/*",
      "test": { 
        "use": "nextest",
        "env": { "DATABASE_URL": "..." }
      }
    }]
  }
}
```

### Proposal 4: Convention-Based Minimal Config

Assume sensible defaults, only specify differences:

```json
{
  "test": "nextest",  // Implies cargo nextest run for tests
  "features": "all",   // Implies --all-features
  "overrides": {
    "integration/*": {
      "env": { "TEST_MODE": "integration" }
    },
    "benches/*": {
      "profile": "release"
    }
  }
}
```

## Recommended Approach: Hybrid Simplification

### 1. Keep Current Structure but Add Shortcuts

```json
{
  // Shorthand for common cases
  "test": "nextest",
  "binary": "release",
  
  // Full control when needed
  "cargo": {
    "test_framework": {
      // Detailed config
    }
  }
}
```

### 2. Single Framework Config with Type

Instead of separate test_framework, binary_framework:

```json
{
  "cargo": {
    "frameworks": [{
      "for": ["test", "benchmark"],
      "command": "cargo",
      "subcommand": "nextest",
      "args": ["run"]
    }, {
      "for": ["binary"],
      "args": ["--release"]
    }]
  }
}
```

### 3. Global Defaults Section

```json
{
  "defaults": {
    "channel": "nightly",
    "features": "all",
    "env": {
      "RUST_LOG": "debug"
    }
  },
  // Build systems only specify overrides from defaults
  "cargo": {
    "test": { "subcommand": "nextest" }
  }
}
```

## Benefits of Simplification

1. **Easier to Write**: Less nesting, fewer fields
2. **Easier to Read**: Clear what's being configured
3. **Less Error-Prone**: Fewer places to make mistakes
4. **Better Defaults**: Works out of box for common cases
5. **Still Flexible**: Can drop down to detailed config when needed

## Migration Strategy

1. **Phase 1**: Support both old and new formats
2. **Phase 2**: Auto-migrate old configs to new format
3. **Phase 3**: Deprecate old format with warnings
4. **Phase 4**: Remove old format support

## Example: Current vs Simplified

### Current (Verbose)
```json
{
  "cargo": {
    "channel": "nightly",
    "features": "all",
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest",
      "channel": "nightly",
      "features": "all",
      "extra_args": ["run"]
    },
    "binary_framework": {
      "command": "cargo",
      "subcommand": "run",
      "channel": "nightly",
      "features": "all",
      "extra_args": ["--release"]
    }
  }
}
```

### Simplified (Option A - Inheritance)
```json
{
  "cargo": {
    "channel": "nightly",
    "features": "all",
    "test": { "subcommand": "nextest run" },
    "binary": { "args": ["--release"] }
  }
}
```

### Simplified (Option B - Presets)
```json
{
  "preset": "nextest-nightly-all",
  "binary": { "profile": "release" }
}
```

### Simplified (Option C - Smart Strings)
```json
{
  "test": "nextest@nightly",
  "binary": "release",
  "features": "all"
}
```

## Recommendation

I recommend **Option A (Inheritance)** because:
1. It maintains backward compatibility
2. It reduces repetition through inheritance
3. It's still explicit about what's being configured
4. It's easier to implement incrementally

The key improvements would be:
- Flatten framework configs to just `test`, `binary`, `bench` instead of `test_framework`
- Have settings inherit from parent level
- Remove legacy Bazel fields
- Add shorthand notations for common patterns