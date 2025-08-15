# Cargo Runner

A sophisticated scope-based runnable detection tool for Rust that supports multiple build systems (Cargo, Bazel, Rustc) and provides intelligent command generation for tests, benchmarks, binaries, and doc tests.

## Features

- 🔍 **Smart Runnable Detection**: Automatically detects tests, benchmarks, binaries, and doc tests at any position in your code
- 🏗️ **Multi Build System Support**: Works with Cargo, Bazel, and standalone Rust files
- 🎯 **Precise Scope Detection**: Uses tree-sitter for accurate AST parsing
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

## Usage:

### Set up your project

```sh
cargo new my-project
cd my-project
cargo runner init
```

This would work for simple or complex rust projects , all rust projects during init are added to `linkedProjects` in `.cargo-runner.json` file.


### `cargo runner init`

Initialize cargo-runner in your project. This command sets up the necessary configuration for your build system.

```bash
# Initialize in current directory
cargo runner init # generate config for cargo / bazel 

# Initialize with specific options
cargo runner init --rustc          # generate config so you can override rustc to your needs
cargo runner init --single-file    # generate config so you can override single-file script to your needs
```

### `cargo runner run`

Run the code at a specific location in a file. The tool will automatically detect what to run based on the cursor position.

```bash
# Run test at line 42
cargo runner run /path/to/src/lib.rs:42

# Run benchmark at line 156  
cargo runner run /path/to/benches/benchmark.rs:156

# Run binary (detects main function)
cargo runner run /path/to/src/main.rs

# Run file without specific line (runs appropriate command for file type)
cargo runner run /path/to/src/bin/server.rs
```

**What it detects:**
- Test functions (`#[test]`)
- Benchmark functions (`#[bench]`)
- Binary files with `main()` function
- Module tests (`mod tests`)
- Doc tests in comments
- Integration tests in `tests/` directory

### `cargo runner analyze`

Analyze a file to see all runnable items it contains. This is useful for understanding what can be run in a file.

```bash
# Analyze entire file
cargo runner analyze /path/to/src/lib.rs

# Analyze at specific line (shows what would run at that position)
cargo runner analyze /path/to/src/lib.rs:42
```

**Example output:**

<details>
<summary>📋 Detailed analyze output example</summary>

```
🔍 Analyzing: project-a/src/lib.rs
================================================================================

📄 File-level command:
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--lib"]
   🚀 Final command: cargo test --package project-a --lib
   📦 Type: Library (lib.rs)
   📏 Scope: lines 1-90

✅ Found 7 runnable(s):

1. Run doc test for 'User'
   📏 Scope: lines 2-13
   📍 Module path: project-a
   🧪 Contains doc tests
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--doc"]
      • extraTestBinaryArgs: ["User"]
   🚀 Final command: cargo test --doc --package project-a -- User
   📦 Type: Doc test for 'User'
   📁 Module path: project-a

2. Run doc test for 'impl User'
   📏 Scope: lines 15-68
   📍 Module path: project-a
   🧪 Contains doc tests
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--doc"]
      • extraTestBinaryArgs: ["User"]
   🚀 Final command: cargo test --doc --package project-a -- User
   📦 Type: Doc test for 'impl User'
   📁 Module path: project-a

3. Run doc test for 'User::new'
   📏 Scope: lines 32-55
   📍 Module path: project-a
   🧪 Contains doc tests
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--doc"]
      • extraTestBinaryArgs: ["User::new"]
   🚀 Final command: cargo test --doc --package project-a -- User::new
   📦 Type: Doc test for 'User'::new
   📁 Module path: project-a

4. Run doc test for 'User::echo'
   📏 Scope: lines 57-67
   📍 Module path: project-a
   🧪 Contains doc tests
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--doc"]
      • extraTestBinaryArgs: ["User::echo"]
   🚀 Final command: cargo test --doc --package project-a -- User::echo
   📦 Type: Doc test for 'User'::echo
   📁 Module path: project-a

5. Run all tests in module 'tests'
   📏 Scope: lines 70-90
   🏷️  Attributes: 1 lines
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--lib"]
      • extraTestBinaryArgs: ["tests"]
   🚀 Final command: cargo test --package project-a --lib -- tests
   📦 Type: Test module 'tests'

6. Run test 'test_it_works'
   📏 Scope: lines 74-78
   📍 Module path: tests
   🏷️  Attributes: 1 lines
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--lib"]
      • extraTestBinaryArgs: ["tests::test_it_works", "--exact"]
   🚀 Final command: cargo test --package project-a --lib -- tests::test_it_works --exact
   📦 Type: Test function 'test_it_works'
   📁 Module path: tests

7. Run test 'test_user'
   📏 Scope: lines 80-89
   📍 Module path: tests
   🏷️  Attributes: 1 lines
   🔧 Command breakdown:
      • command: cargo
      • subcommand: test
      • package: project-a
      • extraArgs: ["--lib"]
      • extraTestBinaryArgs: ["tests::test_user", "--exact"]
   🚀 Final command: cargo test --package project-a --lib -- tests::test_user --exact
   📦 Type: Test function 'test_user'
   📁 Module path: tests

🎯 Command to run:
   cargo test --package project-a --lib -- tests::test_user --exact

================================================================================
```

</details>


## How It Works

### Build System Detection

Cargo Runner automatically detects your build system in this order:

1. **Bazel** - Looks for `BUILD.bazel` or `BUILD` files
2. **Cargo** - Looks for `Cargo.toml`
3. **Rustc** - Fallback for standalone `.rs` files

## Usage with Bazel

For testing cargo-runner with complex Bazel setups, check out: https://github.com/codeitlikemiley/complex-bazel-setup


## Advanced Features

### Pattern Detection Priority

1. **Exact match**: Specific test/benchmark functions
2. **Module tests**: `mod tests` blocks
3. **Binary detection**: Files with `main()` function
4. **Doc tests**: Code blocks in doc comments
5. **File-level fallback**: Appropriate command for file type

### Module Path Resolution

The tool correctly resolves module paths for individual tests for binaries, integration tests, and benchmarks.
e.g. `tests::test_user`

### Build System Detection

Automatic detection order:
1. Bazel (presence of `BUILD.bazel` or `BUILD`)
2. Cargo (presence of `Cargo.toml`)
3. Rustc (fallback for standalone files)

Bazel project have both `Cargo.toml` , and `MODULE.bazel` and `BUILD.bazel` files.
We don't support `WORKSPACE` file for now it would be deprecated this year 2025.

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

## License

MIT