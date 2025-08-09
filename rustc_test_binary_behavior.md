# Understanding Rustc Test Binary Behavior

## Key Discoveries

### 1. The `--` separator is NOT used!

Unlike `cargo test`, rustc-compiled test binaries do NOT use `--` to separate test names from flags.

**Wrong:**
```bash
./test_binary test_alpha -- --nocapture  # The -- is ignored!
```

**Correct:**
```bash
./test_binary test_alpha --nocapture     # Flags go directly after test name
```

### 2. Available Options for Rustc Test Binaries

```
--include-ignored       Run ignored and not ignored tests
--ignored              Run only ignored tests
--test                 Run tests and not benchmarks
--bench                Run benchmarks instead of tests
--list                 List all tests and benchmarks
--no-capture           Don't capture stdout/stderr (NOT --nocapture!)
--test-threads n       Number of threads for parallel execution
--skip FILTER          Skip tests containing FILTER
--quiet                Display one character per test
--exact                Exactly match filters (not substring)
--color auto|always|never
--format pretty|terse|json|junit
--show-output          Show captured stdout of successful tests
--shuffle              Run tests in random order
--shuffle-seed SEED    Seed for random order
```

### 3. Important Differences from Cargo Test

| Feature | Cargo Test | Rustc Test Binary |
|---------|-----------|-------------------|
| Separator | Uses `--` | No separator needed |
| nocapture flag | `--nocapture` | `--no-capture` |
| Arguments order | `cargo test TEST -- FLAGS` | `./binary TEST FLAGS` |

### 4. How Rustc --test Works

- `rustc --test file.rs -o output` creates a test harness binary
- The binary can run specific tests by name (filter)
- Multiple filters can be passed as positional arguments
- All flags are mixed with test names (no separator)

## Examples

```bash
# Compile
rustc --test test.rs -o test_binary

# Run all tests
./test_binary

# Run specific test
./test_binary test_alpha

# Run with flags
./test_binary test_alpha --no-capture --exact

# Run multiple tests
./test_binary test_alpha test_beta

# Skip tests
./test_binary --skip alpha

# Show output of passing tests
./test_binary --show-output --no-capture
```

## Implications for Our Config

Since rustc test binaries don't use `--`, we need to update our command generation:

**Before (incorrect):**
```
./test_binary test_name -- --nocapture
```

**After (correct):**
```
./test_binary test_name --no-capture
```

Also note: The flag is `--no-capture` (with hyphen), not `--nocapture`!