# Cargo Test Complete Reference

## Command Structure
```bash
cargo test [OPTIONS] [TESTNAME] [-- [TEST_BINARY_ARGS]...]
```

## Cargo Test Options

### Core Options
| Option | Description |
|--------|-------------|
| `[TESTNAME]` | If specified, only run tests containing this string in their names |
| `--no-run` | Compile, but don't run tests |
| `--no-fail-fast` | Run all tests regardless of failure |
| `-q, --quiet` | Display one character per test instead of one line |
| `-v, --verbose...` | Use verbose output (-vv very verbose/build.rs output) |
| `--color <WHEN>` | Coloring: `auto`, `always`, `never` |
| `-h, --help` | Print help |

### Package Selection
| Option | Description |
|--------|-------------|
| `-p, --package [<SPEC>]` | Package to run tests for |
| `--workspace` | Test all packages in the workspace |
| `--exclude <SPEC>` | Exclude packages from the test |
| `--all` | Alias for --workspace (deprecated) |

### Target Selection
| Option | Description |
|--------|-------------|
| `--lib` | Test only this package's library |
| `--bins` | Test all binaries |
| `--bin [<NAME>]` | Test only the specified binary |
| `--examples` | Test all examples |
| `--example [<NAME>]` | Test only the specified example |
| `--tests` | Test all targets that have `test = true` set |
| `--test [<NAME>]` | Test only the specified test target |
| `--benches` | Test all targets that have `bench = true` set |
| `--bench [<NAME>]` | Test only the specified bench target |
| `--all-targets` | Test all targets (does not include doctests) |
| `--doc` | Test only this library's documentation |

### Feature Selection
| Option | Description |
|--------|-------------|
| `-F, --features <FEATURES>` | Space or comma separated list of features to activate |
| `--all-features` | Activate all available features |
| `--no-default-features` | Do not activate the `default` feature |

### Compilation Options
| Option | Description |
|--------|-------------|
| `-j, --jobs <N>` | Number of parallel jobs, defaults to # of CPUs |
| `-r, --release` | Build artifacts in release mode, with optimizations |
| `--profile <PROFILE-NAME>` | Build artifacts with the specified profile |
| `--target [<TRIPLE>]` | Build for the target triple |
| `--target-dir <DIRECTORY>` | Directory for all generated artifacts |
| `--unit-graph` | Output build graph in JSON (unstable) |
| `--timings[=<FMTS>]` | Timing output formats (unstable): html, json |

### Manifest Options
| Option | Description |
|--------|-------------|
| `--manifest-path <PATH>` | Path to Cargo.toml |
| `--lockfile-path <PATH>` | Path to Cargo.lock (unstable) |
| `--ignore-rust-version` | Ignore `rust-version` specification in packages |
| `--locked` | Assert that `Cargo.lock` will remain unchanged |
| `--offline` | Run without accessing the network |
| `--frozen` | Equivalent to specifying both --locked and --offline |

### Advanced Options
| Option | Description |
|--------|-------------|
| `--future-incompat-report` | Outputs a future incompatibility report at the end of the build |
| `--message-format <FMT>` | Error format: `human`, `short`, `json`, `json-diagnostic-short`, `json-diagnostic-rendered-ansi`, `json-render-diagnostics` |
| `--config <KEY=VALUE\|PATH>` | Override a configuration value |
| `-Z <FLAG>` | Unstable (nightly-only) flags to Cargo |

## Test Binary Arguments (After `--`)

### Test Execution Control
| Argument | Description |
|----------|-------------|
| `--test` | Run tests and not benchmarks |
| `--bench` | Run benchmarks instead of tests |
| `--list` | List all tests and benchmarks |
| `--ignored` | Run only ignored tests |
| `--include-ignored` | Run ignored and not ignored tests |
| `--exclude-should-panic` | Excludes tests marked as should_panic |
| `--force-run-in-process` | Forces tests to run in-process when panic=abort |

### Test Filtering
| Argument | Description |
|----------|-------------|
| `--exact` | Exactly match filters rather than by substring |
| `--skip FILTER` | Skip tests whose names contain FILTER (can be used multiple times) |

### Parallelization & Order
| Argument | Description |
|----------|-------------|
| `--test-threads n` | Number of threads used for running tests in parallel |
| `--shuffle` | Run tests in random order |
| `--shuffle-seed SEED` | Run tests in random order with specific seed |

### Output Control
| Argument | Description |
|----------|-------------|
| `--no-capture` | Don't capture stdout/stderr of each task |
| `--show-output` | Show captured stdout of successful tests |
| `--quiet` or `-q` | Display one character per test (alias to --format=terse) |
| `--color auto\|always\|never` | Configure coloring of output |
| `--format pretty\|terse\|json\|junit` | Configure formatting of output |

### Timing & Performance
| Argument | Description |
|----------|-------------|
| `--report-time` | Show execution time of each test |
| `--ensure-time` | Treat excess of test execution time limit as error |

### Deprecated/Legacy
| Argument | Description |
|----------|-------------|
| `--logfile PATH` | Write logs to the specified file (deprecated) |

### Unstable Options
| Argument | Description |
|----------|-------------|
| `-Z unstable-options` | Enable nightly-only flags |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_TEST_THREADS` | Number of parallel test threads (alternative to --test-threads) |
| `RUST_TEST_NOCAPTURE` | Set to non-"0" to disable output capture |
| `RUST_TEST_SHUFFLE` | Enable test shuffling |
| `RUST_TEST_SHUFFLE_SEED` | Seed for test shuffling |
| `RUST_TEST_TIME_UNIT` | Warn/critical times for unit tests (ms) |
| `RUST_TEST_TIME_INTEGRATION` | Warn/critical times for integration tests (ms) |
| `RUST_TEST_TIME_DOCTEST` | Warn/critical times for doctests (ms) |

## Test Attributes

| Attribute | Description |
|-----------|-------------|
| `#[test]` | Indicates a function is a test to be run |
| `#[bench]` | Indicates a function is a benchmark |
| `#[should_panic]` | Test passes only if code panics |
| `#[should_panic(expected = "msg")]` | Test passes if panic message contains "msg" |
| `#[ignore]` | Ignore test during normal runs |

---

# Command Builder Examples

## Basic Test Commands
```bash
# Run all tests
cargo test

# Run tests with name containing "user"
cargo test user

# Run tests in release mode
cargo test --release

# Run tests with verbose output
cargo test -v
```

## Package & Target Selection
```bash
# Test specific package
cargo test -p my_package

# Test all packages in workspace
cargo test --workspace

# Test only library
cargo test --lib

# Test specific binary
cargo test --bin my_binary

# Test all examples
cargo test --examples

# Test documentation
cargo test --doc
```

## Feature Control
```bash
# Test with specific features
cargo test --features "feature1,feature2"

# Test with all features
cargo test --all-features

# Test without default features
cargo test --no-default-features
```

## Test Binary Control (After `--`)
```bash
# Run only ignored tests
cargo test -- --ignored

# Run with single thread
cargo test -- --test-threads=1

# Show output from successful tests
cargo test -- --show-output

# Skip tests containing "slow"
cargo test -- --skip slow

# Run tests in random order
cargo test -- --shuffle

# List all tests without running
cargo test -- --list

# Run with JSON output
cargo test -- --format json

# Don't capture output
cargo test -- --no-capture
```

## Complex Commands
```bash
# Test specific package in release mode with features, single-threaded
cargo test -p my_package --release --features "async,db" -- --test-threads=1

# Run ignored tests with output shown
cargo test -- --ignored --show-output

# Test workspace excluding a package, with timing info
cargo test --workspace --exclude slow_tests -- --report-time

# Run benchmarks instead of tests
cargo test -- --bench

# Test with exact name matching and colored output
cargo test my_exact_test -- --exact --color=always

# Run tests in random order with specific seed
cargo test -- --shuffle --shuffle-seed 12345
```

## Performance & Debugging
```bash
# Show test execution times
cargo test -- --report-time

# Fail if tests exceed time limits
cargo test -- --ensure-time

# Run tests without fail-fast
cargo test --no-fail-fast

# Compile tests without running
cargo test --no-run

# Run with custom number of parallel jobs
cargo test -j 4
```

## Environment Variable Usage
```bash
# Run tests single-threaded via env var
RUST_TEST_THREADS=1 cargo test

# Run without output capture via env var
RUST_TEST_NOCAPTURE=1 cargo test

# Run in random order via env var
RUST_TEST_SHUFFLE=1 cargo test

# Set timing thresholds (warn at 500ms, critical at 2000ms)
RUST_TEST_TIME_UNIT=500,2000 cargo test -- --report-time
```