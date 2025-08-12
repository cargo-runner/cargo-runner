//! Cargo-specific implementation of CommandRunner

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
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
        let runnables = detector.detect_runnables(file_path, None)?;
        Ok(runnables)
    }
    
    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let mut detector = RunnableDetector::new()?;
        detector.get_best_runnable_at_line(file_path, line)
    }
    
    fn build_command(
        &self,
        runnable: &Runnable,
        _config: &Self::Config,
        _file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::CommandBuilder;
        
        // Determine package name
        let package = runnable.module_path
            .split("::")
            .next()
            .map(|s| s.to_string());
        
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
            return Err(crate::error::Error::Other("Command has no arguments".to_string()));
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
        crate::command::CargoCommand::execute(self)
            .map_err(|e| crate::error::Error::IoError(e))
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