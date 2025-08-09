# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the cargo-runner project, a Rust library that provides sophisticated scope-based detection of runnable code (tests, benchmarks, binaries, doc tests) and builds appropriate cargo commands. It uses tree-sitter for AST parsing and supports configuration overrides and caching.

## Development Commands

```bash
# Build the project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy

# Run a specific test
cargo test test_detect_test_function -- --exact

# Run examples
cargo run --example basic_usage

# Check compilation
cargo check
```

## Architecture

### Core Components

1. **Parser Module** (`src/parser/`)
   - `RustParser`: Tree-sitter based Rust code parser
   - `ScopeDetector`: Detects and builds scope hierarchy
   - `ModuleResolver`: Resolves full module paths for items

2. **Pattern Module** (`src/patterns/`)
   - `RunnableDetector`: Main entry point for detecting runnables
   - Pattern traits for different runnable types (Test, Benchmark, Binary, DocTest)
   - Handles line-based filtering and scoring

3. **Command Module** (`src/command/`)
   - `CommandBuilder`: Builds cargo commands from runnables
   - `CargoCommand`: Represents a cargo command with args and environment
   - Target detection from file paths

4. **Config Module** (`src/config/`)
   - JSON-based configuration with pattern matching
   - Override system for cargo args, exec args, and environment variables
   - Glob pattern support for flexible matching


5. **Runner Module** (`src/runner.rs`)
   - Main API entry point (`CargoRunner`)
   - Coordinates all components
   - Provides high-level methods for runnable detection and command building

### Key Data Structures

- `Scope`: Represents a code range with type (Function, Test, Module, etc.)
- `Runnable`: Detected runnable item with scope, kind, and module path
- `RunnableKind`: Enum for different runnable types
- `FunctionIdentity`: Used for configuration matching
- `Config`: Configuration with overrides

## Usage Example

```rust
use cargo_runner::{CargoRunner, Config};

let mut runner = CargoRunner::new()?;
let file_path = Path::new("src/lib.rs");

// Get runnable at specific line
if let Some(runnable) = runner.get_best_runnable_at_line(file_path, 42)? {
    if let Some(command) = runner.build_command_for_runnable(&runnable)? {
        println!("Command: {}", command.to_shell_command());
    }
}

// Get all runnables in file
let all_runnables = runner.detect_all_runnables(file_path)?;
```

## Configuration

Create a `cargo-runner.json` or `.cargo-runner.json` file:

```json
{
  "overrides": [
    {
      "matcher": {
        "module_path": "*::tests::*"
      },
      "cargo_args": ["--release"],
      "exec_args": ["--nocapture"]
    }
  ]
}
```

## Important Implementation Details

1. **Tree-sitter Integration**: Uses tree-sitter-rust for accurate AST parsing
2. **Attribute Detection**: Properly detects `#[test]`, `#[bench]`, etc. by checking previous siblings
3. **Doc Test Detection**: Scans for `///` comments with code blocks
4. **Module Path Resolution**: Combines file path and inline module hierarchy
5. **Smart Scoring**: Prioritizes specific runnables over module-level runners

## Testing

The codebase includes comprehensive tests for all components. Key test areas:
- Scope detection accuracy
- Pattern matching for different runnable types
- Module path resolution
- Command building with various targets
- Configuration override system
