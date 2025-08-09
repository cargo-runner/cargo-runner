# Simplified RustC Configuration Design

## Problem
The current rustc configuration with `test_framework` and `binary_framework` is too complex and error-prone. Users shouldn't need to specify command structure.

## Solution: Simplified Config

### User-Facing Config Structure

```json
{
  "rustc": {
    "extra_args": [],           // Applied to ALL rustc commands
    "extra_test_args": [],      // Applied only when building tests (merged with extra_args)
    "extra_test_binary_args": [], // Applied when running test binaries
    "extra_env": {}             // Environment variables
  }
}
```

### Internal Default Frameworks (Hidden from Users)

The system internally maintains these defaults:

```rust
// For tests
default_test_framework = {
    build: {
        command: "rustc",
        args: ["--test", "{source_file}", "-o", "{output_name}"],
        // User's extra_args and extra_test_args go here
    },
    exec: {
        command: "./{output_name}",
        args: ["{test_name}"],
        // User's extra_test_binary_args go here
    }
}

// For binaries
default_binary_framework = {
    build: {
        command: "rustc",
        args: ["--crate-type", "bin", "--crate-name", "{crate_name}", "{source_file}", "-o", "{output_name}"],
        // User's extra_args go here
    },
    exec: {
        command: "./{output_name}",
        args: []
    }
}
```

## Argument Merging Strategy

### Build Phase (Compilation)

1. Start with base args: `["--test", "file.rs", "-o", "output"]`
2. Merge user's `extra_args` (deduplicated)
3. For tests, also merge `extra_test_args` (deduplicated)
4. Insert merged args BEFORE `-o output` to maintain correct order

Example:
- Base: `["--test", "test.rs", "-o", "test_output"]`
- User extra_args: `["--edition", "2021", "-O"]`
- User extra_test_args: `["--cfg", "test", "--edition", "2021"]`
- After dedup: `["--edition", "2021", "-O", "--cfg", "test"]`
- Final: `["--test", "test.rs", "--edition", "2021", "-O", "--cfg", "test", "-o", "test_output"]`

### Exec Phase (Running)

1. Start with base: `["./test_output", "test_name"]`
2. Add `--` separator
3. Append all `extra_test_binary_args`

Example:
- Base: `["./test_output", "test_alpha"]`
- User extra_test_binary_args: `["--nocapture", "--exact"]`
- Final: `["./test_output", "test_alpha", "--", "--nocapture", "--exact"]`

## Deduplication Rules

1. **Exact match**: Remove duplicates (keep last)
   - `["-O", "--verbose", "-O"]` → `["--verbose", "-O"]`

2. **Flag with value**: Deduplicate by flag name
   - `["--edition", "2018", "--edition", "2021"]` → `["--edition", "2021"]`
   - `["-C", "opt-level=2", "-C", "opt-level=3"]` → `["-C", "opt-level=2", "-C", "opt-level=3"]` (keep both as they're different)

3. **Special handling for -C flags**: Group by sub-option
   - `["-C", "opt-level=2", "-C", "target-cpu=native", "-C", "opt-level=3"]`
   - → `["-C", "target-cpu=native", "-C", "opt-level=3"]`

## Implementation Changes

### 1. Update RustcConfig struct

```rust
pub struct RustcConfig {
    pub extra_args: Option<Vec<String>>,
    pub extra_test_args: Option<Vec<String>>,      // NEW - only for test compilation
    pub extra_test_binary_args: Option<Vec<String>>, // NEW - for test execution
    pub extra_env: Option<HashMap<String, String>>,
    
    // DEPRECATED but kept for backward compatibility
    pub test_framework: Option<RustcFramework>,
    pub binary_framework: Option<RustcFramework>,
}
```

### Config Processing Logic

1. If `test_framework` or `binary_framework` are specified → use them (backward compatibility)
2. Otherwise → use simplified fields to build frameworks internally

### 2. Update RustcCommandBuilder

```rust
impl RustcCommandBuilder {
    fn get_test_framework(&self, config: &Config) -> RustcFramework {
        // If user specified framework (old style), use it
        if let Some(framework) = config.rustc.as_ref().and_then(|r| r.test_framework.clone()) {
            return framework;
        }
        
        // Otherwise, use simplified approach
        self.build_simplified_test_framework(config)
    }
    
    fn build_simplified_test_framework(&self, config: &Config) -> RustcFramework {
        RustcFramework {
            build: Some(RustcPhaseConfig {
                command: Some("rustc".to_string()),
                args: Some(vec!["--test", "{source_file}", "-o", "{output_name}"]),
                extra_args: self.merge_build_args(config),
                extra_test_binary_args: None,
            }),
            exec: Some(RustcPhaseConfig {
                command: Some("./{output_name}".to_string()),
                args: Some(vec!["{test_name}"]),
                extra_args: None,
                extra_test_binary_args: config.rustc.as_ref()
                    .and_then(|r| r.extra_test_binary_args.clone()),
            }),
        }
    }
    
    fn merge_build_args(&self, config: &Config) -> Option<Vec<String>> {
        let mut args = Vec::new();
        
        // Add extra_args
        if let Some(extra) = config.rustc.as_ref().and_then(|r| r.extra_args.as_ref()) {
            args.extend(extra.clone());
        }
        
        // Add extra_test_args for test builds
        if let Some(test_extra) = config.rustc.as_ref().and_then(|r| r.extra_test_args.as_ref()) {
            args.extend(test_extra.clone());
        }
        
        // Deduplicate
        let deduped = self.deduplicate_args(args);
        
        if deduped.is_empty() {
            None
        } else {
            Some(deduped)
        }
    }
}
```

## Benefits

1. **Simple for users**: Just specify extra args, not command structure
2. **Safe**: Can't break command order
3. **Smart merging**: Deduplication prevents conflicts
4. **Backward compatible**: Old configs still work
5. **Override friendly**: Easy to override at any level

## Migration Path

1. Support both old and new formats
2. Internally convert new format to old format
3. Documentation shows only new format
4. Eventually deprecate complex format