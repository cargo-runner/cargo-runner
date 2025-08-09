# Understanding Rustc Compilation Modes

## The Key Insight

`rustc` behaves VERY differently depending on whether you use `--test`:

### Without `--test` (Binary Mode)
```bash
rustc file.rs -o binary
./binary
```
- Creates a regular executable
- No `--help` flag
- No test filtering
- No special runtime flags
- Just runs the `main()` function

### With `--test` (Test Mode)
```bash
rustc --test file.rs -o test_binary
./test_binary --help
```
- Creates a test harness
- Has `--help` with many options
- Supports test filtering
- Accepts flags like `--no-capture`, `--exact`, etc.
- Runs `#[test]` functions, NOT `main()`

## Real Examples from Testing

### 1. Running Individual Tests
```bash
# Tests are in a module, so use full path
./test tests::test_alpha
./test tests::test_beta
./test tests::main  # Yes, you can have a test named main!
```

### 2. Using Flags (with `--` separator)
```bash
# Filter goes first, then --, then flags
./test tests::test_alpha -- --exact
./test tests::test_alpha -- --no-capture --exact
```

### 3. Listing Tests
```bash
./test --list
# Output:
# tests::main: test
# tests::test_alpha: test  
# tests::test_beta: test
```

### 4. Benchmarks (Requires Nightly)
```bash
# Compile with nightly
rustc +nightly --test benchmark.rs -o bench_bin

# Run all benchmarks
./bench_bin --bench

# Run specific benchmark
./bench_bin --bench benches::bench_fib_20

# With unstable options
./bench_bin -Z unstable-options --bench benches::bench_fib_20 --format json
```

## Why This Matters for Config

We need THREE different modes:

1. **Binary Mode**: Simple compilation, no test features
2. **Test Mode**: Test harness with filtering and flags
3. **Bench Mode**: Benchmark harness (requires nightly)

Each mode needs:
- `channel`: Which Rust version to use
- `compile_args`: Args for rustc compilation
- `runtime_args`: Args for running the binary

## Config Structure That Makes Sense

```json
{
  "rustc": {
    "binary_mode": {
      "compile_args": ["-O"]
      // No runtime_args - regular binaries don't have special flags
    },
    "test_mode": {
      "compile_args": ["--test", "--cfg", "test"],
      "runtime_args": ["--no-capture"]
    },
    "bench_mode": {
      "channel": "nightly",
      "compile_args": ["--test"],
      "runtime_args": ["--bench"]
    }
  }
}
```

## Common Mistakes to Avoid

1. **Assuming regular binaries have test features** - They don't!
2. **Wrong module paths** - Use `module::function` not just `function`
3. **Forgetting `--` separator** - Flags go after `--`
4. **Using stable for benchmarks** - Benchmarks need nightly