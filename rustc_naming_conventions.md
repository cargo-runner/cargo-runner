# Rustc Configuration Naming Conventions

## Consistent Naming Pattern

Following the pattern we use elsewhere in the codebase:

### Current (Inconsistent)
- `binary_mode`
- `test_mode`  
- `bench_mode`
- `compile_args`
- `runtime_args`

### Proposed (Consistent with cargo config)
- `binary_framework`
- `test_framework`
- `benchmark_framework`
- `extra_args` (for compilation)
- `extra_binary_args` (for runtime)

## Aligned Structure

```json
{
  "rustc": {
    // Global defaults
    "channel": "stable",
    "extra_args": ["--edition", "2021"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    },
    
    // Framework-specific settings
    "binary_framework": {
      "channel": "stable",  // optional, overrides global
      "extra_args": ["-O"],  // compilation args
      "extra_binary_args": []  // runtime args (usually empty for binaries)
    },
    
    "test_framework": {
      "channel": "stable",
      "extra_args": ["--test", "--cfg", "test"],  
      "extra_binary_args": ["--no-capture", "--exact"]
    },
    
    "benchmark_framework": {
      "channel": "nightly",  // benchmarks need nightly
      "extra_args": ["--test"],
      "extra_binary_args": ["--bench"]
    }
  }
}
```

## Naming Consistency Across File Types

### Cargo Projects
```json
{
  "cargo": {
    "extra_args": [],         // cargo args
    "extra_test_binary_args": []  // test binary args
  }
}
```

### Standalone Files (Rustc)
```json
{
  "rustc": {
    "test_framework": {
      "extra_args": [],        // rustc args
      "extra_binary_args": []  // test binary args
    }
  }
}
```

### Single File Scripts
```json
{
  "single_file_script": {
    "extra_args": [],         // cargo script args
    "extra_binary_args": []   // script binary args (if any)
  }
}
```

## Key Principles

1. **Use `framework` not `mode`** - Consistent with existing `test_framework` in cargo config
2. **Use `extra_` prefix** - All additional arguments start with `extra_`
3. **Use `binary_args` for runtime** - Clear distinction from compilation args
4. **Match cargo naming** - Where concepts overlap, use same names

## Implementation Note

For backward compatibility, we can support both:
- Old: `compile_args`, `runtime_args`
- New: `extra_args`, `extra_binary_args`

But documentation should only show the new consistent naming.