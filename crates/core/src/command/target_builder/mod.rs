//! Target-specific command builders
//! 
//! This module provides specialized builders for each target type to ensure
//! proper command construction without cross-contamination between different
//! target types.

mod binary;
mod doc_test;
mod test;
mod benchmark;
mod module_test;

pub use binary::BinaryBuilder;
pub use doc_test::DocTestBuilder;
pub use test::TestBuilder;
pub use benchmark::BenchmarkBuilder;
pub use module_test::ModuleTestBuilder;

use crate::{
    command::CargoCommand,
    config::{Config, FunctionIdentity},
    error::Result,
    types::{Runnable, RunnableKind},
};
use std::path::Path;

/// Trait for building target-specific cargo commands
pub trait TargetCommandBuilder {
    /// Build the cargo command for this target type
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        project_root: &Path,
    ) -> Result<CargoCommand>;
    
    /// Get the base configuration for this builder
    fn config(&self) -> &Config;
    
    /// Check if an override applies to this runnable
    fn get_override(&self, runnable: &Runnable) -> Option<&crate::config::Override> {
        let identity = self.create_identity(runnable);
        self.config().get_override_for(&identity)
    }
    
    /// Create a function identity from the runnable for override matching
    fn create_identity(&self, runnable: &Runnable) -> FunctionIdentity;
    
    /// Apply common configuration (env vars, working directory)
    fn apply_common_config(&self, command: &mut CargoCommand, _runnable: &Runnable) {
        // Apply global env vars
        if let Some(env) = &self.config().env {
            for (key, value) in env {
                command.env.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Factory for creating the appropriate command builder based on runnable kind
pub fn create_builder(config: Config, runnable: &Runnable) -> Box<dyn TargetCommandBuilder> {
    match &runnable.kind {
        RunnableKind::Binary { .. } => Box::new(BinaryBuilder::new(config)),
        RunnableKind::DocTest { .. } => Box::new(DocTestBuilder::new(config)),
        RunnableKind::Test { .. } => Box::new(TestBuilder::new(config)),
        RunnableKind::Benchmark { .. } => Box::new(BenchmarkBuilder::new(config)),
        RunnableKind::ModuleTests { .. } => Box::new(ModuleTestBuilder::new(config)),
    }
}