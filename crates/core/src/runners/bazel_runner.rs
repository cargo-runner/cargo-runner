//! Bazel-specific implementation of CommandRunner

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    parser::{module_resolver::ModuleResolver, RustParser},
    patterns::RunnableDetector,
    types::{FileType, Runnable},
};

use super::traits::CommandRunner;

/// Bazel-specific command runner
pub struct BazelRunner;

impl BazelRunner {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl CommandRunner for BazelRunner {
    type Config = Config;
    type Command = CargoCommand; // Reusing CargoCommand for now

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        // For Bazel, we still parse Rust files the same way
        let mut detector = RunnableDetector::new()?;
        let mut runnables = detector.detect_runnables(file_path, None)?;
        
        // Resolve module paths for each runnable
        // Create module resolver - for Bazel we use default since no package name
        let resolver = ModuleResolver::new();
        
        // Parse the file to get all scopes for module resolution
        let source = std::fs::read_to_string(file_path)?;
        let mut parser = RustParser::new()?;
        let scopes = parser.get_scopes(&source, file_path)?;
        
        // Resolve module paths for each runnable
        for runnable in &mut runnables {
            match resolver.resolve_module_path(file_path, &scopes, &runnable.scope) {
                Ok(module_path) => {
                    tracing::debug!(
                        "Resolved module path for {}: {}",
                        runnable.label,
                        module_path
                    );
                    runnable.module_path = module_path;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to resolve module path for {}: {}",
                        runnable.label,
                        e
                    );
                }
            }
        }
        
        Ok(runnables)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let mut detector = RunnableDetector::new()?;
        if let Some(mut runnable) = detector.get_best_runnable_at_line(file_path, line)? {
            // Resolve module path for the runnable
            let resolver = ModuleResolver::new();
            
            // Parse the file to get all scopes for module resolution
            let source = std::fs::read_to_string(file_path)?;
            let mut parser = RustParser::new()?;
            let scopes = parser.get_scopes(&source, file_path)?;
            
            // Resolve module path
            match resolver.resolve_module_path(file_path, &scopes, &runnable.scope) {
                Ok(module_path) => {
                    tracing::debug!(
                        "Resolved module path for {}: {}",
                        runnable.label,
                        module_path
                    );
                    runnable.module_path = module_path;
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to resolve module path for {}: {}",
                        runnable.label,
                        e
                    );
                }
            }
            
            Ok(Some(runnable))
        } else {
            Ok(None)
        }
    }

    fn build_command(
        &self,
        runnable: &Runnable,
        config: &Self::Config,
        file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::CommandBuilderImpl;
        use crate::command::builder::bazel::BazelCommandBuilder;

        // Build command using BazelCommandBuilder
        let command = BazelCommandBuilder::build(
            runnable, None, // Bazel doesn't use package in the same way
            config, file_type,
        )?;

        Ok(command)
    }

    fn validate_command(&self, command: &Self::Command) -> Result<()> {
        // Bazel-specific validation
        if command.args.is_empty() {
            return Err(crate::error::Error::Other(
                "Bazel command has no arguments".to_string(),
            ));
        }

        // Ensure it's a bazel command
        match &command.command_type {
            crate::command::CommandType::Bazel => Ok(()),
            _ => Err(crate::error::Error::Other(
                "Expected Bazel command type".to_string(),
            )),
        }
    }

    fn name(&self) -> &'static str {
        "bazel"
    }
}
