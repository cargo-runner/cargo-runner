# RustC Configuration: Complex vs Simple

## ❌ Complex (Current) - DON'T USE THIS

```json
{
  "rustc": {
    "test_framework": {
      "build": {
        "command": "rustc",
        "args": ["--test", "{source_file}", "-o", "{output_name}"],
        "extra_args": ["--cfg", "test", "--edition", "2021"]
      },
      "exec": {
        "command": "./{output_name}",
        "args": ["{test_name}"],
        "extra_test_binary_args": ["--nocapture"]
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "args": ["--crate-type", "bin", "--crate-name", "{crate_name}", "{source_file}", "-o", "{output_name}"],
        "extra_args": ["-O"]
      },
      "exec": {
        "command": "./{output_name}",
        "args": []
      }
    }
  }
}
```

### Problems:
- Too complex
- Easy to break command order
- Have to understand rustc internals
- Duplicate settings for test vs binary

## ✅ Simple (Proposed) - USE THIS

```json
{
  "rustc": {
    "extra_args": ["--edition", "2021", "-O"],
    "extra_test_args": ["--cfg", "test"],
    "extra_test_binary_args": ["--nocapture"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    }
  }
}
```

### Benefits:
- Simple and clear
- Can't break command order
- Automatic deduplication
- Same args work for all rustc commands

## How It Works

### For Binary Compilation:
```bash
# System builds: rustc file.rs -o output
# With your extra_args: rustc file.rs --edition 2021 -O -o output
```

### For Test Compilation:
```bash
# System builds: rustc --test file.rs -o output
# With your args: rustc --test file.rs --edition 2021 -O --cfg test -o output
#                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ merged & deduped
```

### For Test Execution:
```bash
# System builds: ./output test_name
# With your args: ./output test_name -- --nocapture
```

## Override Examples

### Global Settings
```json
{
  "rustc": {
    "extra_args": ["--edition", "2021"],
    "extra_test_args": ["--cfg", "test"],
    "extra_test_binary_args": ["--nocapture"]
  }
}
```

### File-Specific Override
```json
{
  "overrides": [{
    "match": {
      "file_type": "Standalone",
      "file_path": "/path/to/file.rs"
    },
    "rustc": {
      "extra_args": ["-O", "--verbose"]
    }
  }]
}
```

### Test-Specific Override
```json
{
  "overrides": [{
    "match": {
      "file_type": "Standalone",
      "function_name": "test_special"
    },
    "rustc": {
      "extra_test_binary_args": ["--exact", "--show-output"]
    }
  }]
}
```

## Deduplication Examples

Input:
```json
{
  "extra_args": ["--edition", "2018", "-O"],
  "extra_test_args": ["--edition", "2021", "--cfg", "test"]
}
```

Result for test compilation:
```bash
rustc --test file.rs --edition 2021 -O --cfg test -o output
#                    ^^^^^^^^^^^^^^^ 2021 wins (last one)
```

## Migration

If you have old complex configs, they'll still work. But for new configs, just use the simple format!