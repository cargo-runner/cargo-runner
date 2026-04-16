# Architecture

```
cargo-runner/
└── crates/
    ├── core/          ← cargo-runner-core  (command engine, config, runners)
    └── cli/           ← cargo-runner-cli   (CLI front-end, fully modularized)
```

### Key subsystems

| Subsystem | Path | Description |
|-----------|------|-------------|
| **ResolverChain** | `crates/core/src/command/resolver/` | Composable resolver pipeline. Uses `.push_resolver()` to build the resolution chain. |
| **CommandBuilder** | `crates/core/src/command/builder/mod.rs` | Unified command constructor. Now requires explicit `FileType` propagation from runners, avoiding ambiguous internal detection files logic. |
| **CommandTemplate** | `crates/core/src/command/template/` | DSL-based template engine (`{target}`, `{test_filter}`, etc.) |
| **BazelCommandBuilder** | `crates/core/src/command/builder/bazel/` | Bazel command generation via `CommandTemplate::parse().render()` |
| **UnifiedRunner Facade** | `crates/core/src/runners/unified_runner.rs` | Thin facade delegating logic. |
| **Runners Infrastructure** | `crates/core/src/runners/` | Sub-modules like `build_system_detector`, `file_command`, and `package_resolver` to modularize execution dispatch. |
| **Config** | `crates/core/src/config/` | v2 schema: `BazelConfig`, `BazelOverride`, `Override` |
| **Error Handling** | `crates/core/src/error.rs` | Strongly-typed `thiserror` variants defining distinct error conditions without generic string wrappers. |
| **CLI Modules** | `crates/cli/src/{commands,config,display,utils}/` | Fully decoupled CLI routing. Each subcommand lives in its own module. |

### Stability & Idiomatic Rust
The Cargo Runner codebase is designed with strict resilience:
- **Zero Runtime Panics**: Extensive audits removed `.unwrap()` and `.expect()` calls in favor of propagating explicit results via `anyhow::Context` and the strongly-typed `crate::error::Error` variants.
- **Optimized Lifetimes**: Minimized heap allocations by pruning unnecessary `.clone()` occurrences across AST parsing and command generation.
- **Clippy Enforced**: Maintained under strict `cargo clippy --workspace` zero-warning standards for both libraries and test modules.

## ResolverChain

The resolver pipeline evaluates in priority order:

1. `IntegrationTestResolver` — files under `tests/`
2. `BinResolver` — files in `src/bin/`
3. _(more resolvers as needed)_
4. Generic fallback

## CommandTemplate DSL

Templates use `{placeholder}` syntax with conditionals:

```
{cmd?bazel} test {target} {?test_output:--test_output={test_output}} {?test_filter:--test_arg={test_filter}}
```

Render:
```rust
let cmd = CommandTemplate::parse(template_str)?.render(&ctx)?;
```

## Testing

```bash
# Run full test suite
cargo test -p cargo-runner-core

# Run a specific test module
cargo test -p cargo-runner-core bazel_builder
```

