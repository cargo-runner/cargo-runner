# Migration Guide: CargoRunner to UnifiedRunner

## Overview

The cargo-runner library has been refactored to provide a more flexible and extensible architecture. The main change is the replacement of `CargoRunner` with `UnifiedRunner`, which supports multiple build systems (Cargo, Bazel, Rustc) through a unified interface.

## Key Changes

### 1. Name Change: CargoRunner → UnifiedRunner

The misleading `CargoRunner` name has been replaced with `UnifiedRunner` to better reflect its multi-build-system support.

```rust
// Old
use cargo_runner_core::CargoRunner;
let mut runner = CargoRunner::new()?;

// New
use cargo_runner_core::UnifiedRunner;
let mut runner = UnifiedRunner::new()?;
```

### 2. New Architecture Benefits

- **Multiple Build Systems**: Seamlessly supports Cargo, Bazel, and standalone Rustc
- **Type-Safe Command Building**: New `CommandBuilder` with compile-time validation
- **Validation Rules**: Prevents invalid command combinations
- **Better Extensibility**: Easy to add new build systems

### 3. API Compatibility

The `UnifiedRunner` maintains full backward compatibility with `CargoRunner`'s API:

```rust
// All these methods work the same way
runner.detect_all_runnables(&path)?;
runner.get_best_runnable_at_line(&path, line)?;
runner.build_command_for_runnable(&runnable)?;
runner.get_command_at_position_with_dir(&filepath, line)?;
runner.analyze(&filepath)?;
runner.analyze_at_line(&filepath, line)?;
```

### 4. New Features

#### Validation

Commands are now validated before execution:

```rust
use cargo_runner_core::runner_v2::{CommandBuilder, FrameworkKind};
use cargo_runner_core::build_system::BuildSystem;

let command = CommandBuilder::new(BuildSystem::Cargo)
    .with_framework(FrameworkKind::Test)
    .with_all_features()
    .with_no_default_features() // This will fail at validation!
    .validate()?  // Returns error for conflicting options
    .build();
```

#### Configuration Validation

Configurations are validated before saving:

```rust
use cargo_runner_core::Config;

let config = Config::load()?;
// ... modify config ...
config.validate()?; // Check for conflicts
config.save_validated(&path)?; // Save only if valid
```

#### Build System Detection

Automatic detection of the appropriate build system:

```rust
let runner = UnifiedRunner::new()?;
let build_system = runner.detect_build_system(&path)?;
// Automatically uses the right runner (Cargo, Bazel, etc.)
```

## Migration Steps

### Step 1: Update Imports

```rust
// Replace
use cargo_runner_core::CargoRunner;

// With
use cargo_runner_core::UnifiedRunner;
```

### Step 2: Update Instantiation

```rust
// Replace
let mut runner = CargoRunner::new()?;

// With
let mut runner = UnifiedRunner::new()?;
```

### Step 3: (Optional) Use New Features

Take advantage of the new validation and type-safe building:

```rust
// Validate configurations
config.validate()?;

// Use type-safe command building
use cargo_runner_core::runner_v2::CommandBuilder;
let command = CommandBuilder::new(BuildSystem::Cargo)
    .with_package("my-crate")
    .with_features(vec!["async".to_string()])
    .validate()?
    .build();
```

## Breaking Changes

None! The migration is designed to be seamless. All existing code using `CargoRunner` will work with `UnifiedRunner`.

## Deprecation Timeline

1. **Current**: Both `CargoRunner` and `UnifiedRunner` are available
2. **Next Minor Version**: `CargoRunner` will be marked as deprecated
3. **Next Major Version**: `CargoRunner` will be removed

## Advanced Usage

### Custom Validation Rules

```rust
use cargo_runner_core::runner_v2::validation::{ValidationRule, ValidationError};

struct MyCustomRule;
impl ValidationRule for MyCustomRule {
    fn validate(&self, options: &CommandOptions) -> Result<(), ValidationError> {
        // Custom validation logic
        Ok(())
    }
    // ...
}
```

### Extending to New Build Systems

The new architecture makes it easy to add support for new build systems:

```rust
use cargo_runner_core::runner_v2::traits::CommandRunner;

struct MyBuildSystemRunner;
impl CommandRunner for MyBuildSystemRunner {
    // Implementation
}
```

## Getting Help

If you encounter any issues during migration:

1. Check that all imports are updated
2. Ensure you're using the latest version
3. File an issue on GitHub with your specific use case

## Example Migration

### Before

```rust
use cargo_runner_core::{CargoRunner, Config};
use std::path::Path;

fn main() -> Result<()> {
    let mut runner = CargoRunner::new()?;
    let path = Path::new("src/main.rs");
    
    if let Some(runnable) = runner.get_best_runnable_at_line(path, 42)? {
        if let Some(command) = runner.build_command_for_runnable(&runnable)? {
            println!("Command: {}", command.to_shell_command());
        }
    }
    
    Ok(())
}
```

### After

```rust
use cargo_runner_core::{UnifiedRunner, Config};
use std::path::Path;

fn main() -> Result<()> {
    let mut runner = UnifiedRunner::new()?;
    let path = Path::new("src/main.rs");
    
    if let Some(runnable) = runner.get_best_runnable_at_line(path, 42)? {
        if let Some(command) = runner.build_command_for_runnable(&runnable)? {
            println!("Command: {}", command.to_shell_command());
        }
    }
    
    Ok(())
}
```

The only change needed is the type name!