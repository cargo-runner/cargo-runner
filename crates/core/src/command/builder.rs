//! Clean API for command building with encapsulated config resolution

use crate::{
    command::CargoCommand,
    config::{Config, ConfigMerger, Features},
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
    
    /// Check if a file is a cargo script file
    fn is_cargo_script_file(&self, file_path: &Path) -> Result<bool> {
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Some(first_line) = content.lines().next() {
                    return Ok(first_line.starts_with("#!") && first_line.contains("cargo") && first_line.contains("-Zscript"));
                }
            }
        }
        Ok(false)
    }
    
    /// Check if a file is a standalone file (has main() and not part of a Cargo project)
    fn is_standalone_file(&self, file_path: &Path) -> bool {
        // First check if file has a main function
        let has_main = if let Ok(content) = std::fs::read_to_string(file_path) {
            content.contains("fn main(") || content.contains("fn main (")
        } else {
            return false; // Can't read file, not standalone
        };
        
        if !has_main {
            return false; // No main function, not standalone
        }
        
        // Check if file is part of a Cargo project
        let cargo_root = file_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists());
        
        match cargo_root {
            None => true, // No Cargo.toml found, definitely standalone
            Some(root) => {
                // Check if the file is in a standard Cargo source location
                if let Ok(relative) = file_path.strip_prefix(root) {
                    let path_str = relative.to_str().unwrap_or("");
                    
                    // Check standard binary locations
                    if path_str == "src/main.rs" || 
                       path_str.starts_with("src/bin/") ||
                       path_str.starts_with("examples/") {
                        return false; // In standard location, not standalone
                    }
                    
                    // Check if it's listed in Cargo.toml as a [[bin]]
                    let cargo_toml_path = root.join("Cargo.toml");
                    if cargo_toml_path.exists() {
                        // Try to read and parse Cargo.toml
                        if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                            // Simple check for [[bin]] entries with this path
                            // This is a simplified check - could use toml parsing for accuracy
                            if content.contains(&format!("path = \"{}\"", path_str)) ||
                               content.contains(&format!("path = '{}'", path_str)) {
                                return false; // Listed in Cargo.toml, not standalone
                            }
                        }
                    }
                    
                    // Has main(), not in standard location, not in Cargo.toml = standalone
                    true
                } else {
                    true // Shouldn't happen, but treat as standalone if strip_prefix fails
                }
            }
        }
    }
    
    /// Build the command
    pub fn build(self) -> Result<CargoCommand> {
        // Check file type for special handling
        let is_cargo_script = self.is_cargo_script_file(&self.runnable.file_path)?;
        let is_standalone = self.is_standalone_file(&self.runnable.file_path);
        
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
            RunnableKind::Binary { .. } => {
                if is_standalone {
                    Box::new(RustcCommandBuilder)
                } else {
                    Box::new(BinaryCommandBuilder)
                }
            },
            RunnableKind::DocTest { .. } => Box::new(DocTestCommandBuilder),
            RunnableKind::Test { .. } => {
                if is_cargo_script {
                    Box::new(SingleFileScriptBuilder)
                } else if is_standalone {
                    Box::new(RustcCommandBuilder)
                } else {
                    Box::new(TestCommandBuilder)
                }
            },
            RunnableKind::ModuleTests { .. } => {
                if is_standalone {
                    Box::new(RustcCommandBuilder)
                } else {
                    Box::new(ModuleTestCommandBuilder)
                }
            },
            RunnableKind::Benchmark { .. } => Box::new(BenchmarkCommandBuilder),
            RunnableKind::Standalone { .. } => Box::new(RustcCommandBuilder),
            RunnableKind::SingleFileScript { .. } => Box::new(SingleFileScriptBuilder),
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
        if let Some(extra_env) = &config.extra_env {
            for (key, value) in extra_env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
    
    /// Apply features from configuration
    fn apply_features(&self, args: &mut Vec<String>, runnable: &Runnable, config: &Config) {
        // Collect features from different levels
        let mut collected_features: Option<Features> = None;
        
        // Base features from config
        collected_features = Features::merge(collected_features.as_ref(), config.features.as_ref());
        
        // Override features
        if let Some(override_config) = self.get_override(runnable, config) {
            if override_config.force_replace_features.unwrap_or(false) {
                // Replace features entirely
                collected_features = override_config.features.clone();
            } else {
                // Merge features
                collected_features = Features::merge(collected_features.as_ref(), override_config.features.as_ref());
            }
        }
        
        // Add feature args to command
        if let Some(features) = collected_features {
            args.extend(features.to_args());
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
        // Apply features first (so they come before other args)
        self.apply_features(args, runnable, config);
        
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
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
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
            
            // Add framework features
            if let Some(features) = &test_framework.features {
                args.extend(features.to_args());
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
            if let Some(extra_env) = &test_framework.extra_env {
                for (key, value) in extra_env {
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
        // Apply features first (so they come before other args)
        self.apply_features(args, runnable, config);
        
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
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
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
            
            // Add framework features
            if let Some(features) = &binary_framework.features {
                args.extend(features.to_args());
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
        // Create the appropriate command type
        let mut command = if let Some(binary_framework) = &config.binary_framework {
            if let Some(cmd) = &binary_framework.command {
                if cmd != "cargo" {
                    // Use Shell command for non-cargo commands
                    CargoCommand::new_shell(cmd.clone(), args)
                } else {
                    CargoCommand::new(args)
                }
            } else {
                CargoCommand::new(args)
            }
        } else {
            CargoCommand::new(args)
        };
        
        // Apply binary framework env
        if let Some(binary_framework) = &config.binary_framework {
            if let Some(extra_env) = &binary_framework.extra_env {
                for (key, value) in extra_env {
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
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
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
            if let Some(extra_env) = &test_framework.extra_env {
                for (key, value) in extra_env {
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
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
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
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

// Rustc command builder for standalone files
struct RustcCommandBuilder;

impl CommandBuilderImpl for RustcCommandBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        _package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        let file_name = runnable.file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| crate::Error::ParseError("Invalid file name".to_string()))?;
        
        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                // Run specific test with rustc --test
                let output_name = format!("{}_test", file_name);
                let mut args = vec![
                    "--test".to_string(),
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "-o".to_string(),
                    output_name,
                ];
                
                // Apply features
                self.apply_features(&mut args, runnable, config);
                
                // Apply extra args
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                if let Some(extra_args) = &config.extra_args {
                    args.extend(extra_args.clone());
                }
                
                // Create a rustc command with test filter
                let mut command = CargoCommand::new_rustc(args)
                    .with_test_filter(test_name.clone());
                
                // Apply env vars
                self.apply_common_config(&mut command, config);
                self.apply_env(&mut command, runnable, config);
                
                Ok(command)
            }
            RunnableKind::ModuleTests { .. } => {
                // Run all tests in module with rustc --test
                let mut args = vec![
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "--test".to_string(),
                    "-o".to_string(),
                    format!("{}_test", file_name),
                ];
                
                // Apply features
                self.apply_features(&mut args, runnable, config);
                
                // Apply extra args
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                if let Some(extra_args) = &config.extra_args {
                    args.extend(extra_args.clone());
                }
                
                let mut command = CargoCommand::new_rustc(args);
                
                // Apply env vars
                self.apply_common_config(&mut command, config);
                self.apply_env(&mut command, runnable, config);
                
                Ok(command)
            }
            RunnableKind::Binary { .. } | RunnableKind::Standalone { .. } => {
                // Run main binary
                let mut args = vec![
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "-o".to_string(),
                    file_name.to_string(),
                ];
                
                // Apply features
                self.apply_features(&mut args, runnable, config);
                
                // Apply extra args
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                if let Some(extra_args) = &config.extra_args {
                    args.extend(extra_args.clone());
                }
                
                let mut command = CargoCommand::new_rustc(args);
                
                // Apply env vars
                self.apply_common_config(&mut command, config);
                self.apply_env(&mut command, runnable, config);
                
                Ok(command)
            }
            _ => Err(crate::Error::ParseError("Unsupported runnable type for rustc".to_string()))
        }
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        FunctionIdentity {
            package: config.package.clone(),
            module_path: None,
            file_path: Some(runnable.file_path.clone()),
            function_name: None,
        }
    }
}

impl RustcCommandBuilder {
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
}

// Single file script builder for cargo script files
struct SingleFileScriptBuilder;

impl CommandBuilderImpl for SingleFileScriptBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        _package_name: Option<&str>,
        config: &Config,
    ) -> Result<CargoCommand> {
        match &runnable.kind {
            RunnableKind::SingleFileScript { shebang } => {
                // Build command for running the script
                let mut args = self.parse_shebang_args(shebang);
                
                // Add the script file path
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());
                
                // Apply features
                self.apply_features(&mut args, runnable, config);
                
                // Apply extra args
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                if let Some(extra_args) = &config.extra_args {
                    args.extend(extra_args.clone());
                }
                
                let mut command = CargoCommand::new(args);
                
                // Apply env vars
                self.apply_common_config(&mut command, config);
                self.apply_env(&mut command, runnable, config);
                
                Ok(command)
            }
            RunnableKind::Test { test_name, .. } => {
                // Build command for running a test in a cargo script
                let mut args = Vec::new();
                
                // Check for test framework config
                if let Some(test_framework) = &config.test_framework {
                    // Use test framework channel if specified
                    if let Some(channel) = &test_framework.channel {
                        args.push(format!("+{}", channel));
                    } else {
                        args.push("+nightly".to_string());
                    }
                } else {
                    args.push("+nightly".to_string());
                }
                
                args.push("-Zscript".to_string());
                args.push("test".to_string());
                args.push("--manifest-path".to_string());
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());
                
                // Apply test framework args if configured
                if let Some(test_framework) = &config.test_framework {
                    if let Some(extra_args) = &test_framework.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                // Apply features
                self.apply_features(&mut args, runnable, config);
                
                // Apply extra args from overrides
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                // Apply global extra args
                if let Some(extra_args) = &config.extra_args {
                    args.extend(extra_args.clone());
                }
                
                // Add test filter
                args.push("--".to_string());
                args.push(test_name.clone());
                
                // Test framework doesn't have extra_test_binary_args, only extra_args
                
                if let Some(override_config) = self.get_override(runnable, config) {
                    if let Some(extra_args) = &override_config.extra_test_binary_args {
                        args.extend(extra_args.clone());
                    }
                }
                
                if let Some(extra_args) = &config.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
                
                let mut command = CargoCommand::new_single_file_script_test(args);
                
                // Apply test framework env
                if let Some(test_framework) = &config.test_framework {
                    if let Some(extra_env) = &test_framework.extra_env {
                        for (key, value) in extra_env {
                            command.env.push((key.clone(), value.clone()));
                        }
                    }
                }
                
                self.apply_common_config(&mut command, config);
                self.apply_env(&mut command, runnable, config);
                
                Ok(command)
            }
            _ => Err(crate::Error::ParseError("Not a single file script or test".to_string()))
        }
    }
    
    fn create_identity(&self, runnable: &Runnable, config: &Config) -> FunctionIdentity {
        FunctionIdentity {
            package: config.package.clone(),
            module_path: None,
            file_path: Some(runnable.file_path.clone()),
            function_name: None,
        }
    }
}

impl SingleFileScriptBuilder {
    fn apply_env(&self, command: &mut CargoCommand, runnable: &Runnable, config: &Config) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config) {
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
    
    fn parse_shebang_args(&self, shebang: &str) -> Vec<String> {
        let mut args = Vec::new();
        
        // Default to cargo +nightly -Zscript if we can't parse the shebang
        if shebang.contains("+nightly") && shebang.contains("-Zscript") {
            args.push("+nightly".to_string());
            args.push("-Zscript".to_string());
        } else {
            // Try to extract the channel and args from shebang
            let parts: Vec<&str> = shebang.split_whitespace().collect();
            let cargo_idx = parts.iter().position(|&p| p == "cargo");
            
            if let Some(idx) = cargo_idx {
                // Add everything after "cargo"
                for i in (idx + 1)..parts.len() {
                    args.push(parts[i].to_string());
                }
            } else {
                // Fallback to default
                args.push("+nightly".to_string());
                args.push("-Zscript".to_string());
            }
        }
        
        args
    }
}