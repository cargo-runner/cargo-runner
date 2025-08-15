# Cargo Runner

A sophisticated scope-based runnable detection tool for Rust that supports multiple build systems (Cargo, Bazel, Rustc) and provides intelligent command generation for tests, benchmarks, binaries, and doc tests.

## Features

- 🔍 **Smart Runnable Detection**: Automatically detects tests, benchmarks, binaries, and doc tests at any position in your code
- 🏗️ **Multi Build System Support**: Works with Cargo, Bazel, and standalone Rust files
- 🎯 **Precise Scope Detection**: Uses tree-sitter for accurate AST parsing
- ⚙️ **Flexible Configuration**: Override commands with pattern matching
- 🚀 **Fast & Reliable**: No compilation required for detection

## Installation

```bash
cargo install cargo-runner
```

Or clone and build from source:

```bash
git clone https://github.com/cargo-runner/cargo-runner
cd cargo-runner
cargo install --path crates/cli
```

## Usage

### Basic Commands

```bash
# Analyze a file to see all runnables
cargo runner analyze src/main.rs

# Run code at a specific line
cargo runner run src/main.rs:42

# Run without specific line (runs file-appropriate command)
cargo runner run src/lib.rs
```

### Build System Support

#### Cargo (Default)
Automatically detected when `Cargo.toml` is present.

```bash
# Runs: cargo test --exact test_name
cargo runner run src/lib.rs:10

# For benchmarks in benches/
# Runs: cargo bench benchmark_name
cargo runner run benches/my_bench.rs:25
```

#### Bazel
Automatically detected when `BUILD.bazel` or `BUILD` files are present.

```bash
# Runs: bazel test //target:test_target --test_arg --nocapture --test_arg --exact --test_arg test_name
cargo runner run src/lib.rs:10

# For benchmarks (runs with optimization)
# Runs: bazel run -c opt //target:bench
cargo runner run benches/fibonacci.rs
```

**Bazel-specific features:**
- Automatic target detection from BUILD files
- Handles `rust_test` targets that reference binaries via `crate` attribute
- Correct module path resolution for `src/bin/` files
- Automatic workspace root detection for proper execution

#### Rustc (Standalone Files)
For standalone Rust files without a build system.

```bash
# Compiles and runs tests
cargo runner run standalone_test.rs:15

# Single-file scripts with shebang
cargo runner run script.rs
```

### Configuration

Create a `.cargo-runner.json` file in your project root:

```json
{
  "cargo": {
    "test_framework": {
      "command": "cargo",
      "subcommand": "nextest run"
    }
  },
  "bazel": {
    "test_framework": {
      "test_args": ["--nocapture", "--exact", "{test_filter}"]
    },
    "binary_framework": {
      "args": ["-c", "opt"]
    }
  }
}
```

### Override Commands

Use pattern matching to customize commands for specific cases:

```json
{
  "overrides": [
    {
      "match": {
        "path": "tests/integration/*",
        "type": "test"
      },
      "cargo": {
        "env": { "TEST_ENV": "integration" },
        "extra_test_binary_args": ["--test-threads=1"]
      }
    }
  ]
}
```

## Examples

### Running Tests

```bash
# Run a specific test function
cargo runner run src/lib.rs:42

# Run all tests in a module
cargo runner run src/utils/mod.rs:10

# Run with custom configuration
echo '{"cargo": {"test_framework": {"subcommand": "nextest run"}}}' > .cargo-runner.json
cargo runner run src/lib.rs:42
```

### Working with Bazel

```bash
# Detects and uses Bazel targets automatically
cargo runner run server/src/main.rs:100

# Runs benchmarks with optimization
cargo runner run server/benches/perf.rs

# Module tests with proper filtering
cargo runner run server/src/bin/proxy.rs:5  # Runs: bazel test //server:proxy_test --test_arg tests
```

### Binary Detection

```bash
# In src/main.rs or src/bin/
cargo runner run src/bin/server.rs  # Runs: cargo run --bin server

# With Bazel
cargo runner run src/bin/proxy.rs   # Runs: bazel run //server:proxy
```

## Advanced Features

### Pattern Detection Priority

1. **Exact match**: Specific test/benchmark functions
2. **Module tests**: `mod tests` blocks
3. **Binary detection**: Files with `main()` function
4. **Doc tests**: Code blocks in doc comments
5. **File-level fallback**: Appropriate command for file type

### Module Path Resolution

The tool correctly resolves module paths for:
- Standard library structure (`src/lib.rs`, `src/main.rs`)
- Binary crates in `src/bin/`
- Integration tests in `tests/`
- Benchmarks in `benches/`
- Workspace members

### Build System Detection

Automatic detection order:
1. Bazel (presence of `BUILD.bazel` or `BUILD`)
2. Cargo (presence of `Cargo.toml`)
3. Rustc (fallback for standalone files)

## Configuration Reference

See [config-analyze.md](config-analyze.md) for complete configuration options.

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

## License

MIT or Apache-2.0, at your option.