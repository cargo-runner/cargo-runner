# Rustc Runtime Flags Clarification

## The Key Understanding

For rustc test and benchmark binaries, runtime flags like `--no-capture`, `--exact`, `--bench` etc. go in the `extra_args` field, NOT in `extra_test_binary_args`.

## Why?

Unlike cargo test which uses a `--` separator between cargo args and test binary args, rustc test binaries accept all flags mixed together without any separator.

## Correct Configuration Structure

```json
{
  "rustc": {
    "test_framework": {
      "channel": "stable",
      "extra_args": [
        "--test",           // Compilation flag
        "--edition", "2021", // Compilation flag
        "--no-capture",     // Runtime flag (but goes here!)
        "--exact"           // Runtime flag (but goes here!)
      ],
      "extra_test_binary_args": []  // Keep empty but present for consistency
    }
  }
}
```

## Command Execution

When running:
```bash
# Compile
rustc --test test.rs -o test_bin --edition 2021

# Run - runtime flags are passed directly
./test_bin tests::test_alpha --no-capture --exact
```

The runtime flags (`--no-capture`, `--exact`) are passed directly to the test binary without any `--` separator.

## Consistency with Cargo Config

We keep `extra_test_binary_args` in the structure for consistency with the cargo configuration, but for rustc it remains empty because all args go in `extra_args`.

## Examples

### Test Framework
```json
"test_framework": {
  "extra_args": ["--test", "--cfg", "test", "--no-capture", "--test-threads=1"],
  "extra_test_binary_args": []  // Always empty for rustc
}
```

### Benchmark Framework  
```json
"benchmark_framework": {
  "channel": "nightly",
  "extra_args": ["--test", "-Z", "unstable-options", "--bench", "--report-time"],
  "extra_test_binary_args": []  // Always empty for rustc
}
```

## Override Example
```json
"overrides": [{
  "match": {
    "module_path": "integration_tests"
  },
  "rustc": {
    "test_framework": {
      "extra_args": ["--test-threads=1", "--no-capture"],  // Runtime flags here
      "extra_test_binary_args": []  // Not here!
    }
  }
}]
```

## Key Takeaway

For rustc configurations:
- All args (compilation AND runtime) go in `extra_args`
- `extra_test_binary_args` stays empty but is kept for structural consistency
- No `--` separator is used when passing args to test binaries