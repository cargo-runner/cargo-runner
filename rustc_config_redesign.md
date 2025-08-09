# Rustc Configuration Redesign

Based on extensive testing, here's what we've learned:

## Key Discoveries

1. **The `--` separator is NOT used** for rustc test binaries (unlike cargo test)
2. **Module paths are required**: `tests::test_alpha` not just `test_alpha`
3. **Only `--test` compiled binaries have test features** (--help, filters, etc.)
4. **Benchmarks require nightly channel**
5. **Regular binaries (without --test) have NO test features**
6. **Runtime flags for test binaries go in `extra_args`** along with compilation args

## Current Issues

- `extra_test_binary_args` only makes sense for binaries compiled with `--test`
- Regular binaries don't support `--help` or any test flags
- Benchmarks need special handling (nightly + different compilation)
- The current structure doesn't make these distinctions clear

## Proposed New Structure

### Option 1: Mode-Based Configuration

```json
{
  "rustc": {
    "channel": "stable",  // default channel
    "extra_env": {
      "RUST_BACKTRACE": "1"
    },
    
    // Mode-specific configurations
    "binary": {
      "extra_args": ["--edition", "2021", "-O"]
      // No extra_binary_args because regular binaries don't have flags
    },
    
    "test": {
      "extra_args": ["--edition", "2021", "--cfg", "test", "--no-capture", "--test-threads=1"],
      "extra_binary_args": []  // Kept for consistency but empty
    },
    
    "bench": {
      "channel": "nightly",  // Override default channel
      "extra_args": ["--edition", "2021"],
      "extra_binary_args": ["--bench"]
    }
  }
}
```

### Option 2: Simplified with Smart Defaults

```json
{
  "rustc": {
    "extra_args": ["--edition", "2021"],
    "extra_test_args": ["--cfg", "test"],        // Added when compiling tests
    "extra_test_binary_args": ["--no-capture"],  // Used when running test binaries
    "extra_bench_args": [],                      // Added when compiling benchmarks
    "extra_bench_binary_args": [],               // Used when running bench binaries
    "test_channel": "stable",                    // Channel for tests
    "bench_channel": "nightly"                   // Channel for benchmarks
  }
}
```

### Option 3: Framework-Based Configuration (Coherent Naming)

```json
{
  "rustc": {
    "binary_framework": {
      "channel": "stable",
      "extra_args": ["--edition", "2021", "-O"],
      "extra_test_binary_args": []  // Not applicable for regular binaries
    },
    
    "test_mode": {
      "channel": "stable",
      "compile_args": ["--test", "--edition", "2021", "--cfg", "test", "--no-capture", "--test-threads=1"],
      "runtime_args": []  // Not used for test binaries
    },
    
    "bench_mode": {
      "channel": "nightly",
      "compile_args": ["--test", "--edition", "2021", "--bench"],
      "runtime_args": []  // Not used for bench binaries
    }
  }
}
```

## How Commands Are Built

### For Regular Binary (main function)
```bash
# Compile
rustc file.rs -o output [extra_args]

# Run
./output [user args if any]
```

### For Tests
```bash
# Compile (includes runtime flags)
rustc --test file.rs -o test_output [extra_args]

# Run all tests (runtime flags from extra_args are passed)
./test_output

# Run specific test
./test_output tests::test_alpha

# Runtime flags are mixed with test names (no -- separator)
./test_output tests::test_alpha --exact --no-capture
```

### For Benchmarks
```bash
# Compile (requires nightly, includes runtime flags)
rustc +nightly --test file.rs -o bench_output [extra_args]

# Run all benchmarks (flags from extra_args are passed)
./bench_output --bench

# Run specific benchmark
./bench_output --bench benches::bench_name
```

## Module Path Handling

For test functions inside modules, we need to build the full path:
- File: `test.rs`
- Module: `tests`
- Function: `test_alpha`
- Full path: `tests::test_alpha`

## Channel Support

Two ways to specify channel:
1. `rustc +nightly --test file.rs`
2. `rustup run nightly rustc --test file.rs`

## Recommendations

I recommend **Option 3** (Explicit Framework Configuration) because:
1. **Clear separation** between binary, test, and bench modes
2. **No confusion** about when args apply
3. **Explicit channel support** per mode
4. **Future proof** for other modes

The structure makes it obvious:
- Binary mode = simple compilation, no test features
- Test mode = test harness with filters and flags
- Bench mode = benchmark harness (requires nightly)

## Migration Path

For backward compatibility:
1. If old `test_framework` exists, use it
2. Otherwise, check for mode-specific config
3. Fall back to simple `extra_args` approach

## Support for Unstable Features

The extra_args can include `-Z` flags for unstable features:
```json
{
  "test_framework": {
    "channel": "nightly",
    "extra_args": ["--test", "-Z", "unstable-options", "--report-time", "--format", "json"],
    "extra_test_binary_args": []
  }
}
```

## Complete Example Usage

```json
{
  "rustc": {
    "binary_framework": {
      "channel": "stable",
      "extra_args": ["-O", "--edition", "2021"],
      "extra_test_binary_args": []
    },
    "test_framework": {
      "channel": "stable",
      "extra_args": ["--test", "--cfg", "test", "--no-capture", "--exact"],
      "extra_test_binary_args": []  // Kept for consistency
    },
    "benchmark_framework": {
      "channel": "nightly",
      "extra_args": ["--test", "-Z", "unstable-options", "--bench", "--report-time"],
      "extra_test_binary_args": []  // Kept for consistency
    }
  },
  "overrides": [{
    "match": {
      "file_type": "Standalone",
      "module_path": "integration_tests"
    },
    "rustc": {
      "test_framework": {
        "extra_args": ["--test-threads=1", "--no-capture"],
        "extra_test_binary_args": []
      }
    }
  }, {
    "match": {
      "file_type": "Standalone",
      "function_name": "bench_*"
    },
    "rustc": {
      "benchmark_framework": {
        "channel": "nightly",
        "extra_args": [
          "-Z", "unstable-options",
          "--bench", 
          "--report-time",
          "--format", "json"
        ],
        "extra_test_binary_args": []
      }
    }
  }]
}
```

## Channel Execution

Commands will be built as:
```bash
# With channel
rustc +nightly --test file.rs -o output

# Or using rustup
rustup run nightly rustc --test file.rs -o output

# Runtime with -Z flags
./output -Z unstable-options --bench --format json
```