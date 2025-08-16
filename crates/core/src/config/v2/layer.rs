//! Configuration layers for cascading overrides
//! 
//! Each layer represents a configuration at a specific scope level.

use super::scope::{Scope, ScopeContext};
use crate::build_system::BuildSystem;
use std::collections::HashMap;

/// A single configuration layer
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// The scope this layer applies to
    pub scope: Scope,
    /// The configuration for this layer
    pub config: LayerConfig,
}

impl ConfigLayer {
    /// Create a new configuration layer
    pub fn new(scope: Scope, config: LayerConfig) -> Self {
        Self { scope, config }
    }
    
    /// Check if this layer matches the given context
    pub fn matches(&self, context: &ScopeContext) -> bool {
        self.scope.matches(context)
    }
    
    /// Get the specificity of this layer
    pub fn specificity(&self) -> u32 {
        self.scope.specificity()
    }
}

/// Configuration within a layer
#[derive(Debug, Clone, Default)]
pub struct LayerConfig {
    /// Override the build system
    pub build_system: Option<BuildSystem>,
    /// Framework strategy overrides
    pub frameworks: FrameworkOverrides,
    /// Additional arguments configuration
    pub args: ArgsConfig,
    /// Environment variable configuration
    pub env: EnvConfig,
}

impl LayerConfig {
    /// Create a new layer configuration
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Builder method for build system
    pub fn with_build_system(mut self, build_system: BuildSystem) -> Self {
        self.build_system = Some(build_system);
        self
    }
    
    /// Builder method for test framework
    pub fn with_test_framework(mut self, strategy: impl Into<String>) -> Self {
        self.frameworks.test = Some(strategy.into());
        self
    }
    
    /// Builder method for binary framework
    pub fn with_binary_framework(mut self, strategy: impl Into<String>) -> Self {
        self.frameworks.binary = Some(strategy.into());
        self
    }
    
    /// Builder method for benchmark framework
    pub fn with_benchmark_framework(mut self, strategy: impl Into<String>) -> Self {
        self.frameworks.benchmark = Some(strategy.into());
        self
    }
    
    /// Builder method for test args
    pub fn with_test_args(mut self, args: Vec<String>) -> Self {
        self.args.test = Some(args);
        self
    }
    
    /// Builder method for environment variables
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.vars.insert(key.into(), value.into());
        self
    }
    
    /// Apply another layer's config on top of this one
    pub fn apply(&mut self, other: &LayerConfig) {
        // Override build system if specified
        if other.build_system.is_some() {
            self.build_system = other.build_system.clone();
        }
        
        // Override frameworks
        self.frameworks.apply(&other.frameworks);
        
        // Merge args
        self.args.apply(&other.args);
        
        // Merge environment variables
        self.env.apply(&other.env);
    }
}

/// Framework strategy overrides
#[derive(Debug, Clone, Default)]
pub struct FrameworkOverrides {
    /// Test framework strategy name
    pub test: Option<String>,
    /// Binary framework strategy name
    pub binary: Option<String>,
    /// Benchmark framework strategy name
    pub benchmark: Option<String>,
    /// Doc test framework strategy name
    pub doctest: Option<String>,
    /// Build framework strategy name
    pub build: Option<String>,
}

impl FrameworkOverrides {
    /// Apply another set of framework overrides
    pub fn apply(&mut self, other: &FrameworkOverrides) {
        if other.test.is_some() {
            self.test = other.test.clone();
        }
        if other.binary.is_some() {
            self.binary = other.binary.clone();
        }
        if other.benchmark.is_some() {
            self.benchmark = other.benchmark.clone();
        }
        if other.doctest.is_some() {
            self.doctest = other.doctest.clone();
        }
        if other.build.is_some() {
            self.build = other.build.clone();
        }
    }
}

/// Arguments configuration
#[derive(Debug, Clone, Default)]
pub struct ArgsConfig {
    /// Arguments for all commands
    pub all: Option<Vec<String>>,
    /// Arguments for test commands
    pub test: Option<Vec<String>>,
    /// Arguments for binary commands
    pub binary: Option<Vec<String>>,
    /// Arguments for benchmark commands
    pub benchmark: Option<Vec<String>>,
    /// Arguments for build commands
    pub build: Option<Vec<String>>,
    /// Arguments passed to test binaries (after --)
    pub test_binary: Option<Vec<String>>,
}

impl ArgsConfig {
    /// Apply another args config
    pub fn apply(&mut self, other: &ArgsConfig) {
        // For args, we extend rather than replace
        if let Some(args) = &other.all {
            self.all = Some(
                self.all.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .chain(args.clone())
                    .collect()
            );
        }
        
        if let Some(args) = &other.test {
            self.test = Some(
                self.test.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .chain(args.clone())
                    .collect()
            );
        }
        
        if let Some(args) = &other.binary {
            self.binary = Some(
                self.binary.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .chain(args.clone())
                    .collect()
            );
        }
        
        if let Some(args) = &other.benchmark {
            self.benchmark = Some(
                self.benchmark.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .chain(args.clone())
                    .collect()
            );
        }
        
        if let Some(args) = &other.build {
            self.build = Some(
                self.build.clone()
                    .unwrap_or_default()
                    .into_iter()
                    .chain(args.clone())
                    .collect()
            );
        }
        
        if let Some(args) = &other.test_binary {
            // For test_binary args, we want more specific config to override
            // So we put the new args first, then existing args
            self.test_binary = Some(
                args.clone()
                    .into_iter()
                    .chain(self.test_binary.clone().unwrap_or_default())
                    .collect()
            );
        }
    }
}

/// Environment variable configuration
#[derive(Debug, Clone, Default)]
pub struct EnvConfig {
    /// Environment variables to set
    pub vars: HashMap<String, String>,
}

impl EnvConfig {
    /// Apply another env config
    pub fn apply(&mut self, other: &EnvConfig) {
        // Environment variables are overridden
        for (key, value) in &other.vars {
            self.vars.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_matching() {
        let layer = ConfigLayer::new(
            Scope::Crate("my-crate".into()),
            LayerConfig::new(),
        );
        
        let context = ScopeContext::new()
            .with_crate("my-crate".into());
        
        assert!(layer.matches(&context));
        
        let other_context = ScopeContext::new()
            .with_crate("other-crate".into());
        
        assert!(!layer.matches(&other_context));
    }

    #[test]
    fn test_layer_config_merge() {
        let mut base = LayerConfig::new()
            .with_test_framework("cargo-test")
            .with_test_args(vec!["--nocapture".into()])
            .with_env("RUST_LOG", "info");
        
        let override_config = LayerConfig::new()
            .with_test_framework("cargo-nextest")
            .with_test_args(vec!["--no-fail-fast".into()])
            .with_env("RUST_LOG", "debug")
            .with_env("RUST_BACKTRACE", "1");
        
        base.apply(&override_config);
        
        // Framework should be overridden
        assert_eq!(base.frameworks.test.as_deref(), Some("cargo-nextest"));
        
        // Args should be extended
        assert_eq!(base.args.test.as_ref().unwrap().len(), 2);
        assert!(base.args.test.as_ref().unwrap().contains(&"--nocapture".into()));
        assert!(base.args.test.as_ref().unwrap().contains(&"--no-fail-fast".into()));
        
        // Env should be overridden/added
        assert_eq!(base.env.vars.get("RUST_LOG").unwrap(), "debug");
        assert_eq!(base.env.vars.get("RUST_BACKTRACE").unwrap(), "1");
    }
}