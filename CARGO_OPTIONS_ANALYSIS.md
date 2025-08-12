# Comprehensive Cargo Options Analysis

## Common Option Categories Across Commands

### 1. Package Selection
Available in: `test`, `run`, `build`, `bench`
```
-p, --package [<SPEC>]  Package to build/test/run
    --workspace         Build all packages in the workspace  
    --exclude <SPEC>    Exclude packages from the operation
    --all               Alias for --workspace (deprecated)
```

### 2. Target Selection
Available in: `test`, `build`, `bench`
```
    --lib               Target only this package's library
    --bins              Target all binaries
    --bin [<NAME>]      Target only the specified binary
    --examples          Target all examples
    --example [<NAME>]  Target only the specified example
    --tests             Target all test targets (test = true)
    --test [<NAME>]     Target only the specified test target
    --benches           Target all benchmark targets (bench = true)
    --bench [<NAME>]    Target only the specified bench target
    --all-targets       Target all targets
    --doc               Test only this library's documentation (test only)
```

### 3. Feature Selection
Available in: ALL commands
```
-F, --features <FEATURES>  Space or comma separated list of features
    --all-features         Activate all available features
    --no-default-features  Do not activate the `default` feature
```

### 4. Compilation Options
```
-j, --jobs <N>                Number of parallel jobs
-r, --release                 Build in release mode
    --profile <PROFILE-NAME>  Build with specified profile
    --target [<TRIPLE>]       Build for the target triple
    --target-dir <DIRECTORY>  Directory for all generated artifacts
    --unit-graph              Output build graph in JSON (unstable)
    --timings[=<FMTS>]        Timing output formats (unstable)
```

### 5. Manifest Options
Available in: ALL commands
```
    --manifest-path <PATH>  Path to Cargo.toml
    --lockfile-path <PATH>  Path to Cargo.lock (unstable)
    --ignore-rust-version   Ignore `rust-version` specification
    --locked                Assert Cargo.lock remains unchanged
    --offline               Run without accessing the network
    --frozen                Equivalent to --locked and --offline
```

### 6. Output Options
```
    --message-format <FMT>     Error format options
-v, --verbose...               Use verbose output
-q, --quiet                    Quiet output
    --color <WHEN>             Coloring: auto, always, never
```

## Command-Specific Options

### cargo test
```
Arguments:
  [TESTNAME]  Only run tests containing this string
  [-- [ARGS]...]  Arguments for the test binary

Unique Options:
    --no-run           Compile, but don't run tests
    --no-fail-fast     Run all tests regardless of failure
    --doc              Test only documentation

Test Binary Options (after --):
    --test-threads n   Number of test threads
    --nocapture        Don't capture stdout/stderr
    --exact            Exactly match test names
    --ignored          Run only ignored tests
    --include-ignored  Run both ignored and normal tests
    --show-output      Show captured stdout of successful tests
    --format <FMT>     Output format: pretty, terse, json, junit
```

### cargo run
```
Arguments:
  [ARGS]...  Arguments for the binary

Unique Options:
    --bin [<NAME>]      Name of the bin target to run
    --example [<NAME>]  Name of the example target to run
```

### cargo build
```
Unique Options:
    --build-plan        Output the build plan in JSON (unstable)
    --future-incompat-report  Output future incompatibility report
```

### cargo bench
```
Arguments:
  [BENCHNAME]  Only run benchmarks containing this string
  [-- [ARGS]...]  Arguments for the benchmark binary

Unique Options:
    --no-run           Compile, but don't run benchmarks
    --no-fail-fast     Run all benchmarks regardless of failure
```

### rustdoc
```
Core Options:
    --crate-name NAME     Specify crate name
    --crate-type TYPE     Crate type (bin, lib, etc.)
    -L, --library-path    Add to crate search path
    --extern NAME[=PATH]  Pass an --extern to rustc
    --test                Run code examples as tests
    --test-args ARGS      Arguments for test runner

Documentation Options:
    --document-private-items     Document private items
    --document-hidden-items      Document hidden items
    --extern-html-root-url       Base URL for dependencies
    --markdown-css FILES         CSS for Markdown
    --html-in-header FILES       Include in <head>
    --html-before-content FILES  Include before content
```

## Validation Rules

### Mutually Exclusive Options

1. **Features**:
   - `--all-features` ↔ `--no-default-features`
   - `--all-features` ↔ `--features` (when --all-features is set, specific features are ignored)

2. **Targets** (for single target commands like `run`):
   - `--lib` ↔ `--bin`
   - `--lib` ↔ `--example`
   - Multiple `--bin` values require specific selection

3. **Package Selection**:
   - `--package` ↔ `--workspace`
   - `--workspace` makes `--exclude` meaningful

4. **Test Specific**:
   - `--doc` ↔ other target selections (--lib, --bin, etc.)
   - `--ignored` ↔ `--include-ignored`

5. **Manifest**:
   - `--frozen` implies `--locked` and `--offline`

### Conditional Requirements

1. **Target Requirements**:
   - `--bin NAME` requires NAME to exist
   - `--example NAME` requires NAME to exist
   - `--test NAME` requires test target to exist

2. **Profile Requirements**:
   - `--profile NAME` requires profile to be defined

3. **Build System**:
   - Some options only available with specific Cargo versions
   - Unstable options require nightly and `-Z unstable-options`

## Option Precedence

1. **Command Line** > **Config File** > **Defaults**
2. **Specific** > **General** (e.g., `--bin name` > `--bins`)
3. **Explicit** > **Implicit** (e.g., `--release` > default dev profile)

## Configuration File Mapping

Many command-line options can be set in `.cargo/config.toml`:
```toml
[build]
target = "x86_64-unknown-linux-gnu"
target-dir = "target"
jobs = 8

[term]
verbose = true
color = "auto"

[profile.release]
opt-level = 3
debug = false
```

## Implementation Considerations

1. **Parse Order**: Parse general options before specific ones
2. **Validation Timing**: Validate after all options are collected
3. **Error Messages**: Provide specific guidance on conflicts
4. **Help Integration**: Generate help from option definitions
5. **Compatibility**: Maintain backward compatibility