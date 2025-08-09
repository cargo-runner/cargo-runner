# Config Override Quick Reference

## Override Matching Rules

The system matches overrides based on `FunctionIdentity`. All specified fields in the match must match for the override to apply.

### Match Fields

| Field | Description | Example |
|-------|-------------|---------|
| `package` | Cargo package name | `"my_crate"` |
| `module_path` | Full module path | `"my_crate::tests::unit"` |
| `file_path` | Absolute file path | `"/Users/name/project/src/lib.rs"` |
| `function_name` | Function name (supports wildcards) | `"test_auth"`, `"bench_*"` |
| `file_type` | Type of file | `"CargoProject"`, `"Standalone"`, `"SingleFileScript"` |

## Examples by Granularity

### 1. Package Level (Cargo only)
```json
{
  "match": {
    "package": "my_crate"
  },
  "cargo": {
    "channel": "nightly"
  }
}
```

### 2. File Level
```json
{
  "match": {
    "file_path": "/path/to/file.rs",
    "file_type": "Standalone"
  },
  "rustc": {
    "extra_args": ["-O"]
  }
}
```

### 3. Module Level
```json
{
  "match": {
    "package": "my_crate",
    "module_path": "my_crate::database::tests"
  },
  "cargo": {
    "extra_env": {
      "DATABASE_URL": "postgres://localhost/test"
    }
  }
}
```

### 4. Function Level
```json
{
  "match": {
    "package": "my_crate",
    "function_name": "test_specific_function"
  },
  "cargo": {
    "extra_test_binary_args": ["--nocapture"]
  }
}
```

## Command Type Specific Overrides

### Cargo Projects
```json
"cargo": {
  "command": "cargo",
  "subcommand": "test",
  "channel": "nightly",
  "features": {
    "selected": ["async", "serde"]
  },
  "extra_args": ["--release"],
  "extra_test_binary_args": ["--nocapture"],
  "extra_env": {
    "RUST_LOG": "debug"
  }
}
```

### Standalone Rust Files
```json
"rustc": {
  "extra_args": ["-O", "--edition", "2021"],
  "extra_env": {
    "RUST_BACKTRACE": "1"
  },
  "test_framework": {
    "build": {
      "command": "rustc",
      "args": ["--test", "{source_file}", "-o", "{output_name}"],
      "extra_args": ["--cfg", "test"]
    },
    "exec": {
      "command": "./{output_name}",
      "args": ["{test_name}"],
      "extra_test_binary_args": ["--nocapture"]
    }
  }
}
```

### Single File Scripts
```json
"single_file_script": {
  "extra_args": ["--verbose"],
  "extra_env": {
    "SCRIPT_VAR": "value"
  }
}
```

## Testing Your Overrides

1. Create a `.cargo-runner.json` in your project
2. Use the `-c` flag with analyze to see config details:
   ```bash
   cargo runner analyze file.rs:line -c
   ```
3. Look for "Matched override:" in the output

## Priority Order

More specific matches override less specific ones:
1. Function + Module + File + Package
2. Function + File
3. Function + Module
4. Module + File
5. File only
6. Module only
7. Package only
8. File type only

## Legacy Support

Old configs with root-level cargo settings are automatically migrated:
```json
{
  "command": "cargo",
  "extra_args": ["--release"]
}
```
Becomes:
```json
{
  "cargo": {
    "command": "cargo",
    "extra_args": ["--release"]
  }
}
```