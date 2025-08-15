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

## Commands

### `cargo runner init`

Initialize cargo-runner in your project. This command sets up the necessary configuration for your build system.

```bash
# Initialize in current directory
cargo runner init

# Initialize with specific options
cargo runner init --rustc          # Enable rustc support for standalone files
cargo runner init --single-file    # Enable single-file script support
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
```
✅ Found 7 runnable(s):

1. Run doc test for 'User'
   📏 Scope: lines 2-13
   🚀 Final command: cargo test --doc --package project-a -- User

2. Run doc test for 'User::new'
   📏 Scope: lines 32-55
   🚀 Final command: cargo test --doc --package project-a -- User::new

3. Run doc test for 'User::echo'
   📏 Scope: lines 57-67
   🚀 Final command: cargo test --doc --package project-a -- User::echo

4. Run all tests in module 'tests'
   📏 Scope: lines 70-90
   🚀 Final command: cargo test --package project-a --lib -- tests

5. Run test 'test_it_works'
   📏 Scope: lines 74-78
   🚀 Final command: cargo test --package project-a --lib -- tests::test_it_works --exact

6. Run test 'test_user'
   📏 Scope: lines 80-89
   🚀 Final command: cargo test --package project-a --lib -- tests::test_user --exact
```

## How It Works

### Build System Detection

Cargo Runner automatically detects your build system in this order:

1. **Bazel** - Looks for `BUILD.bazel` or `BUILD` files
2. **Cargo** - Looks for `Cargo.toml`
3. **Rustc** - Fallback for standalone `.rs` files

### Generated Commands Examples

When you run `cargo runner run /path/to/file.rs:line`, it generates the appropriate command:

**Cargo:**
```bash
# Test function
cargo test test_function_name -- --exact

# Benchmark
cargo bench bench_name

# Binary
cargo run --bin binary_name
```

**Bazel:**
```bash
# Test function
bazel test //target:test_target --test_arg --nocapture --test_arg --exact --test_arg test_name

# Benchmark (with optimization)
bazel run -c opt //target:bench

# Binary
bazel run //target:binary
```

**Rustc (standalone files):**
```bash
# Compile and run tests
rustc --test file.rs -o /tmp/test && /tmp/test test_name

# Run single-file script
rustc file.rs -o /tmp/binary && /tmp/binary
```

## Real-World Examples

### Example 1: Running a specific test

```bash
# You have a test at line 45 in your library
$ cargo runner run /home/user/myproject/src/lib.rs:45

# Output:
Running: cargo test test_parse_config -- --exact
```

### Example 2: Analyzing a file

```bash
$ cargo runner analyze /home/user/myproject/src/parser.rs

# Output:
✅ Found 5 runnable(s):

1. Run test 'test_parse_string'
   📏 Scope: lines 23-35
   🚀 Final command: cargo test --package myproject --lib -- tests::test_parse_string --exact

2. Run test 'test_parse_number'
   📏 Scope: lines 45-58
   🚀 Final command: cargo test --package myproject --lib -- tests::test_parse_number --exact

3. Run test 'test_parse_array'
   📏 Scope: lines 67-79
   🚀 Final command: cargo test --package myproject --lib -- tests::test_parse_array --exact

4. Run all tests in module 'tests'
   📏 Scope: lines 89-125
   🚀 Final command: cargo test --package myproject --lib -- tests

5. Run doc test for 'Parser::new'
   📏 Scope: lines 120-135
   🚀 Final command: cargo test --doc --package myproject -- Parser::new
```

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

### Example 3: Working with Bazel

```bash
# Running a test in a Bazel project
$ cargo runner run /home/user/bazel-project/server/src/handler.rs:78

# Output:
Detected Bazel workspace at /home/user/bazel-project
Running: bazel test //server:handler_test --test_arg --nocapture --test_arg --exact --test_arg test_handle_request
```

For testing cargo-runner with complex Bazel setups, check out: https://github.com/codeitlikemiley/complex-bazel-setup

### Example 4: Standalone Rust files

```bash
# Initialize for standalone file support
$ cargo runner init --rustc

# Run tests in a standalone file
$ cargo runner run /tmp/my_script.rs:25

# Output:
Running: rustc --test /tmp/my_script.rs -o /tmp/cargo-runner-test && /tmp/cargo-runner-test test_calculation
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

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

## License

MIT or Apache-2.0, at your option.