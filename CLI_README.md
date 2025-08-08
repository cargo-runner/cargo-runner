# Cargo Runner CLI

A command-line tool to showcase the cargo-runner library's capabilities. It scans Rust files and displays all available runnables with their scope ranges and corresponding cargo commands.

## Installation

```bash
cargo install --path .
```

Or run directly from the project:

```bash
cargo run --bin cargo-runner-cli -- <file-path> [line-number]
```

## Usage

### Show all runnables in a file:
```bash
cargo-runner-cli src/lib.rs
```

### Show runnable at a specific line:
```bash
cargo-runner-cli src/lib.rs 42
```

### Show test runnables:
```bash
cargo-runner-cli src/tests/mod.rs
```

## Example Output

```
🔍 Scanning: examples/showcase.rs
================================================================================
✅ Found 11 runnable(s):

1. Run test 'test_add'
   📏 Scope: lines 53-57
   🚀 Command: cargo test --package cargo-runner -- cargo-runner::test_add::test_add --exact
   📦 Type: Test function 'test_add'
   📁 Module path: cargo-runner::test_add

2. Run benchmark 'bench_add'
   📏 Scope: lines 101-105
   🚀 Command: cargo bench --package cargo-runner
   📦 Type: Benchmark 'bench_add'
   📁 Module path: cargo-runner::bench_add

3. Run binary 'showcase'
   📏 Scope: lines 114-122
   🚀 Command: cargo run --package cargo-runner
   📦 Type: Binary 'showcase'
   📁 Module path: cargo-runner::main
================================================================================
```

## Detectable Runnables

The CLI can detect:
- ✅ Test functions (`#[test]`)
- ✅ Async tests (`#[tokio::test]`)
- ✅ Benchmarks (`#[bench]`)
- ✅ Binary/main functions
- ✅ Doc tests in `///` comments
- ✅ Test modules

## Features

- 📏 Shows exact line ranges for each runnable
- 🚀 Displays the exact cargo command to run
- 📦 Shows runnable type and details
- 📁 Displays full module path
- 📍 Can find the best runnable at a specific line
- ⚡ Shows if tests are async

## Use Cases

1. **IDE Integration**: Find what test/benchmark to run at cursor position
2. **CI/CD**: Enumerate all tests in a file for selective execution
3. **Documentation**: List all runnable examples in a codebase
4. **Navigation**: Quickly find test locations in large files
5. **Learning**: Understand how cargo commands are constructed