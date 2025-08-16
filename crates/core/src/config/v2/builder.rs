//! Configuration builder for fluent API
//! 
//! Provides an easy way to construct layered configurations.

use super::{Config, ConfigLayer, LayerConfig, Scope};
use crate::build_system::BuildSystem;
use std::path::PathBuf;

/// Builder for creating configurations
pub struct ConfigBuilder {
    layers: Vec<ConfigLayer>,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
        }
    }
    
    /// Add a workspace-level configuration
    pub fn workspace<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut LayerConfig),
    {
        let mut config = LayerConfig::new();
        f(&mut config);
        self.layers.push(ConfigLayer::new(Scope::Workspace, config));
        self
    }
    
    /// Add a crate-level override
    pub fn crate_override<F>(mut self, crate_name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(&mut LayerConfig),
    {
        let mut config = LayerConfig::new();
        f(&mut config);
        self.layers.push(ConfigLayer::new(Scope::Crate(crate_name.into()), config));
        self
    }
    
    /// Add a file-level override
    pub fn file_override<F>(mut self, file_path: impl Into<PathBuf>, f: F) -> Self
    where
        F: FnOnce(&mut LayerConfig),
    {
        let mut config = LayerConfig::new();
        f(&mut config);
        self.layers.push(ConfigLayer::new(Scope::File(file_path.into()), config));
        self
    }
    
    /// Add a module-level override
    pub fn module_override<F>(mut self, module_path: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(&mut LayerConfig),
    {
        let mut config = LayerConfig::new();
        f(&mut config);
        self.layers.push(ConfigLayer::new(Scope::Module(module_path.into()), config));
        self
    }
    
    /// Add a function-level override
    pub fn function_override<F>(mut self, function_name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(&mut LayerConfig),
    {
        let mut config = LayerConfig::new();
        f(&mut config);
        self.layers.push(ConfigLayer::new(Scope::Function(function_name.into()), config));
        self
    }
    
    /// Add a raw configuration layer
    pub fn layer(mut self, scope: Scope, config: LayerConfig) -> Self {
        self.layers.push(ConfigLayer::new(scope, config));
        self
    }
    
    /// Build the final configuration
    pub fn build(self) -> Config {
        Config::new(self.layers)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for LayerConfig to make the builder API nicer
pub trait LayerConfigExt {
    fn build_system(&mut self, system: BuildSystem) -> &mut Self;
    fn framework_test(&mut self, strategy: impl Into<String>) -> &mut Self;
    fn framework_binary(&mut self, strategy: impl Into<String>) -> &mut Self;
    fn framework_benchmark(&mut self, strategy: impl Into<String>) -> &mut Self;
    fn framework_doctest(&mut self, strategy: impl Into<String>) -> &mut Self;
    fn framework_build(&mut self, strategy: impl Into<String>) -> &mut Self;
    fn args_all(&mut self, args: Vec<String>) -> &mut Self;
    fn args_test(&mut self, args: Vec<String>) -> &mut Self;
    fn args_binary(&mut self, args: Vec<String>) -> &mut Self;
    fn args_benchmark(&mut self, args: Vec<String>) -> &mut Self;
    fn args_build(&mut self, args: Vec<String>) -> &mut Self;
    fn args_test_binary(&mut self, args: Vec<String>) -> &mut Self;
    fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self;
}

impl LayerConfigExt for LayerConfig {
    fn build_system(&mut self, system: BuildSystem) -> &mut Self {
        self.build_system = Some(system);
        self
    }
    
    fn framework_test(&mut self, strategy: impl Into<String>) -> &mut Self {
        self.frameworks.test = Some(strategy.into());
        self
    }
    
    fn framework_binary(&mut self, strategy: impl Into<String>) -> &mut Self {
        self.frameworks.binary = Some(strategy.into());
        self
    }
    
    fn framework_benchmark(&mut self, strategy: impl Into<String>) -> &mut Self {
        self.frameworks.benchmark = Some(strategy.into());
        self
    }
    
    fn framework_doctest(&mut self, strategy: impl Into<String>) -> &mut Self {
        self.frameworks.doctest = Some(strategy.into());
        self
    }
    
    fn framework_build(&mut self, strategy: impl Into<String>) -> &mut Self {
        self.frameworks.build = Some(strategy.into());
        self
    }
    
    fn args_all(&mut self, args: Vec<String>) -> &mut Self {
        self.args.all = Some(args);
        self
    }
    
    fn args_test(&mut self, args: Vec<String>) -> &mut Self {
        self.args.test = Some(args);
        self
    }
    
    fn args_binary(&mut self, args: Vec<String>) -> &mut Self {
        self.args.binary = Some(args);
        self
    }
    
    fn args_benchmark(&mut self, args: Vec<String>) -> &mut Self {
        self.args.benchmark = Some(args);
        self
    }
    
    fn args_build(&mut self, args: Vec<String>) -> &mut Self {
        self.args.build = Some(args);
        self
    }
    
    fn args_test_binary(&mut self, args: Vec<String>) -> &mut Self {
        self.args.test_binary = Some(args);
        self
    }
    
    fn env(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.env.vars.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .args_test(vec!["--nocapture".into()])
                 .env("RUST_LOG", "info");
            })
            .build();
        
        assert_eq!(config.layers.len(), 1);
        assert!(matches!(config.layers[0].scope, Scope::Workspace));
    }

    #[test]
    fn test_builder_multiple_layers() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test");
            })
            .crate_override("my-crate", |c| {
                c.framework_test("cargo-nextest");
            })
            .file_override("src/bin/app.rs", |f| {
                f.framework_binary("dioxus-serve");
            })
            .build();
        
        assert_eq!(config.layers.len(), 3);
        
        // Check scopes
        assert!(matches!(config.layers[0].scope, Scope::Workspace));
        assert!(matches!(config.layers[1].scope, Scope::Crate(_)));
        assert!(matches!(config.layers[2].scope, Scope::File(_)));
        
        // Check configurations
        assert_eq!(config.layers[0].config.frameworks.test.as_deref(), Some("cargo-test"));
        assert_eq!(config.layers[1].config.frameworks.test.as_deref(), Some("cargo-nextest"));
        assert_eq!(config.layers[2].config.frameworks.binary.as_deref(), Some("dioxus-serve"));
    }

    #[test]
    fn test_builder_complex() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .framework_binary("cargo-run")
                 .framework_benchmark("cargo-bench")
                 .args_all(vec!["--verbose".into()])
                 .env("RUST_LOG", "info");
            })
            .module_override("tests", |m| {
                m.framework_test("cargo-nextest")
                 .args_test(vec!["--nocapture".into()])
                 .env("RUST_BACKTRACE", "1");
            })
            .function_override("test_complex", |f| {
                f.args_test_binary(vec!["--test-threads=1".into()])
                 .env("RUST_LOG", "debug");
            })
            .build();
        
        assert_eq!(config.layers.len(), 3);
        
        // Verify the layers are in the right order
        assert_eq!(config.layers[0].scope.specificity(), 0); // Workspace
        assert_eq!(config.layers[1].scope.specificity(), 3); // Module
        assert_eq!(config.layers[2].scope.specificity(), 5); // Function
    }
}