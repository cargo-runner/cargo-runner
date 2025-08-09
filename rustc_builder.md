# RustC Builder Design Document

## Overview

The rustc builder needs to handle two distinct phases:
1. **Build Phase**: Compile the Rust source file into an executable
2. **Exec Phase**: Execute the compiled binary with appropriate arguments

## Current Issues

1. RustcConfig doesn't have test_framework and binary_framework fields
2. The command generation is mixing build and exec phases
3. No clear separation between compilation args and runtime args
4. Runnable detection creates cargo-type runnables even for standalone files

## Proposed Config Structures

### Option 1: Separate Build and Exec Sections

```json
{
  "rustc": {
    "extra_args": ["--edition", "2021"],
    "extra_env": {
      "RUST_BACKTRACE": "1"
    },
    "test_framework": {
      "build": {
        "command": "rustc",
        "args": ["--test", "{source_file}", "-o", "{output_name}"],
        "extra_args": ["--cfg", "test"]
      },
      "exec": {
        "command": "./{output_name}",
        "args": ["{test_name}"],
        "extra_args": [],
        "extra_test_binary_args": ["--nocapture", "--test-threads=1"]
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "args": ["--crate-type", "bin", "--crate-name", "{crate_name}", "{source_file}", "-o", "{output_name}"],
        "extra_args": []
      },
      "exec": {
        "command": "./{output_name}",
        "args": [],
        "extra_args": []
      }
    }
  }
}
```

### Option 2: Pipeline-based Config

```json
{
  "rustc": {
    "extra_env": {
      "RUST_BACKTRACE": "1"
    },
    "test_framework": {
      "pipeline": [
        {
          "phase": "build",
          "command": "rustc",
          "args": ["--test", "{source_file}", "-o", "{output_name}"],
          "extra_args": ["--cfg", "test"]
        },
        {
          "phase": "exec",
          "command": "./{output_name}",
          "args": ["{test_name}"],
          "separator": "--",
          "extra_test_binary_args": ["--nocapture"]
        }
      ]
    },
    "binary_framework": {
      "pipeline": [
        {
          "phase": "build",
          "command": "rustc",
          "args": ["--crate-type", "bin", "--crate-name", "{crate_name}", "{source_file}", "-o", "{output_name}"]
        },
        {
          "phase": "exec",
          "command": "./{output_name}",
          "args": []
        }
      ]
    }
  }
}
```

### Option 3: Template-based Config (Most Flexible)

```json
{
  "rustc": {
    "extra_env": {
      "RUST_BACKTRACE": "1"
    },
    "test": {
      "build_template": "rustc --test {source_file} -o {output_name} {extra_args}",
      "exec_template": "./{output_name} {test_name} {separator} {extra_test_binary_args}",
      "build_extra_args": ["--cfg", "test"],
      "exec_separator": "--",
      "exec_extra_test_binary_args": ["--nocapture"]
    },
    "binary": {
      "build_template": "rustc --crate-type bin --crate-name {crate_name} {source_file} -o {output_name} {extra_args}",
      "exec_template": "./{output_name} {extra_args}",
      "build_extra_args": [],
      "exec_extra_args": []
    }
  }
}
```

### Option 4: Minimal Config with Smart Defaults

```json
{
  "rustc": {
    "test": {
      "build_args": ["--cfg", "test"],
      "test_binary_args": ["--nocapture", "--test-threads=1"]
    },
    "binary": {
      "build_args": ["--edition", "2021"],
      "runtime_args": []
    }
  }
}
```

## TypeState Pattern Implementation

```rust
use std::marker::PhantomData;

// States
struct BuildPhase;
struct ExecPhase;

// Builder with TypeState
struct RustcCommandBuilder<State> {
    source_file: PathBuf,
    output_name: String,
    args: Vec<String>,
    _state: PhantomData<State>,
}

impl RustcCommandBuilder<BuildPhase> {
    fn new(source_file: PathBuf) -> Self {
        // Initialize build phase
    }
    
    fn add_build_args(mut self, args: Vec<String>) -> Self {
        // Add compilation arguments
    }
    
    fn build(self) -> Result<RustcCommandBuilder<ExecPhase>> {
        // Execute rustc compilation
        // Transition to ExecPhase
    }
}

impl RustcCommandBuilder<ExecPhase> {
    fn add_test_filter(mut self, test_name: String) -> Self {
        // Add test name to run
    }
    
    fn add_test_binary_args(mut self, args: Vec<String>) -> Self {
        // Add args after --
    }
    
    fn execute(self) -> Result<ExitStatus> {
        // Run the compiled binary
    }
}
```

## Config Merging Strategy

1. **Build Args Merging**:
   - Merge arrays, removing duplicates
   - Later configs override earlier ones for same flags
   - Handle `-o` output name specially (last one wins)

2. **Exec Args Merging**:
   - Test name from runnable (not configurable)
   - Extra test binary args append (don't override)
   - Separator is fixed as "--"

## TODOs

1. **Immediate Tasks**:
   - [ ] Add test_framework and binary_framework to RustcConfig struct
   - [ ] Implement typestate pattern for RustcCommandBuilder
   - [ ] Separate build and exec phases in command generation
   - [ ] Update config parsing to handle new structure

2. **Refactoring Tasks**:
   - [ ] Create separate builders for each phase (BuildCommandBuilder, ExecCommandBuilder)
   - [ ] Implement proper config merging for rustc settings
   - [ ] Add validation for config templates/args

3. **Testing Tasks**:
   - [ ] Test standalone file compilation
   - [ ] Test with various extra_args configurations
   - [ ] Test config merging behavior
   - [ ] Test error handling in both phases

## Current Runnable Detection Issue

The analyzer is showing cargo commands for individual runnables because:

1. **Runnable Detection**: The pattern detectors (test_fn.rs, binary.rs, etc.) create RunnableKind::Test, RunnableKind::Binary which are cargo-centric
2. **CommandBuilder Logic**: In builder/mod.rs, it routes based on RunnableKind, not FileType:
   ```rust
   match &self.runnable.kind {
       RunnableKind::Test { .. } => TestCommandBuilder::build(),  // This goes to cargo!
       RunnableKind::Binary { .. } => BinaryCommandBuilder::build(), // This too!
   }
   ```
3. **File Type Detection**: Even though we detect FileType::Standalone, we still use cargo builders

### Solution Approach

We need to check FileType FIRST, then RunnableKind:
```rust
match (file_type, &self.runnable.kind) {
    (FileType::Standalone, RunnableKind::Test { .. }) => RustcTestBuilder::build(),
    (FileType::Standalone, RunnableKind::Binary { .. }) => RustcBinaryBuilder::build(),
    (FileType::CargoProject, RunnableKind::Test { .. }) => TestCommandBuilder::build(),
    // ... etc
}
```

## Recommendations

I recommend **Option 1** (Separate Build and Exec Sections) because:
- Clear separation of concerns
- Easy to understand and configure
- Flexible enough for most use cases
- Natural fit for the typestate pattern

The config structure clearly shows what happens in each phase, making it easier for users to customize their build and execution separately.

## Next Steps

1. Decide on config structure (I vote Option 1)
2. Update RustcConfig to include test_framework and binary_framework
3. Fix the CommandBuilder routing to check FileType first
4. Implement the typestate pattern for build/exec phases
5. Update the RustcCommandBuilder to use the new config structure