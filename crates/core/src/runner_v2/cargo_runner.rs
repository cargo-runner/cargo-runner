//! Cargo-specific implementation of CommandRunner

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    parser::{RustParser, module_resolver::ModuleResolver, scope_detector::ScopeDetector},
    patterns::RunnableDetector,
    types::{FileType, Runnable},
};

use super::traits::{CommandRunner, RunnerCommand};

/// Cargo-specific command runner
pub struct CargoRunner;

impl CargoRunner {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl CommandRunner for CargoRunner {
    type Config = Config;
    type Command = CargoCommand;

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        // RunnableDetector has its own mutable state, so we need to create a new instance
        let mut detector = RunnableDetector::new()?;
        let mut runnables = detector.detect_runnables(file_path, None)?;

        // Now resolve module paths for all runnables
        if !runnables.is_empty() {
            // Get package name from Cargo.toml
            let package_name = if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path)
            {
                ModuleResolver::get_package_name_from_cargo_toml(&cargo_toml).ok()
            } else {
                None
            };

            // Create module resolver
            let resolver = if let Some(pkg) = package_name {
                ModuleResolver::with_package_name(pkg)
            } else {
                ModuleResolver::new()
            };

            // Parse the file to get all scopes for module resolution
            let source = std::fs::read_to_string(file_path)?;
            let mut parser = RustParser::new()?;
            let tree = parser.parse(&source)?;
            let mut scope_detector = ScopeDetector::new();
            let extended_scopes = scope_detector.detect_scopes(&tree, &source, file_path)?;
            let scopes: Vec<_> = extended_scopes.iter().map(|es| es.scope.clone()).collect();

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
        }

        Ok(runnables)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let mut detector = RunnableDetector::new()?;
        if let Some(mut runnable) = detector.get_best_runnable_at_line(file_path, line)? {
            // Resolve module path for the runnable
            // Get package name from Cargo.toml
            let package_name = if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path)
            {
                ModuleResolver::get_package_name_from_cargo_toml(&cargo_toml).ok()
            } else {
                None
            };

            // Create module resolver
            let resolver = if let Some(pkg) = package_name {
                ModuleResolver::with_package_name(pkg)
            } else {
                ModuleResolver::new()
            };

            // Parse the file to get all scopes for module resolution
            let source = std::fs::read_to_string(file_path)?;
            let mut parser = RustParser::new()?;
            let tree = parser.parse(&source)?;
            let mut scope_detector = ScopeDetector::new();
            let extended_scopes = scope_detector.detect_scopes(&tree, &source, file_path)?;
            let scopes: Vec<_> = extended_scopes.iter().map(|es| es.scope.clone()).collect();

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
        _config: &Self::Config,
        _file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::CommandBuilder;
        use crate::parser::module_resolver::ModuleResolver;

        // Get the actual package name from Cargo.toml
        let package = if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(&runnable.file_path)
        {
            ModuleResolver::get_package_name_from_cargo_toml(&cargo_toml).ok()
        } else {
            None
        };

        // Build command using CommandBuilder
        let mut builder = CommandBuilder::for_runnable(runnable);
        if let Some(pkg) = package {
            builder = builder.with_package(pkg);
        }

        let command = builder.build()?;

        Ok(command)
    }

    fn validate_command(&self, command: &Self::Command) -> Result<()> {
        // Basic validation - ensure command has required components
        if command.args.is_empty() {
            return Err(crate::error::Error::Other(
                "Command has no arguments".to_string(),
            ));
        }

        // Could add more validation here based on cargo rules
        Ok(())
    }

    fn name(&self) -> &'static str {
        "cargo"
    }
}

// Implement RunnerCommand for CargoCommand
impl RunnerCommand for CargoCommand {
    fn to_shell_command(&self) -> String {
        crate::command::CargoCommand::to_shell_command(self)
    }

    fn execute(&self) -> Result<std::process::ExitStatus> {
        crate::command::CargoCommand::execute(self).map_err(|e| crate::error::Error::IoError(e))
    }

    fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_ref().map(|s| Path::new(s))
    }

    fn env_vars(&self) -> &[(String, String)] {
        &self.env
    }

    fn args(&self) -> &[String] {
        &self.args
    }
}
