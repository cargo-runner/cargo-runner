# Argument Passing Analysis

## Current Argument Passing Patterns

### Cargo
```bash
cargo test [cargo_args] -- [test_binary_args]
cargo run [cargo_args] -- [exec_args]
cargo bench [cargo_args] -- [bench_args]

# Examples:
cargo test --release --features foo -- --nocapture --test-threads=1
cargo run --bin server -- --port 8080 --workers 4
```

### Bazel
```bash
bazel test [target] [bazel_args] --test_arg=[test_args]
bazel run [target] [bazel_args] -- [exec_args]

# Examples:
bazel test //src:test --test_output=all --test_arg=--exact --test_arg=my_test
bazel run //src:server --compilation_mode=opt -- --port 8080
```

### Rustc
```bash
# Build phase
rustc [rustc_args] -o output
# Exec phase  
./output [exec_args]

# Example:
rustc --test -C opt-level=2 -o test_bin
./test_bin --exact my_test --nocapture
```

## The Problem

Each build system has different ways to pass arguments:
1. **Cargo**: Uses `--` separator for binary args
2. **Bazel**: Uses `--test_arg` for each test argument (test only), `--` for run
3. **Rustc**: Arguments go directly to the binary

## Current Config Approach (Problematic)

```json
{
  "cargo": {
    "extra_args": ["--release"],           // Before --
    "extra_test_binary_args": ["--nocapture"] // After --
  },
  "bazel": {
    "extra_args": ["--test_output=all"],   // Bazel args
    "test_args": ["--exact", "{test_filter}"]  // Becomes --test_arg=...
  }
}
```

This is confusing because:
- `extra_args` means different things
- `test_args` only exists for Bazel
- `extra_test_binary_args` doesn't map to Bazel well

## Proposed Unified Approach

### Option 1: Universal Argument Categories

```json
{
  "test": {
    "build_args": ["--release"],        // Args to build system
    "filter_args": ["{test_filter}"],   // Test selection
    "runtime_args": ["--nocapture"]     // Runtime behavior
  }
}
```

This maps to:
- **Cargo**: `cargo test {build_args} -- {filter_args} {runtime_args}`
- **Bazel**: `bazel test {target} {build_args} --test_arg={filter_args} --test_arg={runtime_args}`
- **Rustc**: `rustc --test {build_args} && ./output {filter_args} {runtime_args}`

### Option 2: Semantic Argument Types

```json
{
  "test": {
    "args": {
      "build": ["--release", "--features=foo"],
      "filter": ["--exact", "{test_filter}"],
      "runtime": ["--nocapture", "--test-threads=1"],
      "env": { "RUST_LOG": "debug" }
    }
  }
}
```

### Option 3: Build System Aware (Current Style, Clarified)

```json
{
  "test": {
    "cargo": {
      "args": ["--release"],              // Cargo args
      "binary_args": ["--nocapture"]      // After --
    },
    "bazel": {
      "args": ["--test_output=all"],     // Bazel args
      "test_args": ["--exact", "{test_filter}"]  // Via --test_arg
    },
    "rustc": {
      "build": { "args": ["--test"] },
      "exec": { "args": ["{test_filter}", "--nocapture"] }
    }
  }
}
```

### Option 4: Smart Mapping System

```json
{
  "test": {
    // Universal args that get mapped appropriately
    "args": ["--release", "--all-features"],
    "test_binary_args": ["--nocapture", "--test-threads=1"],
    
    // Build-system specific overrides when needed
    "bazel": {
      "args": ["--test_output=all"]  // Bazel-specific
    }
  }
}
```

The system would know:
- For Cargo: `args` go before `--`, `test_binary_args` after
- For Bazel test: `test_binary_args` become `--test_arg=...`
- For Bazel run: `test_binary_args` go after `--`
- For Rustc: `test_binary_args` go to exec phase

## Detailed Mapping Examples

### Test Execution

Config:
```json
{
  "test": {
    "args": ["--release"],
    "test_binary_args": ["--exact", "my_test", "--nocapture"]
  }
}
```

Becomes:
- **Cargo**: `cargo test --release -- --exact my_test --nocapture`
- **Bazel**: `bazel test //target --release --test_arg=--exact --test_arg=my_test --test_arg=--nocapture`
- **Rustc**: `rustc --test --release -o out && ./out --exact my_test --nocapture`

### Binary Execution

Config:
```json
{
  "binary": {
    "args": ["--release"],
    "exec_args": ["--port", "8080"]
  }
}
```

Becomes:
- **Cargo**: `cargo run --release -- --port 8080`
- **Bazel**: `bazel run //target --release -- --port 8080`
- **Rustc**: `rustc --release -o out && ./out --port 8080`

## Recommendation: Option 4 (Smart Mapping)

This approach:
1. **Provides a unified interface** for common cases
2. **Maps intelligently** to each build system
3. **Allows overrides** when build systems differ significantly
4. **Maintains clarity** about what args do

### Full Example with Smart Mapping:

```json
{
  "defaults": {
    "env": { "RUST_LOG": "info" }
  },
  
  "test": {
    // Universal configuration
    "args": ["--all-features"],
    "test_binary_args": ["--nocapture"],
    
    // Build-system specific additions/overrides
    "cargo": {
      "subcommand": "nextest run"
    },
    "bazel": {
      "args": ["--test_output=streamed", "--test_timeout=300"]
      // test_binary_args automatically become --test_arg=...
    }
  },
  
  "binary": {
    "args": ["--release"],
    "exec_args": ["--config", "production.toml"],
    
    "bazel": {
      "target": "//:server"  // Bazel-specific target
    }
  },
  
  "overrides": [{
    "match": { "function": "test_flaky_*" },
    "test": {
      "test_binary_args": ["--test-threads=1"],
      "bazel": {
        "args": ["--runs_per_test=3"]  // Bazel-specific retry
      }
    }
  }]
}
```

This gives us:
- **Simplicity** for common cases (just set args/test_binary_args)
- **Power** for complex cases (override per build system)
- **Clarity** about what each option does
- **Automatic mapping** of test_binary_args to --test_arg for Bazel