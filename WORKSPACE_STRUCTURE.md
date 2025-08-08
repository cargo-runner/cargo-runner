# Workspace Structure

The project has been restructured as a Cargo workspace with two crates:

## crates/core
- The core library (`cargo-runner-core`)
- Contains all the parsing, detection, and command generation logic
- Exposes the main `CargoRunner` API

## crates/cli
- The CLI binary (`cargo-runner`)
- Provides two subcommands:
  - `cargo runner analyze <filepath>` - Analyzes and lists all runnables in a file (equivalent to old `cargo-r`)
  - `cargo runner run <filepath[:line]>` - Runs code at a specific location (equivalent to old `cargo-exec`)
- Supports `--dry-run` or `-d` flag to show commands without executing
- Respects `RUST_LOG` environment variable for debug output

## Usage Examples

```bash
# Analyze a file
cargo runner analyze src/lib.rs

# Run a test at a specific line
cargo runner run src/lib.rs:42

# Show command without executing
cargo runner run src/lib.rs:42 --dry-run

# Enable debug logging
RUST_LOG=debug cargo runner analyze src/lib.rs
```

## Building

```bash
# Build everything
cargo build

# Build release version
cargo build --release

# Install locally
cargo install --path crates/cli
```