# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **windrunner** (cargo-runner), a Rust tool that provides sophisticated scope-based detection of runnable code (tests, benchmarks, binaries, doc tests) and builds appropriate cargo commands. It uses tree-sitter for AST parsing and supports multiple build systems (Cargo, Bazel), configuration overrides, and caching.

## Development Commands

```bash
# Build the entire workspace
cargo build

# Run all tests  
cargo test

# Run tests for a specific crate
cargo test -p cargo-runner-core
cargo test -p cargo-runner

# Run a specific test
cargo test test_detect_test_function -- --exact

# Format code
cargo fmt

# Run linter
cargo clippy

# Check compilation
cargo check

# Run examples
cargo run --example showcase
cargo run --example demo

# Install the CLI tool
cargo install --path crates/cli

# Use the CLI
cargo runner analyze src/main.rs:10
cargo runner run src/main.rs:42
```

## Architecture

### Workspace Structure

- **`crates/core/`**: Core library with all detection and command building logic
- **`crates/cli/`**: CLI binary that uses the core library
- **Root package**: Integration tests and examples

### Core Components (`crates/core/src/`)

1. **Parser Module** (`parser/`)
   - `RustParser`: Tree-sitter based Rust code parser
   - `ScopeDetector`: Detects and builds scope hierarchy (functions, modules, impl blocks)
   - `ModuleResolver`: Resolves full module paths for items

2. **Pattern Module** (`patterns/`)
   - `RunnableDetector`: Main entry point for detecting runnables
   - Specialized detectors: `TestPattern`, `BenchmarkPattern`, `BinaryPattern`, `DocTestPattern`, `ModTestPattern`
   - Handles line-based filtering and scoring

3. **Command Module** (`command/`)
   - `CommandBuilder`: Builds commands from runnables
   - Build system specific builders: `cargo/`, `bazel/`, `rustc/`
   - `CargoCommand`: Represents executable command with args and environment
   - Target detection from file paths

4. **Config Module** (`config/`)
   - JSON-based configuration with pattern matching
   - Override system for cargo args, exec args, and environment variables
   - Supports Cargo, Bazel, and Rustc configurations

5. **Runner V2 Module** (`runner_v2/`)
   - `UnifiedRunner`: Main API entry point that manages all build systems
   - `CargoRunner`, `BazelRunner`: Build system specific runners
   - Backward compatible with legacy `CargoRunner` API

6. **Build System Module** (`build_system/`)
   - Auto-detects build system (Cargo via Cargo.toml, Bazel via BUILD files)
   - Extensible for new build systems

### Key Data Structures

- `Scope`: Code range with type (Function, Test, Module, etc.) and position info
- `Runnable`: Detected runnable item with scope, kind, module path, and label
- `RunnableKind`: Enum for Test, Benchmark, Binary, DocTest, ModuleTests, Standalone, SingleFileScript
- `CargoCommand`: Executable command with program, args, env, and working directory
- `Config`: Configuration with overrides for different build systems

## CLI Commands

The CLI supports both direct invocation (`cargo-runner`) and as cargo subcommand (`cargo runner`):

- **`analyze <filepath>`**: List all runnables in a file
- **`run <filepath>`**: Run code at specific location (supports `file.rs:line` syntax)
- **`init`**: Create configuration file with options for rustc, single-file scripts
- **`override <filepath>`**: Create override configuration for specific locations
- **`unset`**: Remove configuration files

## Configuration

Configuration files (`.cargo-runner.json` or `cargo-runner.json`) support:

```json
{
  "cargo": {
    "overrides": [{
      "matcher": {
        "module_path": "*::tests::*",
        "function_name": "test_*"
      },
      "cargo_args": ["--release"],
      "exec_args": ["--nocapture"],
      "env": {
        "RUST_LOG": "debug"
      }
    }]
  },
  "bazel": {
    "overrides": [{
      "matcher": {
        "target": "//src:*_test"
      },
      "bazel_args": ["--test_output=all"]
    }]
  }
}
```

## Testing Patterns

Tests are organized by module with `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_functionality() {
        // test code
    }
}
```

## Build System Support

1. **Cargo**: Default for Rust projects with Cargo.toml
2. **Bazel**: Detected by BUILD/BUILD.bazel files
3. **Rustc**: Fallback for standalone files without build system

The `UnifiedRunner` automatically detects and uses the appropriate build system.

## Key Implementation Details

- **Tree-sitter Integration**: Provides accurate AST parsing without compilation
- **Attribute Detection**: Checks previous siblings for `#[test]`, `#[bench]` attributes
- **Doc Test Detection**: Scans `///` comments for code blocks with special handling
- **Module Path Resolution**: Combines file path and inline module hierarchy
- **Smart Scoring**: Prioritizes specific runnables over module-level runners
- **Extended Scopes**: Tracks doc comments for better doc test detection
- **Configuration Matching**: Uses glob patterns and specific matchers for flexibility