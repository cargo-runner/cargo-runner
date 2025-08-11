# CLI Refactoring Plan

The `main.rs` file has grown to 2,481 lines. Here's a plan to refactor it into a more maintainable module structure.

## Proposed Module Structure

```
crates/cli/src/
├── main.rs                    # Entry point, CLI setup (~100 lines)
├── lib.rs                     # Module declarations
├── cli.rs                     # CLI argument definitions (Commands enum)
├── commands/
│   ├── mod.rs
│   ├── analyze.rs             # analyze_command and related functions
│   ├── run.rs                 # run_command
│   ├── init.rs                # init_command
│   ├── unset.rs               # unset_command
│   └── override.rs            # override_command and related functions
├── config/
│   ├── mod.rs
│   ├── generators.rs          # Config generation functions
│   ├── templates.rs           # Config templates (rustc, bazel, etc.)
│   └── workspace.rs           # Workspace detection and handling
├── display/
│   ├── mod.rs
│   ├── formatter.rs           # Formatting functions for output
│   ├── command_breakdown.rs   # print_command_breakdown
│   └── analysis.rs            # print_formatted_analysis
├── utils/
│   ├── mod.rs
│   ├── parser.rs              # parse_filepath_with_line, parse_override_args
│   └── file.rs                # File type detection, path utilities
└── error.rs                   # Custom error types

## Refactoring Steps

### Step 1: Create Module Structure
```bash
mkdir -p crates/cli/src/{commands,config,display,utils}
touch crates/cli/src/lib.rs
touch crates/cli/src/cli.rs
touch crates/cli/src/error.rs
touch crates/cli/src/commands/{mod.rs,analyze.rs,run.rs,init.rs,unset.rs,override.rs}
touch crates/cli/src/config/{mod.rs,generators.rs,templates.rs,workspace.rs}
touch crates/cli/src/display/{mod.rs,formatter.rs,command_breakdown.rs,analysis.rs}
touch crates/cli/src/utils/{mod.rs,parser.rs,file.rs}
```

### Step 2: Move CLI Definitions
Move to `cli.rs`:
- `Cargo` struct
- `CargoCommand` enum
- `Runner` struct
- `Commands` enum

### Step 3: Extract Commands
Each command gets its own file in `commands/`:

**analyze.rs**: (~300 lines)
- `analyze_command()`
- `print_formatted_analysis()`
- Related helper functions

**run.rs**: (~50 lines)
- `run_command()`

**init.rs**: (~400 lines)
- `init_command()`
- Package detection logic
- Config file creation logic

**unset.rs**: (~50 lines)
- `unset_command()`

**override.rs**: (~800 lines)
- `override_command()`
- `create_file_level_override()`
- `add_override_to_existing_config()`
- `parse_override_args()`

### Step 4: Extract Config Generation
Move to `config/`:

**generators.rs**:
- `create_default_config()`
- `create_root_config()`
- `create_workspace_config()`

**templates.rs**:
- `create_rustc_config()`
- `create_combined_config()`
- `create_single_file_script_config()`

**workspace.rs**:
- `is_workspace_only()`
- `get_package_name()`

### Step 5: Extract Display Functions
Move to `display/`:

**command_breakdown.rs**:
- `print_command_breakdown()`
- `parse_cargo_command()`

**analysis.rs**:
- `print_runnable_type()`
- Analysis output formatting

**formatter.rs**:
- General formatting utilities

### Step 6: Extract Utilities
Move to `utils/`:

**parser.rs**:
- `parse_filepath_with_line()`
- `parse_override_args()`

**file.rs**:
- `determine_file_type()`
- Path resolution utilities

### Step 7: Update main.rs
The main.rs should only contain:
- `main()` function
- Basic CLI setup
- Command dispatch to module functions

## Benefits

1. **Separation of Concerns**: Each module has a clear, single responsibility
2. **Easier Testing**: Individual modules can be tested in isolation
3. **Better Code Organization**: Related functions are grouped together
4. **Reduced Cognitive Load**: Developers can focus on one module at a time
5. **Parallel Development**: Multiple developers can work on different modules
6. **Reusability**: Modules can be reused in other contexts (e.g., library API)

## Implementation Order

1. Start with utilities and display modules (least dependencies)
2. Move config generation functions
3. Extract individual commands
4. Update main.rs and create lib.rs
5. Add tests for each module

## Example: Refactored main.rs

```rust
mod cli;
mod commands;
mod config;
mod display;
mod utils;
mod error;

use clap::Parser;
use anyhow::Result;
use cli::{Cargo, Commands};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cargo = Cargo::parse_from(std::env::args_os().skip(1));
    
    match cargo.command {
        Some(commands) => match commands {
            Commands::Analyze { filepath, verbose, config } => 
                commands::analyze::execute(&filepath, verbose, config),
            Commands::Run { filepath, dry_run } => 
                commands::run::execute(&filepath, dry_run),
            Commands::Init { cwd, force, rustc, single_file_script } => 
                commands::init::execute(cwd.as_deref(), force, rustc, single_file_script),
            Commands::Unset { clean } => 
                commands::unset::execute(clean),
            Commands::Override { filepath, root, override_args } => 
                commands::override_cmd::execute(&filepath, root, override_args),
        },
        None => {
            eprintln!("No subcommand provided. Use --help for usage information.");
            std::process::exit(1);
        }
    }
}
```

## Testing Strategy

Each module should have its own test file:
- `commands/analyze_test.rs`
- `config/generators_test.rs`
- `utils/parser_test.rs`
- etc.

This makes it easier to maintain test coverage and run focused tests during development.