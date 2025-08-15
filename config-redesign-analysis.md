# Config Redesign Analysis

## Build System Analysis

### Common Aspects (All Build Systems)
```
- Execute a command (cargo/rustc/bazel)
- Handle runnable types (test/binary/bench/doctest)
- Support environment variables
- Support additional arguments
- Need file/target resolution
```

### Build System Specifics

#### Cargo & Single File Script
- **Command**: `cargo` (default)
- **Structure**: `cargo [subcommand] [args] -- [binary_args]`
- **Examples**:
  - Test: `cargo test --test mytest -- --nocapture`
  - Binary: `cargo run --bin mybin -- --port 8080`
  - Bench: `cargo bench --bench mybench`
  - Doc: `cargo test --doc`

#### Rustc
- **Command**: `rustc` + execution
- **Structure**: Two-phase
  - Build: `rustc [args] -o output`
  - Exec: `./output [args]`
- **Examples**:
  - Test: `rustc --test -o test_binary && ./test_binary`
  - Binary: `rustc -o binary && ./binary`

#### Bazel
- **Command**: `bazel`
- **Structure**: `bazel [subcommand] [target] [args]`
- **Target-based** not flag-based
- **Examples**:
  - Test: `bazel test //src:mytest --test_output=streamed`
  - Binary: `bazel run //src:server -- --port 8080`
  - Bench: `bazel test //bench:perf --test_arg=--bench`

## Key Insights

1. **Commands are Typed**: A command configuration is ALWAYS for a specific runnable type
   - You configure "how to run tests" not "how to run cargo"
   - The runnable detection determines which config applies

2. **Target Selection vs Command Type**:
   - Command type: Determined by runnable (test/binary/bench)
   - Target selection: Can be mixed (`--bin foo --test bar`)
   - Some exclusions: `--test` and `--doc` are mutually exclusive

3. **Inheritance Hierarchy**:
   ```
   Global defaults
   └── Build system defaults (cargo/bazel/rustc)
       └── Runnable type config (test/binary/bench/doc)
           └── File-level overrides
               └── Function-level overrides
   ```

## Proposed Unified Config Structure

### Design Principles
1. **Runnable-type first**: Config is organized by what you're running
2. **Build system agnostic**: Common interface, system-specific details
3. **Inheritance**: Each level inherits from parent
4. **Minimal repetition**: Smart defaults

### Structure

```json
{
  // Global defaults (apply to everything)
  "defaults": {
    "env": {
      "RUST_LOG": "debug"
    },
    "channel": "stable"
  },

  // Runnable type configurations
  "test": {
    // Default test configuration
    "cargo": {
      "subcommand": "test",
      "args": ["--verbose"]
    },
    "bazel": {
      "args": ["--test_output=streamed"],
      "test_args": ["--exact", "{test_filter}"]
    },
    "rustc": {
      "build": { "args": ["--test"] },
      "exec": { "args": ["{test_filter}"] }
    }
  },

  "binary": {
    // Default binary configuration
    "cargo": {
      "subcommand": "run",
      "args": ["--release"]
    },
    "bazel": {
      "subcommand": "run"
    },
    "rustc": {
      "build": { "args": ["-C", "opt-level=3"] },
      "exec": {}
    }
  },

  "bench": {
    // Default benchmark configuration
    "cargo": {
      "subcommand": "bench"
    },
    "bazel": {
      "subcommand": "test",
      "args": ["--test_arg=--bench"]
    }
  },

  "doc": {
    // Default doc test configuration
    "cargo": {
      "subcommand": "test",
      "args": ["--doc"]
    }
  },

  // Overrides (specific locations)
  "overrides": [
    {
      "match": {
        "path": "tests/integration/*",
        "type": "test"  // Only applies to tests
      },
      "env": {
        "TEST_ENV": "integration"
      },
      "cargo": {
        "args": ["--test-threads=1"]
      }
    },
    {
      "match": {
        "function": "bench_*",
        "type": "bench"
      },
      "cargo": {
        "args": ["--features=bench-utils"]
      }
    }
  ]
}
```

### Alternative: Even Simpler

```json
{
  // Smart defaults for common cases
  "profiles": {
    "default": {
      "test": "cargo test",
      "binary": "cargo run --release",
      "bench": "cargo bench"
    },
    "nextest": {
      "test": "cargo nextest run"
    },
    "bazel-mono": {
      "test": "bazel test {target} --test_output=all",
      "binary": "bazel run {target}"
    }
  },

  // Use a profile
  "use": "nextest",

  // Override specific things
  "overrides": {
    "integration/*": {
      "test": {
        "env": { "DATABASE_URL": "test://db" }
      }
    }
  }
}
```

### Benefits of This Approach

1. **Clear Mental Model**: "How do I run tests?" not "How do I configure cargo's test framework?"
2. **Build System Agnostic**: Same config concepts work across cargo/bazel/rustc
3. **Type Safety**: Can't accidentally apply test config to binaries
4. **Inheritance**: Natural hierarchy reduces repetition
5. **Extensible**: Easy to add new build systems or runnable types

### Examples

#### Simple: Just use nextest
```json
{
  "test": {
    "cargo": { "subcommand": "nextest run" }
  }
}
```

#### Complex: Multiple build systems
```json
{
  "test": {
    "cargo": { 
      "subcommand": "nextest run",
      "env": { "NEXTEST_PROFILE": "ci" }
    },
    "bazel": {
      "args": ["--test_output=all", "--test_timeout=300"]
    }
  },
  "overrides": [{
    "match": { "path": "crates/gpu/*" },
    "test": {
      "cargo": { "features": ["cuda"] }
    }
  }]
}
```

#### Per-function override
```json
{
  "overrides": [{
    "match": {
      "function": "test_database_*",
      "type": "test"
    },
    "env": {
      "DATABASE_URL": "postgres://test@localhost/test"
    },
    "cargo": {
      "args": ["--test-threads=1"]
    }
  }]
}
```

## Migration Path

1. Current config keys map naturally:
   - `cargo.test_framework` → `test.cargo`
   - `cargo.binary_framework` → `binary.cargo`
   - `bazel.benchmark_framework` → `bench.bazel`

2. Build system detection remains the same

3. Override matching gets a `type` field to specify which runnable type

This structure better reflects how the tool is actually used: you're configuring how to run different types of code, not configuring build systems.