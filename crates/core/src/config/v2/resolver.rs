//! Configuration resolver for command building
//! 
//! Resolves layered configurations to build final commands.

use super::{
    ConfigLayer, LayerConfig, StrategyRegistry,
    scope::ScopeContext,
    strategy::{CommandContext, FrameworkKind},
};
use crate::command::CargoCommand;
use crate::types::{RunnableKind, FileType};
use crate::utils::detect_file_type;

/// Resolves configuration for command building
pub struct ConfigResolver<'a> {
    layers: &'a [ConfigLayer],
    registry: &'a StrategyRegistry,
    linked_projects: &'a Option<Vec<String>>,
}

impl<'a> ConfigResolver<'a> {
    /// Create a new configuration resolver
    pub fn new(layers: &'a [ConfigLayer], registry: &'a StrategyRegistry, linked_projects: &'a Option<Vec<String>>) -> Self {
        tracing::debug!("Creating ConfigResolver with linked_projects: {:?}", linked_projects);
        Self { layers, registry, linked_projects }
    }
    
    /// Resolve a command for the given scope context and runnable kind
    pub fn resolve_command(
        &self,
        scope_context: &ScopeContext,
        runnable_kind: RunnableKind,
    ) -> Result<CargoCommand, String> {
        // Find all matching layers
        let mut matching_layers: Vec<&ConfigLayer> = self.layers
            .iter()
            .filter(|layer| layer.matches(scope_context))
            .collect();
        
        // Sort by specificity (most specific last, so it applies last)
        matching_layers.sort_by_key(|layer| layer.specificity());
        
        // Merge configurations
        let merged_config = self.merge_layers(&matching_layers);
        
        // Determine build system
        if merged_config.build_system.is_none() {
            return Err("No build system specified".to_string());
        }
        
        // Get framework strategy name
        let framework_kind = FrameworkKind::from_runnable_kind(&runnable_kind);
        
        // Check if this is a single-file script and override strategy if needed
        let strategy_name = if let Some(ref file_path) = scope_context.file_path {
            tracing::debug!("Checking file path: {:?}", file_path);
            let file_type = detect_file_type(file_path);
            tracing::debug!("File type detection for {:?}: {:?}", file_path, file_type);
            
            if file_type == FileType::SingleFileScript {
                // Override with cargo-script strategies for single-file scripts
                match framework_kind {
                    FrameworkKind::Test => {
                        tracing::debug!("Using cargo-script-test strategy for single-file script");
                        Some("cargo-script-test".to_string())
                    },
                    FrameworkKind::Binary => {
                        tracing::debug!("Using cargo-script-run strategy for single-file script");
                        Some("cargo-script-run".to_string())
                    },
                    _ => self.get_strategy_name(&merged_config, framework_kind),
                }
            } else {
                self.get_strategy_name(&merged_config, framework_kind)
            }
        } else {
            self.get_strategy_name(&merged_config, framework_kind)
        }
        .ok_or_else(|| format!("No framework strategy for {:?}", framework_kind))?;
        
        // Get strategy from registry
        let strategy = self.registry.get(&strategy_name)
            .ok_or_else(|| format!("Unknown strategy: {}", strategy_name))?;
        
        // Determine working directory from linked_projects
        let working_dir = if let Some(file_path) = &scope_context.file_path {
            let wd = self.find_working_dir_from_linked_projects(file_path);
            tracing::debug!("Working directory from linked_projects: {:?}", wd);
            wd
        } else {
            None
        };
        
        // Build command context
        let command_context = CommandContext {
            file_path: scope_context.file_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            crate_name: scope_context.crate_name.clone(),
            package_name: scope_context.crate_name.clone(), // For now, same as crate
            module_path: scope_context.module_path.clone(),
            function_name: scope_context.function_name.clone(),
            runnable_kind,
            working_dir,
        };
        
        // Build base command using strategy
        let mut command = strategy.build_command(&command_context)?;
        
        // Apply additional configuration
        self.apply_args(&mut command, &merged_config, framework_kind);
        self.apply_env(&mut command, &merged_config);
        
        Ok(command)
    }
    
    /// Merge configuration layers
    fn merge_layers(&self, layers: &[&ConfigLayer]) -> LayerConfig {
        let mut merged = LayerConfig::new();
        
        for layer in layers {
            merged.apply(&layer.config);
        }
        
        merged
    }
    
    /// Get the strategy name for a framework kind
    fn get_strategy_name(&self, config: &LayerConfig, kind: FrameworkKind) -> Option<String> {
        match kind {
            FrameworkKind::Test => config.frameworks.test.clone(),
            FrameworkKind::Binary => config.frameworks.binary.clone(),
            FrameworkKind::Benchmark => config.frameworks.benchmark.clone(),
            FrameworkKind::DocTest => config.frameworks.doctest.clone(),
            FrameworkKind::Build => config.frameworks.build.clone(),
        }
    }
    
    /// Apply additional arguments to the command
    fn apply_args(&self, command: &mut CargoCommand, config: &LayerConfig, kind: FrameworkKind) {
        let mut args_to_add = Vec::new();
        
        // Add "all" args first
        if let Some(args) = &config.args.all {
            args_to_add.extend(args.clone());
        }
        
        // Add framework-specific args
        match kind {
            FrameworkKind::Test => {
                if let Some(args) = &config.args.test {
                    args_to_add.extend(args.clone());
                }
            }
            FrameworkKind::Binary => {
                if let Some(args) = &config.args.binary {
                    args_to_add.extend(args.clone());
                }
            }
            FrameworkKind::Benchmark => {
                if let Some(args) = &config.args.benchmark {
                    args_to_add.extend(args.clone());
                }
            }
            FrameworkKind::Build => {
                if let Some(args) = &config.args.build {
                    args_to_add.extend(args.clone());
                }
            }
            _ => {}
        }
        
        // Insert args before any existing "--" separator
        if let Some(separator_pos) = command.args.iter().position(|arg| arg == "--") {
            for (i, arg) in args_to_add.into_iter().enumerate() {
                command.args.insert(separator_pos + i, arg);
            }
        } else {
            command.args.extend(args_to_add);
        }
        
        // Add test binary args if applicable
        if matches!(kind, FrameworkKind::Test) {
            if let Some(args) = &config.args.test_binary {
                // Ensure we have a "--" separator
                if !command.args.contains(&"--".to_string()) {
                    command.args.push("--".to_string());
                }
                command.args.extend(args.clone());
            }
        }
    }
    
    /// Apply environment variables to the command
    fn apply_env(&self, command: &mut CargoCommand, config: &LayerConfig) {
        for (key, value) in &config.env.vars {
            command.env.push((key.clone(), value.clone()));
        }
    }
    
    /// Find working directory from linked projects
    fn find_working_dir_from_linked_projects(
        &self,
        file_path: &std::path::Path,
    ) -> Option<String> {
        // Check if we have linked_projects from the root config
        let linked_projects = self.linked_projects.as_ref()?;
        tracing::debug!("Checking linked_projects: {:?}", linked_projects);
        
        // Get absolute file path
        let abs_file_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir().ok()?.join(file_path)
        };
        
        // Try to find which linked project contains this file
        for project_path in linked_projects {
            let cargo_toml_path = std::path::Path::new(project_path);
            
            // Make the cargo toml path absolute if it's relative
            let abs_cargo_toml_path = if cargo_toml_path.is_absolute() {
                cargo_toml_path.to_path_buf()
            } else {
                // Relative paths are relative to the config file location
                // We need to find where the config was loaded from
                std::env::current_dir().ok()?.join(cargo_toml_path)
            };
            
            tracing::debug!("Checking linked project: {} (absolute: {:?})", project_path, abs_cargo_toml_path);
            
            // Get the project directory (parent of Cargo.toml)
            if let Some(project_dir) = abs_cargo_toml_path.parent() {
                tracing::debug!("Project dir: {:?}, checking if {:?} starts with it", project_dir, abs_file_path);
                // Check if our file is under this project directory
                if abs_file_path.starts_with(project_dir) {
                    // Canonicalize the path to clean it up
                    let clean_dir = project_dir.canonicalize().unwrap_or_else(|_| project_dir.to_path_buf());
                    tracing::debug!("Found matching project! Working dir: {:?}", clean_dir);
                    return Some(clean_dir.to_string_lossy().to_string());
                }
            }
        }
        
        // Fallback: find the nearest Cargo.toml
        abs_file_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists())
            .map(|p| p.to_string_lossy().to_string())
    }
}

/// Extension methods for easy resolution
impl<'a> ConfigResolver<'a> {
    /// Resolve a test command
    pub fn resolve_test_command(
        &self,
        crate_name: &str,
        module_path: &str,
        function_name: &str,
    ) -> Result<CargoCommand, String> {
        let context = ScopeContext::new()
            .with_crate(crate_name.to_string())
            .with_module(module_path.to_string())
            .with_function(function_name.to_string());
        
        self.resolve_command(&context, RunnableKind::Test { 
            test_name: function_name.to_string(), 
            is_async: false 
        })
    }
    
    /// Resolve a binary command
    pub fn resolve_binary_command(
        &self,
        file_path: &str,
    ) -> Result<CargoCommand, String> {
        let context = ScopeContext::new()
            .with_file(file_path.into());
        
        self.resolve_command(&context, RunnableKind::Binary { bin_name: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::v2::{ConfigBuilder, builder::LayerConfigExt};
    use crate::build_system::BuildSystem;

    #[test]
    fn test_basic_resolution() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .args_test(vec!["--nocapture".into()])
                 .env("RUST_LOG", "info");
            })
            .build();
        
        let resolver = config.resolver();
        let context = ScopeContext::new()
            .with_crate("my-crate".into())
            .with_function("test_something".into());
        
        let command = resolver.resolve_command(&context, RunnableKind::Test { 
            test_name: "test_something".into(), 
            is_async: false 
        }).unwrap();
        
        assert_eq!(command.command_type, crate::command::CommandType::Cargo);
        assert!(command.args.contains(&"test".into()));
        assert!(command.args.contains(&"--nocapture".into()));
        assert!(command.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "info"));
    }

    #[test]
    fn test_layered_resolution() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .args_test(vec!["--nocapture".into()])
                 .env("RUST_LOG", "info");
            })
            .crate_override("my-crate", |c| {
                c.framework_test("cargo-nextest")
                 .env("RUST_LOG", "debug");
            })
            .build();
        
        let resolver = config.resolver();
        let context = ScopeContext::new()
            .with_crate("my-crate".into())
            .with_function("test_something".into());
        
        let command = resolver.resolve_command(&context, RunnableKind::Test { 
            test_name: "test_something".into(), 
            is_async: false 
        }).unwrap();
        
        // Should use nextest due to crate override
        assert!(command.args.contains(&"nextest".into()));
        // Should still have nocapture from workspace
        assert!(command.args.contains(&"--nocapture".into()));
        // Should have debug log level from crate override
        assert!(command.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "debug"));
    }

    #[test]
    fn test_specificity_ordering() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .env("LEVEL", "workspace");
            })
            .module_override("tests", |m| {
                m.env("LEVEL", "module");
            })
            .function_override("test_specific", |f| {
                f.env("LEVEL", "function");
            })
            .build();
        
        let resolver = config.resolver();
        
        // Test workspace level
        let context1 = ScopeContext::new()
            .with_crate("my-crate".into());
        let command1 = resolver.resolve_command(&context1, RunnableKind::Test { 
            test_name: "test".into(), 
            is_async: false 
        }).unwrap();
        assert!(command1.env.iter().any(|(k, v)| k == "LEVEL" && v == "workspace"));
        
        // Test module level
        let context2 = ScopeContext::new()
            .with_module("tests".into());
        let command2 = resolver.resolve_command(&context2, RunnableKind::Test { 
            test_name: "test".into(), 
            is_async: false 
        }).unwrap();
        assert!(command2.env.iter().any(|(k, v)| k == "LEVEL" && v == "module"));
        
        // Test function level
        let context3 = ScopeContext::new()
            .with_module("tests".into())
            .with_function("test_specific".into());
        let command3 = resolver.resolve_command(&context3, RunnableKind::Test { 
            test_name: "test_specific".into(), 
            is_async: false 
        }).unwrap();
        assert!(command3.env.iter().any(|(k, v)| k == "LEVEL" && v == "function"));
    }
}