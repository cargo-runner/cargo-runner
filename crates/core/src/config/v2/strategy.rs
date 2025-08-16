//! Framework strategy pattern for command building
//!
//! Defines the strategy trait and common implementations.

use crate::command::CargoCommand;
use crate::types::RunnableKind;
use std::path::Path;

// Re-export FrameworkKind for convenience
pub use crate::runners::framework::FrameworkKind;

/// Context for building commands
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub file_path: Option<String>,
    pub crate_name: Option<String>,
    pub package_name: Option<String>,
    pub module_path: Option<String>,
    pub function_name: Option<String>,
    pub runnable_kind: RunnableKind,
    pub working_dir: Option<String>,
}

/// Strategy for building framework-specific commands
pub trait FrameworkStrategy: Send + Sync {
    /// Build a command for the given context
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String>;

    /// Get the name of this strategy
    fn name(&self) -> &str;

    /// Get the framework kind this strategy handles
    fn framework_kind(&self) -> FrameworkKind;
}

/// Base implementation for Cargo-based strategies
pub struct CargoStrategy {
    pub name: String,
    pub command: String,
    pub subcommand: String,
    pub channel: Option<String>,
    pub default_args: Vec<String>,
}

impl CargoStrategy {
    pub fn new(name: impl Into<String>, subcommand: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: "cargo".into(),
            subcommand: subcommand.into(),
            channel: None,
            default_args: Vec::new(),
        }
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.default_args = args;
        self
    }
}

/// Cargo test strategy
pub struct CargoTestStrategy {
    base: CargoStrategy,
}

impl CargoTestStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-test", "test"),
        }
    }
}

impl FrameworkStrategy for CargoTestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Check if we need --bin, --lib, --example, or --bench based on file path
        if let Some(file_path) = &context.file_path {
            if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                // For benchmark files, use --bench flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--bench".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.contains("/examples/") || file_path.contains("\\examples\\") {
                // For example files, use --example flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--example".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.ends_with("src/main.rs") || file_path.ends_with("/src/main.rs") {
                // For src/main.rs, add --bin flag
                if let Some(package) = &context.package_name {
                    args.push("--bin".into());
                    args.push(package.clone());
                }
            } else if file_path.ends_with("src/lib.rs") || file_path.ends_with("/src/lib.rs") {
                // For src/lib.rs, add --lib flag
                args.push("--lib".into());
            }
        }

        // Check if this is a DocTest
        if let RunnableKind::DocTest {
            struct_or_module_name,
            method_name,
        } = &context.runnable_kind
        {
            // For doctests, add --doc flag
            args.push("--doc".into());

            // Add default args
            args.extend(self.base.default_args.clone());

            // Doc test filtering works differently
            args.push("--".into());

            if let Some(method) = method_name {
                // For method doctests, use struct::method format
                args.push(format!("{}::{}", struct_or_module_name, method));
            } else {
                // For struct/module doctests, just use the name
                args.push(struct_or_module_name.clone());
            }
        } else {
            // Regular test handling
            // Add default args
            args.extend(self.base.default_args.clone());

            // Handle different runnable kinds
            match &context.runnable_kind {
                RunnableKind::Test { test_name, .. } => {
                    // For specific test functions, add the test filter
                    args.push("--".into());

                    // Build test path - for benchmark files, use shorter path
                    if let Some(file_path) = &context.file_path {
                        if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                            // For tests in benchmark files, strip the bench file prefix
                            if let Some(module) = &context.module_path {
                                // Remove "benches::filename::" prefix to get just the module path
                                let parts: Vec<&str> = module.split("::").collect();
                                if parts.len() > 2 && parts[0] == "benches" {
                                    // Skip "benches::filename::" and use the rest
                                    let short_module = parts[2..].join("::");
                                    if !short_module.is_empty() {
                                        args.push(format!("{}::{}", short_module, test_name));
                                    } else {
                                        args.push(test_name.clone());
                                    }
                                } else {
                                    args.push(test_name.clone());
                                }
                            } else {
                                args.push(test_name.clone());
                            }
                        } else {
                            // Regular test path
                            if let Some(module) = &context.module_path {
                                if !module.is_empty() {
                                    args.push(format!("{}::{}", module, test_name));
                                } else {
                                    args.push(test_name.clone());
                                }
                            } else {
                                args.push(test_name.clone());
                            }
                        }
                    } else {
                        // No file path, use regular format
                        if let Some(module) = &context.module_path {
                            if !module.is_empty() {
                                args.push(format!("{}::{}", module, test_name));
                            } else {
                                args.push(test_name.clone());
                            }
                        } else {
                            args.push(test_name.clone());
                        }
                    }

                    args.push("--exact".into());
                }
                RunnableKind::ModuleTests { module_name } => {
                    // For module tests, add filter without --exact
                    if !module_name.is_empty() {
                        args.push("--".into());
                        // For benchmark files, use just the module name without prefix
                        if let Some(file_path) = &context.file_path {
                            if file_path.contains("/benches/") || file_path.contains("\\benches\\")
                            {
                                // Remove "benches::filename::" prefix
                                let parts: Vec<&str> = module_name.split("::").collect();
                                if parts.len() > 2 && parts[0] == "benches" {
                                    let short_module = parts[2..].join("::");
                                    args.push(short_module);
                                } else if parts.len() == 1 {
                                    // Already short form
                                    args.push(module_name.clone());
                                } else {
                                    args.push(module_name.clone());
                                }
                            } else {
                                args.push(module_name.clone());
                            }
                        } else {
                            args.push(module_name.clone());
                        }
                        // No --exact for module tests
                    }
                    // Otherwise run all tests in the crate
                }
                _ => {
                    // For other kinds, use function_name if available
                    if let Some(test_name) = &context.function_name {
                        args.push("--".into());

                        // Build full test path
                        if let Some(module) = &context.module_path {
                            args.push(format!("{}::{}", module, test_name));
                        } else {
                            args.push(test_name.clone());
                        }

                        args.push("--exact".into());
                    }
                }
            }
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }
}

/// Cargo nextest strategy
pub struct CargoNextestStrategy {
    base: CargoStrategy,
}

impl CargoNextestStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-nextest", "nextest").with_args(vec!["run".into()]),
        }
    }
}

impl FrameworkStrategy for CargoNextestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());
        args.extend(self.base.default_args.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Check if we need --bin, --lib, --example, or --bench based on file path
        if let Some(file_path) = &context.file_path {
            if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                // For benchmark files, use --bench flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--bench".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.contains("/examples/") || file_path.contains("\\examples\\") {
                // For example files, use --example flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--example".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.ends_with("src/main.rs") || file_path.ends_with("/src/main.rs") {
                // For src/main.rs, add --bin flag
                if let Some(package) = &context.package_name {
                    args.push("--bin".into());
                    args.push(package.clone());
                }
            } else if file_path.ends_with("src/lib.rs") || file_path.ends_with("/src/lib.rs") {
                // For src/lib.rs, add --lib flag
                args.push("--lib".into());
            }
        }

        // Add test filter based on runnable kind
        match &context.runnable_kind {
            RunnableKind::Test { test_name, .. } => {
                // For specific test functions, add the test filter
                if let Some(file_path) = &context.file_path {
                    if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                        // For tests in benchmark files, strip the bench file prefix
                        if let Some(module) = &context.module_path {
                            // Remove "benches::filename::" prefix to get just the module path
                            let parts: Vec<&str> = module.split("::").collect();
                            if parts.len() > 2 && parts[0] == "benches" {
                                // Skip "benches::filename::" and use the rest
                                let short_module = parts[2..].join("::");
                                if !short_module.is_empty() {
                                    args.push(format!("{}::{}", short_module, test_name));
                                } else {
                                    args.push(test_name.clone());
                                }
                            } else {
                                args.push(test_name.clone());
                            }
                        } else {
                            args.push(test_name.clone());
                        }
                    } else {
                        // Regular test path
                        if let Some(module) = &context.module_path {
                            if !module.is_empty() {
                                args.push(format!("{}::{}", module, test_name));
                            } else {
                                args.push(test_name.clone());
                            }
                        } else {
                            args.push(test_name.clone());
                        }
                    }
                } else {
                    // No file path, use regular format
                    if let Some(module) = &context.module_path {
                        if !module.is_empty() {
                            args.push(format!("{}::{}", module, test_name));
                        } else {
                            args.push(test_name.clone());
                        }
                    } else {
                        args.push(test_name.clone());
                    }
                }
            }
            RunnableKind::ModuleTests { module_name } => {
                // For module tests, only add filter if module is specified
                if !module_name.is_empty() {
                    // For benchmark files, use just the module name without prefix
                    if let Some(file_path) = &context.file_path {
                        if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                            // Remove "benches::filename::" prefix
                            let parts: Vec<&str> = module_name.split("::").collect();
                            if parts.len() > 2 && parts[0] == "benches" {
                                let short_module = parts[2..].join("::");
                                args.push(short_module);
                            } else if parts.len() == 1 {
                                // Already short form
                                args.push(module_name.clone());
                            } else {
                                args.push(module_name.clone());
                            }
                        } else {
                            args.push(module_name.clone());
                        }
                    } else {
                        args.push(module_name.clone());
                    }
                }
                // Otherwise run all tests in the crate
            }
            _ => {
                // For other kinds (file-level), don't add any filter
            }
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }
}

/// Cargo run strategy for binaries
pub struct CargoRunStrategy {
    base: CargoStrategy,
}

impl CargoRunStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-run", "run"),
        }
    }
}

impl FrameworkStrategy for CargoRunStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Add binary name based on the runnable kind
        if let RunnableKind::Binary { bin_name } = &context.runnable_kind {
            if let Some(name) = bin_name {
                // Explicit binary name (e.g., from src/bin/foo.rs)
                args.push("--bin".into());
                args.push(name.clone());
            } else if let Some(file_path) = &context.file_path {
                // Check if this is an example file
                if file_path.contains("/examples/") || file_path.contains("\\examples\\") {
                    // For example files, use --example flag
                    if let Some(stem) = Path::new(file_path).file_stem() {
                        args.push("--example".into());
                        args.push(stem.to_string_lossy().to_string());
                    }
                } else if file_path.ends_with("src/main.rs") || file_path.ends_with("/src/main.rs")
                {
                    // For src/main.rs, use the package name as binary name
                    if let Some(package) = &context.package_name {
                        args.push("--bin".into());
                        args.push(package.clone());
                    }
                } else if let Some(stem) = Path::new(file_path).file_stem() {
                    // For other binary files (src/bin/*.rs), use the file stem
                    args.push("--bin".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            }
        }

        // Add default args
        args.extend(self.base.default_args.clone());

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Cargo bench strategy
pub struct CargoBenchStrategy {
    base: CargoStrategy,
}

impl CargoBenchStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-bench", "bench"),
        }
    }
}

impl FrameworkStrategy for CargoBenchStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Check if we need --bin, --lib, --example, or --bench based on file path
        if let Some(file_path) = &context.file_path {
            if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                // For benchmark files, use --bench flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--bench".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.contains("/examples/") || file_path.contains("\\examples\\") {
                // For example files, use --example flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--example".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if file_path.ends_with("src/main.rs") || file_path.ends_with("/src/main.rs") {
                // For src/main.rs, add --bin flag
                if let Some(package) = &context.package_name {
                    args.push("--bin".into());
                    args.push(package.clone());
                }
            } else if file_path.ends_with("src/lib.rs") || file_path.ends_with("/src/lib.rs") {
                // For src/lib.rs, add --lib flag
                args.push("--lib".into());
            }
        }

        // Add default args
        args.extend(self.base.default_args.clone());

        // Add benchmark target based on context
        if let RunnableKind::Benchmark { bench_name } = &context.runnable_kind {
            // For benchmark runnable kind
            args.push("--bench".into());
            args.push(bench_name.clone());
        } else if let Some(file_path) = &context.file_path {
            // Check if this is a benchmark file
            if file_path.contains("/benches/") || file_path.contains("\\benches\\") {
                // For benchmark files, use --bench flag
                if let Some(stem) = Path::new(file_path).file_stem() {
                    args.push("--bench".into());
                    args.push(stem.to_string_lossy().to_string());
                }
            } else if let Some(bench_name) = &context.function_name {
                // For other files with function name
                args.push("--bench".into());
                args.push(bench_name.clone());
            }
        } else if let Some(bench_name) = &context.function_name {
            // Fallback to function name
            args.push("--bench".into());
            args.push(bench_name.clone());
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Benchmark
    }
}

/// Cargo doctest strategy
pub struct CargoDocTestStrategy {
    base: CargoStrategy,
}

/// Bazel test strategy
pub struct BazelTestStrategy {
    name: String,
}

impl BazelTestStrategy {
    pub fn new() -> Self {
        Self {
            name: "bazel-test".to_string(),
        }
    }

    fn find_target(
        &self,
        file_path: &str,
        kind: crate::bazel::BazelTargetKind,
    ) -> Result<String, String> {
        use crate::bazel::{BazelTargetFinder, BazelTargetKind};
        use std::path::Path;

        // Find the BUILD file
        let path = Path::new(file_path);
        let dir = path.parent().ok_or("Invalid file path")?;

        // Look for BUILD.bazel or BUILD file
        let build_file = if dir.join("BUILD.bazel").exists() {
            dir.join("BUILD.bazel")
        } else if dir.join("BUILD").exists() {
            dir.join("BUILD")
        } else {
            // Walk up to find BUILD file
            let mut current = dir;
            loop {
                if current.join("BUILD.bazel").exists() {
                    break current.join("BUILD.bazel");
                } else if current.join("BUILD").exists() {
                    break current.join("BUILD");
                }
                current = current.parent().ok_or("No BUILD file found")?;
            }
        };

        // Parse BUILD file to find targets
        let mut finder = BazelTargetFinder::new()
            .map_err(|e| format!("Failed to create target finder: {}", e))?;
        let targets = finder
            .find_targets_in_build_file(&build_file)
            .map_err(|e| format!("Failed to parse BUILD file: {}", e))?;

        // Find target that uses this source file and matches the kind
        for target in targets {
            if target.kind == kind && target.sources.iter().any(|src| file_path.ends_with(src)) {
                return Ok(target.label);
            }
        }

        Err(format!(
            "No {} target found for {}",
            match kind {
                BazelTargetKind::Test => "test",
                BazelTargetKind::Binary => "binary",
                _ => "matching",
            },
            file_path
        ))
    }
}

impl FrameworkStrategy for BazelTestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["test".to_string()];

        // Find the actual test target for this file
        if let Some(file_path) = &context.file_path {
            match self.find_target(file_path, crate::bazel::BazelTargetKind::Test) {
                Ok(target) => args.push(target),
                Err(e) => return Err(e),
            }

            // Add test filter if specified
            if let Some(test_name) = &context.function_name {
                args.push(format!("--test_filter={}", test_name));
            }
        }

        let mut command = CargoCommand::new_bazel(args);

        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }
}

/// Bazel run strategy
pub struct BazelRunStrategy {
    name: String,
}

impl BazelRunStrategy {
    pub fn new() -> Self {
        Self {
            name: "bazel-run".to_string(),
        }
    }

    fn find_target(
        &self,
        file_path: &str,
        kind: crate::bazel::BazelTargetKind,
    ) -> Result<String, String> {
        use crate::bazel::{BazelTargetFinder, BazelTargetKind};
        use std::path::Path;

        // Find the BUILD file
        let path = Path::new(file_path);
        let dir = path.parent().ok_or("Invalid file path")?;

        // Look for BUILD.bazel or BUILD file
        let build_file = if dir.join("BUILD.bazel").exists() {
            dir.join("BUILD.bazel")
        } else if dir.join("BUILD").exists() {
            dir.join("BUILD")
        } else {
            // Walk up to find BUILD file
            let mut current = dir;
            loop {
                if current.join("BUILD.bazel").exists() {
                    break current.join("BUILD.bazel");
                } else if current.join("BUILD").exists() {
                    break current.join("BUILD");
                }
                current = current.parent().ok_or("No BUILD file found")?;
            }
        };

        // Parse BUILD file to find targets
        let mut finder = BazelTargetFinder::new()
            .map_err(|e| format!("Failed to create target finder: {}", e))?;
        let targets = finder
            .find_targets_in_build_file(&build_file)
            .map_err(|e| format!("Failed to parse BUILD file: {}", e))?;

        // Find target that uses this source file and matches the kind
        for target in targets {
            if target.kind == kind && target.sources.iter().any(|src| file_path.ends_with(src)) {
                return Ok(target.label);
            }
        }

        Err(format!(
            "No {} target found for {}",
            match kind {
                BazelTargetKind::Binary => "binary",
                BazelTargetKind::Test => "test",
                _ => "matching",
            },
            file_path
        ))
    }
}

impl FrameworkStrategy for BazelRunStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["run".to_string()];

        // Find the actual binary target for this file
        if let Some(file_path) = &context.file_path {
            match self.find_target(file_path, crate::bazel::BazelTargetKind::Binary) {
                Ok(target) => args.push(target),
                Err(e) => return Err(e),
            }
        }

        let mut command = CargoCommand::new_bazel(args);

        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Bazel benchmark strategy
pub struct BazelBenchStrategy {
    name: String,
}

impl BazelBenchStrategy {
    pub fn new() -> Self {
        Self {
            name: "bazel-bench".to_string(),
        }
    }
}

impl FrameworkStrategy for BazelBenchStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["run".to_string()];

        // Add benchmark target
        if let Some(bench_name) = &context.function_name {
            args.push(format!("//:{}_bench", bench_name));
        }

        let mut command = CargoCommand::new_bazel(args);

        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Benchmark
    }
}

impl CargoDocTestStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-doctest", "test").with_args(vec!["--doc".into()]),
        }
    }
}

impl FrameworkStrategy for CargoDocTestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Add --doc flag for doc tests
        args.extend(self.base.default_args.clone());

        // For doctests, we need to handle the special case of struct/module names
        if let RunnableKind::DocTest {
            struct_or_module_name,
            method_name,
        } = &context.runnable_kind
        {
            // Doc test filtering works differently - we need to use the full path
            args.push("--".into());

            if let Some(method) = method_name {
                // For method doctests, use struct::method format
                args.push(format!("{}::{}", struct_or_module_name, method));
            } else {
                // For struct/module doctests, just use the name
                args.push(struct_or_module_name.clone());
            }
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::DocTest
    }
}

/// Cargo build strategy
pub struct CargoBuildStrategy {
    base: CargoStrategy,
}

impl CargoBuildStrategy {
    pub fn new() -> Self {
        Self {
            base: CargoStrategy::new("cargo-build", "build"),
        }
    }
}

impl FrameworkStrategy for CargoBuildStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec![];

        // Add channel if specified
        if let Some(channel) = &self.base.channel {
            args.push(format!("+{}", channel));
        }

        // Add subcommand
        args.push(self.base.subcommand.clone());

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        // Add default args
        args.extend(self.base.default_args.clone());

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Build
    }
}

/// Bazel build strategy  
pub struct BazelBuildStrategy {
    name: String,
}

impl BazelBuildStrategy {
    pub fn new() -> Self {
        Self {
            name: "bazel-build".to_string(),
        }
    }

    fn find_target(&self, file_path: &str) -> Result<String, String> {
        use crate::bazel::BazelTargetFinder;
        use std::path::Path;

        // Find the BUILD file
        let path = Path::new(file_path);
        let dir = path.parent().ok_or("Invalid file path")?;

        // Look for BUILD.bazel or BUILD file
        let build_file = if dir.join("BUILD.bazel").exists() {
            dir.join("BUILD.bazel")
        } else if dir.join("BUILD").exists() {
            dir.join("BUILD")
        } else {
            // Walk up to find BUILD file
            let mut current = dir;
            loop {
                if current.join("BUILD.bazel").exists() {
                    break current.join("BUILD.bazel");
                } else if current.join("BUILD").exists() {
                    break current.join("BUILD");
                }
                current = current.parent().ok_or("No BUILD file found")?;
            }
        };

        // Parse BUILD file to find any target that uses this source file
        let mut finder = BazelTargetFinder::new()
            .map_err(|e| format!("Failed to create target finder: {}", e))?;
        let targets = finder
            .find_targets_in_build_file(&build_file)
            .map_err(|e| format!("Failed to parse BUILD file: {}", e))?;

        // Find any target that uses this source file (library, binary, etc)
        for target in targets {
            if target.sources.iter().any(|src| file_path.ends_with(src)) {
                return Ok(target.label);
            }
        }

        Err(format!("No build target found for {}", file_path))
    }
}

impl FrameworkStrategy for BazelBuildStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["build".to_string()];

        // Find the build target for this file
        if let Some(file_path) = &context.file_path {
            match self.find_target(file_path) {
                Ok(target) => args.push(target),
                Err(e) => return Err(e),
            }
        }

        let mut command = CargoCommand::new_bazel(args);

        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Build
    }
}

/// Leptos watch strategy for development
pub struct LeptosWatchStrategy {
    name: String,
}

impl LeptosWatchStrategy {
    pub fn new() -> Self {
        Self {
            name: "leptos-watch".to_string(),
        }
    }
}

impl FrameworkStrategy for LeptosWatchStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["leptos".to_string(), "watch".to_string()];

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary // Leptos watch is for running binaries
    }
}

/// Dioxus serve strategy for development
pub struct DioxusServeStrategy {
    name: String,
}

impl DioxusServeStrategy {
    pub fn new() -> Self {
        Self {
            name: "dx-serve".to_string(),
        }
    }
}

impl FrameworkStrategy for DioxusServeStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        // Dioxus uses 'dx' command directly, not through cargo
        let args = vec!["serve".to_string()];

        // Add platform if needed
        // args.push("--platform".to_string());
        // args.push("web".to_string());

        let mut command = CargoCommand::new_shell("dx".to_string(), args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary // Dioxus serve is for running binaries
    }
}

/// Trunk serve strategy for WASM development
pub struct TrunkServeStrategy {
    name: String,
}

impl TrunkServeStrategy {
    pub fn new() -> Self {
        Self {
            name: "trunk-serve".to_string(),
        }
    }
}

impl FrameworkStrategy for TrunkServeStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        // Trunk is a standalone tool
        let mut args = vec!["serve".to_string()];

        // Add common trunk options
        args.push("--open".to_string());

        let mut command = CargoCommand::new_shell("trunk".to_string(), args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Cargo Tauri strategy
pub struct CargoTauriStrategy {
    name: String,
}

impl CargoTauriStrategy {
    pub fn new() -> Self {
        Self {
            name: "cargo-tauri".to_string(),
        }
    }
}

impl FrameworkStrategy for CargoTauriStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let args = vec!["tauri".to_string(), "dev".to_string()];

        // No package needed for tauri

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Cargo Leptos strategy
pub struct CargoLeptosStrategy {
    name: String,
}

impl CargoLeptosStrategy {
    pub fn new() -> Self {
        Self {
            name: "cargo-leptos".to_string(),
        }
    }
}

impl FrameworkStrategy for CargoLeptosStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let mut args = vec!["leptos".to_string(), "watch".to_string()];

        // Add package if specified
        if let Some(package) = &context.package_name {
            args.push("--package".into());
            args.push(package.clone());
        }

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Cargo Shuttle strategy
pub struct CargoShuttleStrategy {
    name: String,
}

impl CargoShuttleStrategy {
    pub fn new() -> Self {
        Self {
            name: "cargo-shuttle".to_string(),
        }
    }
}

impl FrameworkStrategy for CargoShuttleStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let args = vec!["shuttle".to_string(), "run".to_string()];

        let mut command = CargoCommand::new(args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Dioxus CLI dx serve strategy
pub struct DxServeStrategy {
    name: String,
}

impl DxServeStrategy {
    pub fn new() -> Self {
        Self {
            name: "dx-serve".to_string(),
        }
    }
}

impl FrameworkStrategy for DxServeStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let args = vec!["serve".to_string()];

        let mut command = CargoCommand::new_shell("dx".to_string(), args);

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Rustc run strategy for standalone files
pub struct RustcRunStrategy {
    name: String,
}

impl RustcRunStrategy {
    pub fn new() -> Self {
        Self {
            name: "rustc-run".to_string(),
        }
    }
}

impl FrameworkStrategy for RustcRunStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let file_path = context.file_path.as_ref()
            .ok_or_else(|| "No file path provided for rustc-run".to_string())?;
        
        // Extract the file stem for the output binary name
        let file_stem = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file path".to_string())?;
        
        // Create a temp directory for the binary
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join(file_stem);
        
        // Build rustc compile command
        let mut compile_args = vec![
            file_path.clone(),
            "-o".to_string(),
            output_path.to_string_lossy().to_string(),
        ];
        
        // Add optimization in release mode (could be configurable)
        compile_args.push("-O".to_string());
        
        // Create compile command
        let compile_cmd = format!("rustc {} && {}", 
            compile_args.join(" "),
            output_path.to_string_lossy()
        );
        
        // Use shell command to compile and run
        let mut command = CargoCommand::new_shell("sh".to_string(), vec![
            "-c".to_string(),
            compile_cmd,
        ]);
        
        // Set command type to Shell
        command.command_type = crate::command::CommandType::Shell;

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Rustc test strategy for standalone files with tests
pub struct RustcTestStrategy {
    name: String,
}

impl RustcTestStrategy {
    pub fn new() -> Self {
        Self {
            name: "rustc-test".to_string(),
        }
    }
}

impl FrameworkStrategy for RustcTestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let file_path = context.file_path.as_ref()
            .ok_or_else(|| "No file path provided for rustc-test".to_string())?;
        
        // Extract the file stem for the output binary name
        let file_stem = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid file path".to_string())?;
        
        // Create a temp directory for the test binary
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join(format!("{}_test", file_stem));
        
        // Build rustc compile command with test flag
        let compile_args = vec![
            "--test".to_string(),
            file_path.clone(),
            "-o".to_string(),
            output_path.to_string_lossy().to_string(),
        ];
        
        // Create compile and run command
        let mut test_args = vec![];
        
        // Add test filter if specific test is requested
        if let RunnableKind::Test { test_name, .. } = &context.runnable_kind {
            if let Some(module_path) = &context.module_path {
                test_args.push(format!("{}::{}", module_path, test_name));
            } else {
                test_args.push(test_name.clone());
            }
            test_args.push("--exact".to_string());
        }
        
        let compile_cmd = if test_args.is_empty() {
            format!("rustc {} && {}", 
                compile_args.join(" "),
                output_path.to_string_lossy()
            )
        } else {
            format!("rustc {} && {} {}", 
                compile_args.join(" "),
                output_path.to_string_lossy(),
                test_args.join(" ")
            )
        };
        
        // Use shell command to compile and run tests
        let mut command = CargoCommand::new_shell("sh".to_string(), vec![
            "-c".to_string(),
            compile_cmd,
        ]);
        
        // Set command type to Shell
        command.command_type = crate::command::CommandType::Shell;

        // Set working directory if specified
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }

        Ok(command)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_test_strategy() {
        let strategy = CargoTestStrategy::new();
        let context = CommandContext {
            file_path: Some("src/lib.rs".into()),
            crate_name: Some("my-crate".into()),
            package_name: Some("my-crate".into()),
            module_path: Some("tests".into()),
            function_name: Some("test_something".into()),
            runnable_kind: RunnableKind::Test {
                test_name: "test_something".into(),
                is_async: false,
            },
            working_dir: Some("/project".into()),
        };

        let command = strategy.build_command(&context).unwrap();
        assert_eq!(command.command_type, crate::command::CommandType::Cargo);
        assert!(command.args.contains(&"test".into()));
        assert!(command.args.contains(&"--package".into()));
        assert!(command.args.contains(&"my-crate".into()));
        assert!(command.args.contains(&"tests::test_something".into()));
        assert!(command.args.contains(&"--exact".into()));
    }

    #[test]
    fn test_cargo_nextest_strategy() {
        let strategy = CargoNextestStrategy::new();
        let context = CommandContext {
            file_path: Some("src/lib.rs".into()),
            crate_name: Some("my-crate".into()),
            package_name: Some("my-crate".into()),
            module_path: Some("tests".into()),
            function_name: Some("test_something".into()),
            runnable_kind: RunnableKind::Test {
                test_name: "test_something".into(),
                is_async: false,
            },
            working_dir: None,
        };

        let command = strategy.build_command(&context).unwrap();
        assert_eq!(command.command_type, crate::command::CommandType::Cargo);
        assert!(command.args.contains(&"nextest".into()));
        assert!(command.args.contains(&"run".into()));
        assert!(command.args.contains(&"tests::test_something".into()));
    }
}

/// Strategy for running single-file Rust scripts with cargo script
pub struct CargoScriptRunStrategy {
    name: String,
}

impl CargoScriptRunStrategy {
    pub fn new() -> Self {
        Self {
            name: "cargo-script-run".to_string(),
        }
    }
}

impl FrameworkStrategy for CargoScriptRunStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }

    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let file_path = context.file_path.as_ref()
            .ok_or_else(|| "No file path provided".to_string())?;

        // Use cargo +nightly with -Z script flag
        let mut cmd = CargoCommand::new(vec![
            "+nightly".to_string(),
            "-Zscript".to_string(),
            file_path.to_string(),
        ]);
        
        // Set working directory if provided
        if let Some(ref working_dir) = context.working_dir {
            cmd.working_dir = Some(working_dir.clone());
        }

        Ok(cmd)
    }
}

/// Strategy for testing single-file Rust scripts with cargo script
pub struct CargoScriptTestStrategy {
    name: String,
}

impl CargoScriptTestStrategy {
    pub fn new() -> Self {
        Self {
            name: "cargo-script-test".to_string(),
        }
    }
}

impl FrameworkStrategy for CargoScriptTestStrategy {
    fn name(&self) -> &str {
        &self.name
    }

    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }

    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        let file_path = context.file_path.as_ref()
            .ok_or_else(|| "No file path provided".to_string())?;

        // Build args for cargo script with test flag
        let mut args = vec![
            "+nightly".to_string(),
            "-Zscript".to_string(),
            "--test".to_string(),
            file_path.to_string(),
            "--".to_string(),
        ];
        
        // Add test filter if specific test is requested
        if let RunnableKind::Test { test_name, .. } = &context.runnable_kind {
            // Add module path if present
            if let Some(ref module_path) = context.module_path {
                if !module_path.is_empty() {
                    args.push(format!("{}::{}", module_path, test_name));
                } else {
                    args.push(test_name.clone());
                }
            } else {
                args.push(test_name.clone());
            }
            args.push("--exact".to_string());
        } else if let RunnableKind::ModuleTests { module_name } = &context.runnable_kind {
            args.push(module_name.clone());
        }
        
        let mut cmd = CargoCommand::new(args);
        
        // Set working directory if provided
        if let Some(ref working_dir) = context.working_dir {
            cmd.working_dir = Some(working_dir.clone());
        }

        Ok(cmd)
    }
}
