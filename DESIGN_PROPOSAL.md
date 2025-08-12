# Cargo Runner Design Refactoring Proposal

## Current Issues
1. **Naming Confusion**: `CargoRunner` handles multiple build systems (Cargo, Bazel, rustc)
2. **Limited Extensibility**: Hard to add new build systems
3. **No Validation**: Options can conflict (e.g., `--all-features` with `--no-default-features`)
4. **Monolithic Design**: All logic bundled together

## Proposed Architecture

### 1. Core Abstractions

```rust
// Core trait for any command runner
pub trait CommandRunner {
    type Config;
    type Command;
    
    fn detect_runnables(&self, file: &Path) -> Result<Vec<Runnable>>;
    fn build_command(&self, runnable: &Runnable, config: &Self::Config) -> Result<Self::Command>;
    fn validate_command(&self, command: &Self::Command) -> Result<()>;
}

// Specific implementations
pub struct CargoRunner;
pub struct BazelRunner;
pub struct RustcRunner;
```

### 2. Framework System

```rust
// Base trait for all frameworks
pub trait Framework {
    type Options;
    
    fn name(&self) -> &'static str;
    fn validate_options(&self, options: &Self::Options) -> Result<()>;
    fn build_args(&self, options: &Self::Options) -> Vec<String>;
}

// Specific frameworks
pub struct TestFramework;
pub struct BinaryFramework;
pub struct BenchmarkFramework;
pub struct DocTestFramework;
pub struct BuildFramework;
```

### 3. Option Groups with Validation Rules

```rust
// Option categories based on cargo help output
pub mod options {
    #[derive(Debug, Clone)]
    pub struct PackageSelection {
        pub package: Option<Vec<String>>,
        pub workspace: bool,
        pub exclude: Vec<String>,
        pub all: bool, // deprecated alias for workspace
    }
    
    #[derive(Debug, Clone)]
    pub struct TargetSelection {
        pub lib: bool,
        pub bins: bool,
        pub bin: Option<Vec<String>>,
        pub examples: bool,
        pub example: Option<Vec<String>>,
        pub tests: bool,
        pub test: Option<Vec<String>>,
        pub benches: bool,
        pub bench: Option<Vec<String>>,
        pub all_targets: bool,
        pub doc: bool,
    }
    
    #[derive(Debug, Clone)]
    pub struct FeatureSelection {
        pub features: Vec<String>,
        pub all_features: bool,
        pub no_default_features: bool,
    }
    
    #[derive(Debug, Clone)]
    pub struct CompilationOptions {
        pub jobs: Option<u32>,
        pub release: bool,
        pub profile: Option<String>,
        pub target: Option<String>,
        pub target_dir: Option<PathBuf>,
        pub timings: Option<Vec<String>>,
        pub unit_graph: bool,
    }
    
    #[derive(Debug, Clone)]
    pub struct ManifestOptions {
        pub manifest_path: Option<PathBuf>,
        pub lockfile_path: Option<PathBuf>,
        pub ignore_rust_version: bool,
        pub locked: bool,
        pub offline: bool,
        pub frozen: bool,
    }
    
    #[derive(Debug, Clone)]
    pub struct TestOptions {
        pub no_run: bool,
        pub no_fail_fast: bool,
        pub test_threads: Option<u32>,
        pub nocapture: bool,
        pub exact: bool,
        pub quiet: bool,
        pub show_output: bool,
        pub ignored: bool,
        pub include_ignored: bool,
    }
}
```

### 4. Validation Rules Engine

```rust
pub trait ValidationRule {
    fn validate(&self, options: &CommandOptions) -> Result<()>;
    fn description(&self) -> &str;
}

// Example validation rules
pub struct MutuallyExclusiveRule {
    fields: Vec<String>,
    message: String,
}

pub struct RequiredIfRule {
    condition_field: String,
    required_field: String,
    message: String,
}

pub struct ConflictingOptionsRule {
    option1: String,
    option2: String,
    message: String,
}

// Validation rules for cargo
pub fn cargo_validation_rules() -> Vec<Box<dyn ValidationRule>> {
    vec![
        // Features conflicts
        Box::new(ConflictingOptionsRule {
            option1: "all_features".to_string(),
            option2: "no_default_features".to_string(),
            message: "--all-features and --no-default-features cannot be used together".to_string(),
        }),
        
        // Target conflicts
        Box::new(MutuallyExclusiveRule {
            fields: vec!["lib".to_string(), "bin".to_string()],
            message: "--lib and --bin cannot be used together".to_string(),
        }),
        
        // Test-specific conflicts
        Box::new(ConflictingOptionsRule {
            option1: "doc".to_string(),
            option2: "test".to_string(),
            message: "--doc and --test cannot be used together".to_string(),
        }),
        
        // Manifest conflicts
        Box::new(ConflictingOptionsRule {
            option1: "frozen".to_string(),
            option2: "update".to_string(),
            message: "--frozen implies --locked and --offline, conflicts with update operations".to_string(),
        }),
    ]
}
```

### 5. Type-State Pattern for Safe Command Building

```rust
// Type states for command building
pub struct Unvalidated;
pub struct Validated;

pub struct CommandBuilder<State = Unvalidated> {
    runner_type: RunnerType,
    framework: Box<dyn Framework>,
    package_selection: PackageSelection,
    target_selection: TargetSelection,
    feature_selection: FeatureSelection,
    compilation_options: CompilationOptions,
    manifest_options: ManifestOptions,
    _phantom: PhantomData<State>,
}

impl CommandBuilder<Unvalidated> {
    pub fn new(runner_type: RunnerType) -> Self { ... }
    
    pub fn with_package(mut self, package: &str) -> Self { ... }
    pub fn with_workspace(mut self) -> Self { ... }
    pub fn with_features(mut self, features: Vec<String>) -> Self { ... }
    
    // Validate transitions to Validated state
    pub fn validate(self) -> Result<CommandBuilder<Validated>> {
        let rules = match self.runner_type {
            RunnerType::Cargo => cargo_validation_rules(),
            RunnerType::Bazel => bazel_validation_rules(),
            RunnerType::Rustc => rustc_validation_rules(),
        };
        
        for rule in rules {
            rule.validate(&self.to_options())?;
        }
        
        Ok(CommandBuilder {
            runner_type: self.runner_type,
            framework: self.framework,
            package_selection: self.package_selection,
            target_selection: self.target_selection,
            feature_selection: self.feature_selection,
            compilation_options: self.compilation_options,
            manifest_options: self.manifest_options,
            _phantom: PhantomData,
        })
    }
}

impl CommandBuilder<Validated> {
    // Only validated builders can build commands
    pub fn build(self) -> Command {
        self.framework.build_command(&self.to_options())
    }
}
```

### 6. Configuration Validation

```rust
// Ensure configs are valid before saving
pub trait ConfigValidator {
    fn validate(&self, config: &Config) -> Result<()>;
}

pub struct CargoConfigValidator;

impl ConfigValidator for CargoConfigValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Check for conflicting options in overrides
        for override_config in &config.overrides {
            if let Some(cargo_args) = &override_config.cargo_args {
                // Parse and validate cargo args
                let options = parse_cargo_args(cargo_args)?;
                let rules = cargo_validation_rules();
                for rule in rules {
                    rule.validate(&options)?;
                }
            }
        }
        Ok(())
    }
}
```

### 7. Unified Runner Interface

```rust
pub struct UnifiedRunner {
    runners: HashMap<BuildSystem, Box<dyn CommandRunner>>,
}

impl UnifiedRunner {
    pub fn new() -> Self {
        let mut runners = HashMap::new();
        runners.insert(BuildSystem::Cargo, Box::new(CargoRunner::new()));
        runners.insert(BuildSystem::Bazel, Box::new(BazelRunner::new()));
        runners.insert(BuildSystem::Rustc, Box::new(RustcRunner::new()));
        
        Self { runners }
    }
    
    pub fn detect_build_system(&self, path: &Path) -> Result<BuildSystem> {
        DefaultBuildSystemDetector::detect(path)
            .ok_or_else(|| anyhow!("No build system detected"))
    }
    
    pub fn get_runner(&self, build_system: &BuildSystem) -> Result<&dyn CommandRunner> {
        self.runners.get(build_system)
            .map(|r| r.as_ref())
            .ok_or_else(|| anyhow!("No runner for build system: {:?}", build_system))
    }
}
```

## Migration Strategy

1. **Phase 1**: Create new abstractions alongside existing code
2. **Phase 2**: Implement validation rules and type-state builder
3. **Phase 3**: Migrate existing functionality to new architecture
4. **Phase 4**: Deprecate old `CargoRunner` name, introduce `UnifiedRunner`
5. **Phase 5**: Remove deprecated code

## Benefits

1. **Clear Separation**: Each build system has its own runner
2. **Type Safety**: Type-state pattern prevents invalid commands
3. **Validation**: Rules engine catches conflicts early
4. **Extensibility**: Easy to add new build systems or frameworks
5. **Maintainability**: Modular design with clear responsibilities
6. **User Experience**: Better error messages for invalid configurations

## Example Usage

```rust
// Old way
let runner = CargoRunner::new()?;
let command = runner.get_command_at_position(&path, Some(42))?;

// New way with validation
let runner = UnifiedRunner::new();
let build_system = runner.detect_build_system(&path)?;
let command = CommandBuilder::new(RunnerType::Cargo)
    .with_framework(TestFramework)
    .with_package("my-crate")
    .with_features(vec!["async".to_string()])
    .with_release()
    .validate()?  // Returns error if invalid combination
    .build();
```

## Implementation Priority

1. **High Priority**:
   - Core abstractions (CommandRunner trait, Framework trait)
   - Validation rules engine
   - Type-state command builder

2. **Medium Priority**:
   - Migration of existing functionality
   - Configuration validation
   - Comprehensive test suite

3. **Low Priority**:
   - Advanced features (custom validation rules)
   - Performance optimizations
   - Additional build system support