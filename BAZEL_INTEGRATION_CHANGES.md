# Bazel Integration Changes Summary

## Overview
Added comprehensive Bazel support to cargo-runner, including integration tests, doc tests, and build scripts.

## 1. Fixed Integration Test Detection (rust_test_suite)

### Problem
- Integration tests in `tests/` directory weren't being detected
- Command was generating `:test` instead of the actual `rust_test_suite` target
- The glob pattern `glob(["tests/**"])` wasn't being matched correctly

### Changes Made

#### Added glob pattern support in `target_finder.rs`:
```rust
// Handle tests/** pattern (matches all files under tests/ at any depth)
if pattern == "tests/**" {
    let result = file_path.starts_with("tests/");
    tracing::debug!("  Pattern 'tests/**' => {}", result);
    return result;
}
```

#### Enhanced error logging in `bazel_builder.rs`:
- Added detailed error messages when no integration test target is found
- Shows the BUILD file location and suggests adding rust_test_suite rule
- Lists all targets found but explains why none match

#### Added module path resolution in `bazel_runner.rs`:
- Integration tests now include proper module paths in test filters
- Example: `--test_arg tests::just_test::tests::see_if_it_works`

### Result
- ✅ Correctly detects `rust_test_suite` targets
- ✅ Generates proper command: `bazel test //server:integrated_tests_suite --test_arg --exact --test_arg tests::just_test::tests::see_if_it_works`

## 2. Added Doc Test Support (rust_doc_test)

### Problem
- Bazel can only run all doc tests together, not individual ones
- Need to detect `rust_doc_test` targets in BUILD files

### Changes Made

#### Added doc test detection in `bazel_builder.rs`:
```rust
fn build_doc_test_command(...) {
    // Find rust_doc_test target for this file
    if let Some(doc_test_target) = finder.find_doc_test_target(&abs_file_path, workspace_root)? {
        // Build command to run the doc test target
        let mut args = vec!["test".to_string(), doc_test_target.label];
        // ... rest of implementation
    }
}
```

### Result
- ✅ Detects `rust_doc_test` targets
- ✅ Adds warning that Bazel runs all doc tests together

## 3. Added Build Script Support (cargo_build_script)

### Problem
- `build.rs` files need to use `cargo_build_script` target
- Should use `bazel build` instead of `bazel run`

### Changes Made

#### Added build script detection in `determine_target`:
```rust
// Check if this is a build.rs file
if !is_test && runnable.file_path.file_name().map(|f| f == "build.rs").unwrap_or(false) {
    // Look for a cargo_build_script target
    for target in targets {
        if matches!(target.kind, BazelTargetKind::BuildScript) {
            return target.label;
        }
    }
}
```

#### Modified `build_binary_command` to use correct subcommand:
```rust
// For build scripts, override the subcommand to 'build'
if is_build_script {
    framework.subcommand = Some("build".to_string());
}
```

### Result
- ✅ Detects `cargo_build_script` targets
- ✅ Generates correct command: `bazel build //server:build_script`

## 4. Complete Bazel Redesign with Tree-sitter Starlark

### Problem
- Old regex-based approach was brittle and couldn't handle complex BUILD files
- Couldn't parse function calls like `all_crate_deps()` or `glob()`

### Architecture Created

#### New modules in `crates/core/src/bazel/`:
1. **`starlark_parser.rs`** - Uses tree-sitter-starlark for proper AST parsing
2. **`rule_extractor.rs`** - Walks AST and extracts rule calls with attributes
3. **`target_analyzer.rs`** - Analyzes rules to create BazelTarget instances
4. **`target_finder.rs`** - Main API for finding targets for source files
5. **`rules/`** directory - Individual handlers for each Bazel rule type

#### Key improvements:
- Proper Starlark parsing instead of regex matching
- Handles complex expressions like `deps = all_crate_deps() + ["//foo:bar"]`
- Extensible rule handler system
- Accurate glob pattern matching
- Better error messages with source locations

## 5. Enhanced Debugging and Error Messages

### Added comprehensive logging:
- Target detection process
- BUILD file discovery
- Rule extraction and analysis
- Pattern matching details
- Clear error messages when targets aren't found

### Example error output:
```
ERROR: No rust_test_suite target found for integration test: "server/tests/just_test.rs"
ERROR: Make sure your BUILD.bazel file contains a rust_test_suite rule
ERROR: Found BUILD file at: "/Users/uriah/Code/yoyo/server"
ERROR: This BUILD file should contain a rust_test_suite rule with glob pattern matching Some("just_test.rs")
```

## Files Modified

### Core Bazel support:
- `/crates/core/src/bazel/` - New directory with complete Bazel support
- `/crates/core/src/command/builder/bazel/bazel_builder.rs` - Enhanced with new target detection
- `/crates/core/src/runner_v2/bazel_runner.rs` - Added module path resolution

### Configuration:
- `/crates/core/src/bazel/rules/*.rs` - Rule handlers for each Bazel rule type
- `/crates/core/src/command/builder/mod.rs` - Exports for new Bazel modules

### Tests:
- `/crates/core/src/bazel/integration_test.rs` - Integration tests
- `/crates/core/src/bazel/integration_server_test.rs` - Server-specific tests

## Summary of Capabilities

1. **Integration Tests** - Detects and runs tests in `tests/` directories via `rust_test_suite`
2. **Doc Tests** - Finds and runs `rust_doc_test` targets
3. **Build Scripts** - Detects `build.rs` and runs via `cargo_build_script` 
4. **Proper Parsing** - Uses tree-sitter for accurate Starlark/BUILD file parsing
5. **Module Paths** - Includes full module paths in test filters
6. **Error Messages** - Clear, actionable error messages when targets aren't found
7. **Extensible** - Easy to add support for new Bazel rule types