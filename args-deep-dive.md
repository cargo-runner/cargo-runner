# Deep Dive: Argument Passing Patterns

## Cargo Pattern

```bash
cargo [subcommand] [cargo-flags] -- [binary-args]

# The -- separator divides:
# - Left side: Arguments TO cargo
# - Right side: Arguments THROUGH cargo to the binary

# Examples:
cargo test --lib --release -- --nocapture
           ^^^^^^^^^^^^^^     ^^^^^^^^^^^^
           cargo processes    passed to test binary

cargo run --bin server --release -- --port 8080
          ^^^^^^^^^^^^^^^^^^^^^^     ^^^^^^^^^^^
          cargo processes            passed to server binary
```

## Bazel Pattern

```bash
# For TEST command:
bazel test [target] [bazel-flags] --test_arg=[value]

# For RUN command:
bazel run [target] [bazel-flags] -- [binary-args]

# Examples:
bazel test //src:my_test --test_output=all --test_arg=--nocapture --test_arg=--exact --test_arg=my_test
                         ^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                         bazel processes      each becomes an arg to test binary

bazel run //src:server --compilation_mode=opt -- --port 8080
                       ^^^^^^^^^^^^^^^^^^^^^^     ^^^^^^^^^^^
                       bazel processes            passed to server binary
```

## Key Insight: Bazel's Inconsistency

**Bazel `test`**: Uses `--test_arg=` prefix for EACH argument
**Bazel `run`**: Uses `--` separator like Cargo

This is because:
- `bazel test` is a test runner that needs to distinguish test args
- `bazel run` just executes and passes args through

## Let's Test Our Understanding

### Scenario 1: Running a specific test with options

**What we want the test binary to receive**: `--exact my_test --nocapture`

**How to achieve it**:
```bash
# Cargo
cargo test -- --exact my_test --nocapture

# Bazel
bazel test //target --test_arg=--exact --test_arg=my_test --test_arg=--nocapture

# Rustc (after building)
./test_binary --exact my_test --nocapture
```

### Scenario 2: Running a binary with arguments

**What we want the binary to receive**: `--port 8080 --workers 4`

**How to achieve it**:
```bash
# Cargo
cargo run -- --port 8080 --workers 4

# Bazel
bazel run //target -- --port 8080 --workers 4

# Rustc (after building)
./binary --port 8080 --workers 4
```

## The Real Question: How to Model This?

### Option A: Expose the Difference
```json
{
  "test": {
    "cargo": {
      "args": ["--release"],
      "test_binary_args": ["--nocapture"]  // After --
    },
    "bazel": {
      "args": ["--test_output=all"],
      "test_args": ["--nocapture"]  // Becomes --test_arg=
    }
  }
}
```
**Problem**: Users need to know Bazel uses different field name

### Option B: Hide the Difference
```json
{
  "test": {
    "args": ["--release"],  // Build system args
    "test_binary_args": ["--nocapture"]  // Args to test binary
  }
}
```
**System translates**:
- Cargo: `cargo test --release -- --nocapture`
- Bazel: `bazel test //target --release --test_arg=--nocapture`

### Option C: Semantic Naming
```json
{
  "test": {
    "build_flags": ["--release"],
    "filter": "--exact my_test",
    "runtime_flags": ["--nocapture", "--test-threads=1"]
  }
}
```

## Wait, What About Complex Bazel Args?

Bazel has multiple argument types:
1. **Bazel flags**: `--test_output=all`, `--cache_test_results=no`
2. **Test filter**: Built into bazel with `--test_filter` (different from --test_arg)
3. **Test binary args**: Via `--test_arg=`

Example:
```bash
bazel test //src:all \
  --test_output=all \           # Bazel flag
  --test_filter=my_test \        # Bazel's built-in filter
  --test_arg=--nocapture \       # Passed to test binary
  --test_arg=--test-threads=1    # Passed to test binary
```

## Aha! The Consolidation Challenge

The challenge isn't just `--test_arg` vs `--`:

1. **Cargo** conflates test filtering and binary args (both after `--`)
2. **Bazel** separates them:
   - Test selection: `--test_filter` (Bazel feature)
   - Test binary args: `--test_arg=` (passed through)

3. **Different features available**:
   - Cargo: Relies on test binary for filtering
   - Bazel: Has built-in test filtering, retries, sharding

## Proposed Unified Model

```json
{
  "test": {
    // Universal concepts
    "build_args": ["--release", "--all-features"],
    "runtime_args": ["--nocapture", "--test-threads=1"],
    
    // Build system specific features
    "cargo": {
      // Cargo-specific
    },
    "bazel": {
      "bazel_args": ["--test_output=all", "--runs_per_test=3"],
      "use_test_filter": true  // Use --test_filter instead of --test_arg
    }
  }
}
```

Mapping:
- **build_args**: Go to build system (before `--` or as bazel flags)
- **runtime_args**: Go to binary (after `--` or via `--test_arg`)
- **bazel_args**: Bazel-specific flags that have no cargo equivalent

## Questions to Resolve

1. Should we expose Bazel's `--test_filter` vs `--test_arg` distinction?
2. Should we have a unified "filter" concept that maps differently?
3. How do we handle Bazel-only features like `--runs_per_test`?

The key is: **What mental model do we want users to have?**