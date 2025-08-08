//! Clean API for command building with encapsulated config resolution

use crate::{
    command::CargoCommand,
    config::{Config, ConfigMerger},
    error::Result,
    types::{FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;
use tracing::debug;

/// Main entry point for building commands
/// 
/// # Example
/// ```
/// let runnable = /* ... */;
/// let command = CommandBuilder::for_runnable(&runnable)
///     .with_package("my-package")
///     .with_project_root("/path/to/project")
///     .build()?;
/// ```
pub struct CommandBuilder<'a> {
    runnable: &'a Runnable,
    package_name: Option<String>,
    project_root: Option<&'a Path>,
    config_override: Option<Config>,
}

impl<'a> CommandBuilder<'a> {
    /// Create a new command builder for a runnable
    pub fn for_runnable(runnable: &'a Runnable) -> Self {
        Self {
            runnable,
            package_name: None,
            project_root: None,
            config_override: None,
        }
    }
    
    /// Set the package name
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package_name = Some(package.into());
        self
    }
    
    /// Set the project root
    pub fn with_project_root(mut self, root: &'a Path) -> Self {
        self.project_root = Some(root);
        self
    }
    
    /// Override the configuration (for testing or special cases)
    pub fn with_config(mut self, config: Config) -> Self {
        self.config_override = Some(config);
        self
    }
    
    /// Build the command
    pub fn build(self) -> Result<CargoCommand> {
        // Resolve configuration
        let config = if let Some(config) = self.config_override {
            config
        } else {
            ConfigResolver::new()
                .for_runnable(self.runnable)
                .with_project_root(self.project_root)
                .resolve()?
        };
        
        // Select appropriate builder based on runnable type
        let builder: Box<dyn CommandBuilderImpl> = match &self.runnable.kind {
            RunnableKind::Binary { .. } => Box::new(BinaryCommandBuilder),
            RunnableKind::DocTest { .. } => Box::new(DocTestCommandBuilder),
            RunnableKind::Test { .. } => Box::new(TestCommandBuilder),
            RunnableKind::Benchmark { .. } => Box::new(BenchmarkCommandBuilder),
            RunnableKind::ModuleTests { .. } => Box::new(ModuleTestCommandBuilder),
        };
        
        // Build the command
        builder.build(
            self.runnable,
            self.package_name.as_deref(),
            &config,
        )
    }
}

/// Configuration resolver with clean API
struct ConfigResolver<'a> {
    runnable: Option<&'a Runnable>,
    project_root: Option<&'a Path>,
}

impl<'a> ConfigResolver<'a> {
    fn new() -> Self {
        Self {
            runnable: None,
            project_root: None,
        }
    }
    
    fn for_runnable(mut self, runnable: &'a Runnable) -> Self {
        self.runnable = Some(runnable);
        self
    }
    
    fn with_project_root(mut self, root: Option<&'a Path>) -> Self {
        self.project_root = root;
        self
    }
    
    fn resolve(self) -> Result<Config> {
        let mut merger = ConfigMerger::new();
        
        // Load configs based on the runnable's location
        if let Some(runnable) = self.runnable {
            merger.load_configs_for_path(&runnable.file_path)?;
        } else if let Some(root) = self.project_root {
            merger.load_configs_for_path(root)?;
        } else {
            // Load from current directory
            if let Ok(cwd) = std::env::current_dir() {
                merger.load_configs_for_path(&cwd)?;
            }
        }
        
        Ok(merger.get_merged_config())
    }
}

/// Internal trait for command builders
trait CommandBuilderImpl {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand>;
    
    /// Get override configuration for the runnable
    fn get_override<'a>(&self, runnable: &Runnable, config: &'a Config) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config);
        config.get_override_for(&identity)
    }
    
    /// Create function identity for override matching
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity;
    
    /// Apply common configuration
    fn apply_common_config(&self, command: &mut CargoCommand, config: &Config) {
        if let Some(env) = &config.env {
            for (key, value) in env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
}

/// Doc test command builder
struct DocTestCommandBuilder;

impl CommandBuilderImpl for DocTestCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let mut args = vec!["test".to_string(), "--doc".to_string()];
        
        // Add package if specified
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
        
        // Apply configuration
        self.apply_args(&mut args, runnable, config);
        
        // Add doc test filter
        if let RunnableKind::DocTest { struct_or_module_name, method_name } = &runnable.kind {
            args.push("--".to_string());
            
            let test_path = if let Some(method) = method_name {
                format!("{}::{}", struct_or_module_name, method)
            } else {
                struct_or_module_name.clone()
            };
            args.push(test_path);
            
            // Apply test binary args
            self.apply_test_binary_args(&mut args, runnable, config);
        }
        
        let mut command = CargoCommand::new(args);
        self.apply_common_config(&mut command, config);
        self.apply_env(&mut command, runnable, config);
        
        Ok(command)
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        if let RunnableKind::DocTest { struct_or_module_name, method_name } = &runnable.kind {
            let function_name = if let Some(method) = method_name {
                Some(format!("{}::{}", struct_or_module_name, method))
            } else {
                Some(struct_or_module_name.clone())
            };
            
            FunctionIdentity {
                package: config.package.clone(),
                module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
                file_path: Some(runnable.file_path.clone()),
                function_name,
            }
        } else {
            FunctionIdentity::default()
        }
    }
}

impl DocTestCommandBuilder {
    fn apply_args(&self, args: &mut Vec<String>, runnable: &Runnable, config: &Config) {
        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_args) = &override_config.extra_args {
                args.extend(extra_args.clone());
            }
        }
        
        // Apply global args
        if let Some(extra_args) = &config.extra_args {
            args.extend(extra_args.clone());
        }
    }
    
    fn apply_test_binary_args(&self, args: &mut Vec<String>, runnable: &Runnable, config: &Config) {
        // Apply override test binary args
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_args) = &override_config.extra_test_binary_args {
                args.extend(extra_args.clone());
            }
        }
        
        // Apply global test binary args
        if let Some(extra_args) = &config.extra_test_binary_args {
            args.extend(extra_args.clone());
        }
    }
    
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

/// Test command builder with test framework support
struct TestCommandBuilder;

impl CommandBuilderImpl for TestCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let mut args = vec![];
        
        // Handle test framework configuration
        if let Some(test_framework) = &config.test_framework {
            // Add channel
            if let Some(channel) = &test_framework.channel {
                args.push(format!("+{}", channel));
            } else if let Some(channel) = &config.channel {
                args.push(format!("+{}", channel));
            }
            
            // Add subcommand
            if let Some(subcommand) = &test_framework.subcommand {
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                args.push("test".to_string());
            }
            
            // Add framework args
            if let Some(framework_args) = &test_framework.extra_args {
                args.extend(framework_args.clone());
            }
        } else {
            // Standard test command
            if let Some(channel) = &config.channel {
                args.push(format!("+{}", channel));
            }
            args.push("test".to_string());
        }
        
        // Add package
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
        
        // Add target
        self.add_target(&mut args, &runnable.file_path, package_name)?;
        
        // Apply configuration
        self.apply_args(&mut args, runnable, config);
        
        // Add test filter
        self.add_test_filter(&mut args, runnable, config);
        
        let mut command = CargoCommand::new(args);
        
        // Apply test framework env
        if let Some(test_framework) = &config.test_framework {
            if let Some(env) = &test_framework.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
        
        self.apply_common_config(&mut command, config);
        self.apply_env(&mut command, runnable, config);
        
        Ok(command)
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        FunctionIdentity {
            package: config.package.clone(),
            module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
                _ => None,
            },
        }
    }
}

impl TestCommandBuilder {
    fn add_target(&self, args: &mut Vec<String>, file_path: &Path, package_name: Option<&str>) -> Result<()> {
        use crate::command::Target;
        
        if let Some(target) = Target::from_file_path(file_path) {
            match target {
                Target::Lib => args.push("--lib".to_string()),
                Target::Bin(name) => {
                    args.push("--bin".to_string());
                    // Special case: if binary name is "main" and we have a package name,
                    // use the package name as the binary name (Rust convention for src/main.rs)
                    if name == "main" && package_name.is_some() {
                        args.push(package_name.unwrap().to_string());
                    } else {
                        args.push(name);
                    }
                }
                Target::Test(name) => {
                    args.push("--test".to_string());
                    args.push(name);
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    fn apply_args(&self, args: &mut Vec<String>, runnable: &Runnable, config: &Config) {
        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_args) = &override_config.extra_args {
                args.extend(extra_args.clone());
            }
        }
        
        // Apply global args
        if let Some(extra_args) = &config.extra_args {
            args.extend(extra_args.clone());
        }
    }
    
    fn add_test_filter(&self, args: &mut Vec<String>, runnable: &Runnable, config: &Config) {
        if let RunnableKind::Test { test_name, .. } = &runnable.kind {
            args.push("--".to_string());
            
            let test_path = if runnable.module_path.is_empty() {
                test_name.clone()
            } else {
                format!("{}::{}", runnable.module_path, test_name)
            };
            args.push(test_path);
            args.push("--exact".to_string());
            
            // Apply test binary args
            if let Some(override_config) = self.get_override(runnable, config) {
                if let Some(extra_args) = &override_config.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
            
            if let Some(extra_args) = &config.extra_test_binary_args {
                args.extend(extra_args.clone());
            }
        }
    }
    
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

/// Binary command builder with binary_framework support
struct BinaryCommandBuilder;

impl CommandBuilderImpl for BinaryCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let mut args = vec![];
        
        debug!("BinaryCommandBuilder: config.binary_framework = {:?}", config.binary_framework);
        debug!("BinaryCommandBuilder: runnable.module_path = '{}', config.package = {:?}", runnable.module_path, config.package);
        
        // Check for binary framework configuration
        if let Some(binary_framework) = &config.binary_framework {
            // Check if we're using a custom command
            let is_custom_command = binary_framework.command.as_deref().map_or(false, |cmd| cmd != "cargo");
            
            // Add channel (only for cargo commands)
            if !is_custom_command {
                if let Some(channel) = &binary_framework.channel {
                    args.push(format!("+{}", channel));
                } else if let Some(channel) = &config.channel {
                    args.push(format!("+{}", channel));
                }
            }
            
            // Add subcommand (e.g., "serve" for dx, "leptos watch" for cargo)
            if let Some(subcommand) = &binary_framework.subcommand {
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                args.push("run".to_string());
            }
            
            // Add framework-specific args
            if let Some(extra_args) = &binary_framework.extra_args {
                args.extend(extra_args.clone());
            }
        } else {
            // Standard cargo run
            if let Some(channel) = &config.channel {
                args.push(format!("+{}", channel));
            }
            args.push("run".to_string());
        }
        
        // Determine if we're using standard cargo command
        let is_cargo_command = if let Some(binary_framework) = &config.binary_framework {
            binary_framework.command.as_deref() == Some("cargo") || binary_framework.command.is_none()
        } else {
            true
        };
        
        // Determine if we're using standard run subcommand
        let is_run_subcommand = if let Some(binary_framework) = &config.binary_framework {
            binary_framework.subcommand.as_deref() == Some("run") || binary_framework.subcommand.is_none()
        } else {
            true
        };
        
        // Only add --package if using cargo command
        if is_cargo_command {
            if let Some(pkg) = package_name {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }
        }
        
        // Only add --bin if using cargo run
        if is_cargo_command && is_run_subcommand {
            if let RunnableKind::Binary { bin_name } = &runnable.kind {
                // Check if this is an example file
                let path_str = runnable.file_path.to_str().unwrap_or("");
                let is_example = path_str.contains("/examples/") || path_str.contains("examples/");
                
                if is_example {
                    // Example target
                    args.push("--example".to_string());
                    args.push(bin_name.clone().unwrap_or_else(|| {
                        runnable.file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    }));
                } else {
                    // Regular binary target
                    match bin_name {
                        Some(name) => {
                            // Explicit binary name (from src/bin/foo.rs)
                            args.push("--bin".to_string());
                            args.push(name.clone());
                        }
                        None => {
                            // src/main.rs - use package name if available
                            if let Some(pkg) = package_name {
                                args.push("--bin".to_string());
                                args.push(pkg.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Apply configuration overrides
        debug!("BinaryCommandBuilder checking for overrides");
        if let Some(override_config) = self.get_override(runnable, config) {
            debug!("Found override config: {:?}", override_config);
            if let Some(extra_args) = &override_config.extra_args {
                debug!("Applying extra_args from override: {:?}", extra_args);
                args.extend(extra_args.clone());
            }
        } else {
            debug!("No override found for runnable");
        }
        
        // Apply global extra args
        if let Some(extra_args) = &config.extra_args {
            args.extend(extra_args.clone());
        }
        
        // Create command
        let mut command = CargoCommand::new(args);
        
        // If we have a custom command, we need to handle it differently
        // For now, we'll prepend the custom command to the args
        if let Some(binary_framework) = &config.binary_framework {
            if let Some(cmd) = &binary_framework.command {
                if cmd != "cargo" {
                    // Insert the custom command at the beginning of args
                    // This will make the shell command look like: cargo dx serve ...
                    // But it's actually: dx serve ...
                    command.args.insert(0, cmd.clone());
                }
            }
        }
        
        // Apply binary framework env
        if let Some(binary_framework) = &config.binary_framework {
            if let Some(env) = &binary_framework.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
        
        self.apply_common_config(&mut command, config);
        self.apply_env(&mut command, runnable, config);
        
        Ok(command)
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        let identity = FunctionIdentity {
            package: config.package.clone(),
            module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Binary { bin_name } => bin_name.clone(),
                _ => None,
            },
        };
        debug!("BinaryCommandBuilder creating identity: {:?}", identity);
        identity
    }
}

impl BinaryCommandBuilder {
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

/// Module test command builder (runs all tests in a module)
struct ModuleTestCommandBuilder;

impl CommandBuilderImpl for ModuleTestCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let mut args = vec![];
        
        // Handle test framework configuration (same as TestCommandBuilder)
        if let Some(test_framework) = &config.test_framework {
            // Add channel
            if let Some(channel) = &test_framework.channel {
                args.push(format!("+{}", channel));
            } else if let Some(channel) = &config.channel {
                args.push(format!("+{}", channel));
            }
            
            // Add subcommand
            if let Some(subcommand) = &test_framework.subcommand {
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                args.push("test".to_string());
            }
            
            // Add framework-specific args
            if let Some(extra_args) = &test_framework.extra_args {
                args.extend(extra_args.clone());
            }
        } else {
            // Standard test command
            if let Some(channel) = &config.channel {
                args.push(format!("+{}", channel));
            }
            args.push("test".to_string());
        }
        
        // Add package
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
        
        // Add target (same logic as TestCommandBuilder)
        use crate::command::Target;
        if let Some(target) = Target::from_file_path(&runnable.file_path) {
            match target {
                Target::Lib => args.push("--lib".to_string()),
                Target::Bin(name) => {
                    args.push("--bin".to_string());
                    // Special case: if binary name is "main" and we have a package name,
                    // use the package name as the binary name (Rust convention for src/main.rs)
                    if name == "main" && package_name.is_some() {
                        args.push(package_name.unwrap().to_string());
                    } else {
                        args.push(name);
                    }
                }
                Target::Test(name) => {
                    args.push("--test".to_string());
                    args.push(name);
                }
                _ => {}
            }
        }
        
        // Apply configuration overrides
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_args) = &override_config.extra_args {
                args.extend(extra_args.clone());
            }
        }
        
        // Apply global extra args
        if let Some(extra_args) = &config.extra_args {
            args.extend(extra_args.clone());
        }
        
        // Add module filter
        if let RunnableKind::ModuleTests { module_name } = &runnable.kind {
            args.push("--".to_string());
            args.push(module_name.clone());
            
            // Apply test binary args from override
            if let Some(override_config) = self.get_override(runnable, config) {
                if let Some(extra_args) = &override_config.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
            
            // Apply global test binary args
            if let Some(extra_args) = &config.extra_test_binary_args {
                args.extend(extra_args.clone());
            }
        }
        
        let mut command = CargoCommand::new(args);
        
        // Apply test framework env
        if let Some(test_framework) = &config.test_framework {
            if let Some(env) = &test_framework.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
        
        self.apply_common_config(&mut command, config);
        self.apply_env(&mut command, runnable, config);
        
        Ok(command)
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        FunctionIdentity {
            package: config.package.clone(),
            module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::ModuleTests { module_name } => Some(module_name.clone()),
                _ => None,
            },
        }
    }
}

impl ModuleTestCommandBuilder {
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

// Benchmark command builder
struct BenchmarkCommandBuilder;

impl CommandBuilderImpl for BenchmarkCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let mut args = vec![];
        
        // Add channel
        if let Some(channel) = &config.channel {
            args.push(format!("+{}", channel));
        }
        
        // Add bench subcommand
        args.push("bench".to_string());
        
        // Add package
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
        
        // Add bench target
        if let RunnableKind::Benchmark { bench_name } = &runnable.kind {
            // Check if this is in the benches/ directory or a specific benchmark
            let path_str = runnable.file_path.to_str().unwrap_or("");
            if path_str.contains("/benches/") || path_str.contains("benches/") {
                args.push("--bench".to_string());
                args.push(bench_name.clone());
            }
            
            // Apply configuration overrides
            if let Some(override_config) = self.get_override(runnable, config) {
                if let Some(extra_args) = &override_config.extra_args {
                    args.extend(extra_args.clone());
                }
            }
            
            // Apply global extra args
            if let Some(extra_args) = &config.extra_args {
                args.extend(extra_args.clone());
            }
            
            // Add benchmark filter
            args.push("--".to_string());
            args.push(bench_name.clone());
            
            // Apply test binary args
            if let Some(override_config) = self.get_override(runnable, config) {
                if let Some(extra_args) = &override_config.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
            
            if let Some(extra_args) = &config.extra_test_binary_args {
                args.extend(extra_args.clone());
            }
        }
        
        let mut command = CargoCommand::new(args);
        self.apply_common_config(&mut command, config);
        self.apply_env(&mut command, runnable, config);
        
        Ok(command)
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        FunctionIdentity {
            package: config.package.clone(),
            module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
                _ => None,
            },
        }
    }
}

impl BenchmarkCommandBuilder {
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}