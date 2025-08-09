# Rustc Test Binary Flags Reference

## Important: No `--` Separator!

Unlike `cargo test`, rustc test binaries do NOT use `--` to separate test names from flags.

❌ **Wrong**: `./test test_name -- --no-capture`  
✅ **Correct**: `./test test_name --no-capture`

## Common Flags for extra_test_binary_args

| Flag | Description | Example |
|------|-------------|---------|
| `--no-capture` | Don't capture stdout/stderr | `["--no-capture"]` |
| `--exact` | Match test names exactly | `["--exact"]` |
| `--show-output` | Show output of successful tests | `["--show-output"]` |
| `--test-threads N` | Number of parallel threads | `["--test-threads=1"]` |
| `--quiet` | One character per test | `["--quiet"]` |
| `--list` | List all tests | `["--list"]` |
| `--ignored` | Run only ignored tests | `["--ignored"]` |
| `--include-ignored` | Run all tests including ignored | `["--include-ignored"]` |
| `--skip FILTER` | Skip tests containing FILTER | `["--skip", "slow"]` |

## Configuration Examples

### Basic Test Config
```json
{
  "rustc": {
    "extra_test_binary_args": ["--no-capture", "--test-threads=1"]
  }
}
```

### Function-Specific Override
```json
{
  "overrides": [{
    "match": {
      "file_type": "Standalone",
      "function_name": "test_integration"
    },
    "rustc": {
      "extra_test_binary_args": ["--exact", "--no-capture", "--show-output"]
    }
  }]
}
```

### Module-Level Config
```json
{
  "overrides": [{
    "match": {
      "module_path": "integration_tests"
    },
    "rustc": {
      "extra_test_binary_args": ["--test-threads=1", "--no-capture"]
    }
  }]
}
```

## Complete Command Example

Given config:
```json
{
  "rustc": {
    "extra_args": ["--edition", "2021"],
    "extra_test_args": ["--cfg", "test"],
    "extra_test_binary_args": ["--no-capture", "--exact"]
  }
}
```

Results in:
```bash
# Compilation
rustc --test test.rs --edition 2021 --cfg test -o test_output

# Execution
./test_output test_alpha --no-capture --exact
```

## Common Mistakes

1. **Using `--nocapture`** - It's `--no-capture` (with hyphen)
2. **Using `--` separator** - Not needed for rustc binaries
3. **Wrong order** - Flags go after test name, not before