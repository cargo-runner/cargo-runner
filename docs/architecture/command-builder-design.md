# Command Builder Architecture

## Overview

The command builder system provides a clean, type-safe API for constructing cargo commands for different target types (tests, doc tests, binaries, benchmarks, etc.) while properly handling configuration merging and test framework integration.

## Design Principles

1. **Separation of Concerns**: Each target type has its own builder with specific logic
2. **Encapsulation**: Config merging and builder selection are hidden from users
3. **Clean API**: Simple, chainable methods that are easy to discover and use
4. **Type Safety**: Compile-time guarantees about command construction

## Architecture

```
┌─────────────────┐
│ CommandBuilder  │  <- Public API
│   (builder_v2)  │
└────────┬────────┘
         │
         ├─── Resolves Config (ConfigResolver)
         │
         └─── Selects Builder Based on RunnableKind
                    │
    ┌───────────────┼───────────────┬─────────────────┬──────────────────┐
    │               │               │                 │                  │
┌───▼────┐  ┌──────▼──────┐  ┌────▼─────┐  ┌───────▼──────┐  ┌─────────▼────┐
│DocTest  │  │    Test     │  │ Binary   │  │  Benchmark   │  │ ModuleTest   │
│Builder  │  │   Builder   │  │ Builder  │  │   Builder    │  │   Builder    │
└─────────┘  └─────────────┘  └──────────┘  └──────────────┘  └──────────────┘
              ▲                                                  ▲
              │                                                  │
              └──────── test_framework config applies here ─────┘
```

## Public API

```rust
// Simple and discoverable
let command = CommandBuilder::for_runnable(&runnable)
    .with_package("my-package")
    .with_project_root("/path/to/project")
    .build()?;
```

## Key Features

### 1. Target-Specific Logic

Each builder handles its target's specific requirements:
- **DocTestBuilder**: Never adds `--lib`, handles doc test paths
- **TestBuilder**: Applies test framework config, handles test filters
- **BinaryBuilder**: Handles `--bin` targets and run arguments
- **BenchmarkBuilder**: Handles benchmark-specific flags
- **ModuleTestBuilder**: Like TestBuilder but for module-level tests

### 2. Test Framework Integration

Test framework configuration only applies to:
- `TestBuilder`
- `ModuleTestBuilder`

Doc tests, binaries, and benchmarks are unaffected by test framework settings.

### 3. Configuration Priority

1. Function-specific overrides (highest priority)
2. Test framework configuration (for tests only)
3. Global configuration
4. Default values (lowest priority)

### 4. Clean Error Handling

All builders return `Result<CargoCommand>` for proper error propagation.

## Example Usage

### Running Tests with Nextest

```rust
// Config with nextest
let config = Config {
    test_frameworks: Some(TestFramework {
        subcommand: Some("nextest run".to_string()),
        channel: Some("nightly".to_string()),
        args: Some(vec!["--retries=3".to_string()]),
    }),
    ..Default::default()
};

// This applies nextest
let test_cmd = CommandBuilder::for_runnable(&test_runnable)
    .with_config(config.clone())
    .build()?;
// Result: cargo +nightly nextest run --retries=3 ...

// This does NOT apply nextest (doc tests have their own rules)
let doc_cmd = CommandBuilder::for_runnable(&doc_test_runnable)
    .with_config(config)
    .build()?;
// Result: cargo test --doc ...
```

### Override Matching

```rust
// Override for specific test
let override = Override {
    identity: FunctionIdentity {
        function_name: Some("test_important".to_string()),
        ..Default::default()
    },
    extra_test_binary_args: Some(vec!["--nocapture".to_string()]),
    ..Default::default()
};
```

## Benefits

1. **Maintainability**: Each builder is self-contained and easy to modify
2. **Testability**: Builders can be unit tested independently
3. **Extensibility**: New target types can be added without affecting existing ones
4. **User-Friendly**: Clean API hides complexity while providing full control