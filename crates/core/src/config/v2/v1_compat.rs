//! V1 compatibility layer for v2 config
//! 
//! This module provides a way to use v2 config with v1-expecting code
//! during the migration period.

use crate::{
    config::legacy::{
        settings::Config as V1Config,
        cargo_config::CargoConfig,
        rustc_config::RustcConfig,
        bazel_config::BazelConfig,
        frameworks::Frameworks,
    },
    build_system::BuildSystem,
    types::RunnableKind,
};
use super::{Config as V2Config, FrameworkKind};

/// Convert v2 config to v1 format for compatibility
pub fn v2_to_v1_compat(v2_config: &V2Config) -> V1Config {
    // Get workspace layer if it exists
    let workspace_layer = v2_config.layers.iter()
        .find(|layer| matches!(layer.scope, super::Scope::Workspace))
        .map(|l| &l.config);
    
    // Extract build system from workspace layer
    let build_system = workspace_layer
        .and_then(|l| l.build_system.as_ref())
        .cloned()
        .unwrap_or(BuildSystem::Cargo);
    
    // Create frameworks from workspace layer
    let frameworks = workspace_layer.map(|l| {
        Frameworks {
            test: l.frameworks.test.clone(),
            binary: l.frameworks.binary.clone(),
            benchmark: l.frameworks.benchmark.clone(),
            doctest: l.frameworks.doctest.clone(),
        }
    });
    
    // Create appropriate config based on build system
    let (cargo, bazel, rustc) = match build_system {
        BuildSystem::Cargo => {
            let cargo_config = CargoConfig {
                frameworks,
                ..Default::default()
            };
            (Some(cargo_config), None, None)
        }
        BuildSystem::Bazel => {
            let bazel_config = BazelConfig {
                frameworks,
                ..Default::default()
            };
            (None, Some(bazel_config), None)
        }
    };
    
    V1Config {
        cargo,
        bazel,
        rustc,
        single_file_script: None,
        overrides: vec![], // V2 handles overrides differently
    }
}

/// Create a minimal v1 config that delegates to v2
pub fn create_delegating_v1_config() -> V1Config {
    // Return a minimal v1 config that will be overridden by v2
    V1Config {
        cargo: Some(CargoConfig::default()),
        bazel: None,
        rustc: None,
        single_file_script: None,
        overrides: vec![],
    }
}