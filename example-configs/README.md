# Cargo Runner Configuration Examples

This directory contains example configuration files demonstrating how to override settings at different levels.

## Configuration Structure

The new configuration system supports three main command types:
- `cargo`: For Cargo projects
- `rustc`: For standalone Rust files
- `single_file_script`: For Cargo script files (with shebang)

## Override Matching

Overrides are matched using the `FunctionIdentity` fields:
- `package`: Package name (for Cargo projects)
- `module_path`: Module path (e.g., "my_crate::tests::unit")
- `file_path`: Absolute file path
- `function_name`: Function name (supports wildcards like "bench_*")
- `file_type`: Type of file ("CargoProject", "Standalone", "SingleFileScript")

## Examples

### 01-cargo-project-overrides.json
Demonstrates overrides for Cargo projects at different levels:
- Package-level override (all tests in a package)
- File-level override (all tests in a specific file)
- Module-level override (all tests in a module)
- Function-level override (specific test function)
- Pattern-based override (all benchmarks)

### 02-standalone-rust-overrides.json
Shows how to configure standalone Rust files using `rustc`:
- Global rustc settings with test and binary frameworks
- File-level overrides
- Module-level overrides for test modules
- Function-level overrides for specific tests
- Performance optimization for specific files

### 03-single-file-script-overrides.json
Configuration for Cargo script files:
- Global settings for all script files
- File-specific overrides
- Test-specific configurations
- Environment variables for scripts

### 04-mixed-overrides-example.json
Complex example showing:
- Multiple command types in one config
- Pattern matching with wildcards
- Integration test configurations
- Benchmark configurations
- Feature selection

### 05-legacy-format-example.json
Shows backward compatibility with the old format where cargo settings were at the root level.

## Testing Overrides

To test these configurations:

1. Copy a config file to `.cargo-runner.json` in your project
2. Run `cargo runner analyze <file>` with the `-c` flag to see config details
3. Use `cargo runner run <file>` to execute with the overrides

Example:
```bash
# Test standalone file override
cp example-configs/02-standalone-rust-overrides.json ~/Code/nodes/.cargo-runner.json
cd ~/Code/nodes
cargo runner analyze test.rs:25 -c

# Test single file script override
cp example-configs/03-single-file-script-overrides.json ~/Code/nodes/.cargo-runner.json
cargo runner analyze sfc.rs:25 -c
```

## Override Priority

When multiple overrides match, the most specific one wins:
1. Function-level (most specific)
2. Module + File
3. Module-only
4. File-only
5. Package-only
6. File-type only (least specific)

## Environment Variables

Environment variables are merged (not replaced) unless using legacy `force_replace_env`.

## Features

For Cargo projects, features can be specified as:
```json
"features": {
  "all": "all"  // Use all features
}
```
or
```json
"features": {
  "selected": ["feature1", "feature2"]  // Specific features
}
```