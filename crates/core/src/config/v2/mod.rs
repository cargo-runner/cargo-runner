//! V2 Configuration System
//! 
//! This module contains the new scope-based configuration system with
//! strategy pattern for framework commands.

use std::path::Path;

pub mod scope;
pub mod strategy;
pub mod registry;
pub mod layer;
pub mod builder;
pub mod resolver;
pub mod json;
pub mod loader;
pub mod helpers;
pub mod target_detection;
pub mod target_selection_v2;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod v2_config_test;

#[cfg(test)]
mod case_insensitive_test;

pub use scope::{Scope, ScopeKind, ScopeContext};
pub use strategy::{FrameworkStrategy, FrameworkKind};
pub use registry::StrategyRegistry;
pub use layer::{ConfigLayer, LayerConfig};
pub use builder::{ConfigBuilder, LayerConfigExt};
pub use resolver::ConfigResolver;
pub use json::JsonConfig;
pub use loader::ConfigLoader;
pub use helpers::scope_context_from_identity;

// Re-export Config as V2Config for backward compatibility
pub type V2Config = Config;

/// V2 Configuration root structure
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Configuration layers from least to most specific
    layers: Vec<ConfigLayer>,
    /// Strategy registry for framework commands
    registry: StrategyRegistry,
    /// Linked projects (from PROJECT_ROOT config)
    pub linked_projects: Option<Vec<String>>,
}

impl Config {
    /// Create a new configuration with the given layers
    pub fn new(layers: Vec<ConfigLayer>) -> Self {
        Self {
            layers,
            registry: StrategyRegistry::new(),
            linked_projects: None,
        }
    }
    
    /// Create a default config with Cargo build system
    pub fn default_with_build_system() -> Self {
        use crate::build_system::BuildSystem;
        
        ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                    .framework_test("cargo-test")
                    .framework_binary("cargo-run")
                    .framework_benchmark("cargo-bench")
                    .framework_doctest("cargo-test")
                    .framework_build("cargo-build");
            })
            .build()
    }
    
    /// Create a default config with detected build system
    pub fn default_with_detected_build_system(file_path: &Path) -> Self {
        use crate::build_system::{BuildSystem, BuildSystemDetector, DefaultBuildSystemDetector};
        
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(file_path).unwrap_or(BuildSystem::Cargo);
        
        match build_system {
            BuildSystem::Cargo => Self::default_with_build_system(),
            BuildSystem::Bazel => ConfigBuilder::new()
                .workspace(|w| {
                    w.build_system(BuildSystem::Bazel)
                        .framework_test("bazel-test")
                        .framework_binary("bazel-run")
                        .framework_benchmark("bazel-bench")
                        .framework_doctest("bazel-test")
                        .framework_build("bazel-build");
                })
                .build(),
            BuildSystem::Rustc => ConfigBuilder::new()
                .workspace(|w| {
                    w.build_system(BuildSystem::Rustc)
                        .framework_test("rustc-test")
                        .framework_binary("rustc-run");
                })
                .build(),
        }
    }

    /// Create a configuration resolver for command building
    pub fn resolver(&self) -> ConfigResolver<'_> {
        ConfigResolver::new(&self.layers, &self.registry, &self.linked_projects)
    }
    
    /// Check if the configuration has any layers
    pub fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }
    
    /// Get the number of layers
    pub fn layers_count(&self) -> usize {
        self.layers.len()
    }
    
    /// Get the layers
    pub fn layers(&self) -> &[ConfigLayer] {
        &self.layers
    }
    
    /// Get a reference to the registry
    pub fn registry(&self) -> &StrategyRegistry {
        &self.registry
    }
    
    /// Merge another config into this one
    /// The other config's layers will be added after this config's layers,
    /// giving them higher priority
    pub fn merge(&mut self, other: Config) {
        tracing::debug!("Merging configs: self has linked_projects: {:?}, other has: {:?}", 
            self.linked_projects.is_some(), other.linked_projects.is_some());
        
        // Add all layers from the other config
        self.layers.extend(other.layers);
        
        // Keep the first linked_projects we find (from the root-most config)
        // Don't overwrite if we already have linked_projects
        if self.linked_projects.is_none() && other.linked_projects.is_some() {
            tracing::debug!("Taking linked_projects from other config");
            self.linked_projects = other.linked_projects;
        } else if self.linked_projects.is_some() && other.linked_projects.is_some() {
            tracing::debug!("Keeping existing linked_projects, ignoring other config's linked_projects");
        }
    }
}