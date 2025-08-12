//! Unified runner that manages all specific runners

use std::collections::HashMap;
use std::path::Path;

use crate::{
    build_system::{BuildSystem, BuildSystemDetector, DefaultBuildSystemDetector},
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

use super::{
    cargo_runner::CargoRunner,
    bazel_runner::BazelRunner,
    traits::CommandRunner,
};

/// Unified runner that manages multiple command runners
pub struct UnifiedRunner {
    runners: HashMap<BuildSystem, Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>>,
    config: Config,
}

impl UnifiedRunner {
    /// Create a new unified runner with all available runners
    pub fn new() -> Result<Self> {
        let mut runners = HashMap::new();
        
        // Initialize all runners
        runners.insert(
            BuildSystem::Cargo,
            Box::new(CargoRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>
        );
        
        // Load config
        let config = Config::load()?;
        
        Ok(Self { runners, config })
    }
    
    /// Create with a specific config
    pub fn with_config(config: Config) -> Result<Self> {
        let mut runners = HashMap::new();
        
        runners.insert(
            BuildSystem::Cargo,
            Box::new(CargoRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>
        );
        
        Ok(Self { runners, config })
    }
    
    /// Detect the build system for a given path
    pub fn detect_build_system(&self, path: &Path) -> Result<BuildSystem> {
        // Get the project root to check for build files
        let check_path = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        
        DefaultBuildSystemDetector::detect(check_path)
            .ok_or_else(|| crate::error::Error::Other(format!("No build system detected for path: {}", path.display())))
    }
    
    /// Detect build system with fallback to standalone rustc
    pub fn detect_build_system_with_fallback(&self, path: &Path) -> BuildSystem {
        self.detect_build_system(path)
            .unwrap_or(BuildSystem::Cargo) // Default to Cargo for now
    }
    
    /// Get the appropriate runner for a build system
    pub fn get_runner(&self, build_system: &BuildSystem) -> Result<&dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>> {
        self.runners
            .get(build_system)
            .map(|r| r.as_ref())
            .ok_or_else(|| crate::error::Error::Other(format!("No runner available for build system: {:?}", build_system)))
    }
    
    /// Detect all runnables in a file
    pub fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        let build_system = self.detect_build_system_with_fallback(file_path);
        let runner = self.get_runner(&build_system)?;
        runner.detect_runnables(file_path)
    }
    
    /// Get the best runnable at a specific line
    pub fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let build_system = self.detect_build_system_with_fallback(file_path);
        let runner = self.get_runner(&build_system)?;
        runner.get_runnable_at_line(file_path, line)
    }
    
    /// Build a command for a runnable
    pub fn build_command(&self, runnable: &Runnable) -> Result<crate::command::CargoCommand> {
        let build_system = self.detect_build_system_with_fallback(&runnable.file_path);
        let runner = self.get_runner(&build_system)?;
        
        // Determine file type based on build system and runnable kind
        let file_type = match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => FileType::SingleFileScript,
            RunnableKind::Standalone { .. } => FileType::Standalone,
            _ => FileType::CargoProject,
        };
        
        let command = runner.build_command(runnable, &self.config, file_type)?;
        runner.validate_command(&command)?;
        
        Ok(command)
    }
    
    /// Build a command for a position in a file
    pub fn build_command_at_position(&self, file_path: &Path, line: Option<u32>) -> Result<crate::command::CargoCommand> {
        let runnable = if let Some(line_num) = line {
            self.get_runnable_at_line(file_path, line_num)?
                .ok_or_else(|| crate::error::Error::NoRunnableFound)?
        } else {
            // Get any runnable in the file
            self.detect_runnables(file_path)?
                .into_iter()
                .next()
                .ok_or_else(|| crate::error::Error::NoRunnableFound)?
        };
        
        self.build_command(&runnable)
    }
    
    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
    
    /// Update the configuration
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }
    
    /// Get the name of the currently detected build system
    pub fn current_build_system_name(&self, path: &Path) -> &'static str {
        match self.detect_build_system_with_fallback(path) {
            BuildSystem::Cargo => "cargo",
            BuildSystem::Bazel => "bazel",
        }
    }
}

// Convenience methods that mirror the old CargoRunner API for backward compatibility
impl UnifiedRunner {
    /// Get the best runnable at a position (backward compatibility)
    pub fn get_best_runnable_at_line(&self, path: &Path, line: u32) -> Result<Option<Runnable>> {
        self.get_runnable_at_line(path, line)
    }
    
    /// Get command at position with working directory (backward compatibility)
    pub fn get_command_at_position_with_dir(
        &self, 
        filepath: &Path, 
        line: Option<u32>
    ) -> Result<crate::command::CargoCommand> {
        self.build_command_at_position(filepath, line)
    }
    
    /// Build command for a specific runnable (backward compatibility)
    pub fn build_command_for_runnable(&self, runnable: &Runnable) -> Result<Option<crate::command::CargoCommand>> {
        Ok(Some(self.build_command(runnable)?))
    }
    
    /// Detect all runnables (backward compatibility)
    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.detect_runnables(file_path)
    }
    
    /// Detect runnables at a specific line (backward compatibility)
    pub fn detect_runnables_at_line(&mut self, file_path: &Path, line: u32) -> Result<Vec<Runnable>> {
        let all_runnables = self.detect_runnables(file_path)?;
        
        // Filter to runnables that contain the line
        let runnables: Vec<_> = all_runnables
            .into_iter()
            .filter(|r| r.scope.contains_line(line))
            .collect();
            
        Ok(runnables)
    }
    
    /// Get file command (backward compatibility)
    pub fn get_file_command(&mut self, file_path: &Path) -> Result<Option<crate::command::CargoCommand>> {
        // Try to get any runnable from the file
        let runnables = self.detect_runnables(file_path)?;
        if let Some(runnable) = runnables.into_iter().next() {
            Ok(Some(self.build_command(&runnable)?))
        } else {
            // No runnables found, try to build a generic command
            self.build_command_at_position(file_path, None).map(Some).or(Ok(None))
        }
    }
    
    /// Analyze a file and return all runnables as JSON
    pub fn analyze(&mut self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_runnables(path)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }
    
    /// Analyze a file at a specific line and return runnables as JSON
    pub fn analyze_at_line(&mut self, file_path: &str, line: usize) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_runnables_at_line(path, line as u32)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }
    
    /// Get the override configuration for a specific runnable
    pub fn get_override_for_runnable(
        &self,
        runnable: &Runnable,
    ) -> Option<&crate::config::Override> {
        // Determine file type
        let file_type = match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => crate::types::FileType::SingleFileScript,
            RunnableKind::Standalone { .. } => crate::types::FileType::Standalone,
            _ => crate::types::FileType::CargoProject,
        };
        
        // Create a FunctionIdentity from the runnable
        let identity = crate::types::FunctionIdentity {
            package: None, // TODO: Get package from runnable
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
                RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
                RunnableKind::DocTest {
                    struct_or_module_name,
                    method_name,
                } => {
                    if let Some(method) = method_name {
                        Some(format!("{}::{}", struct_or_module_name, method))
                    } else {
                        Some(struct_or_module_name.clone())
                    }
                }
                _ => None,
            },
            file_type: Some(file_type),
        };
        
        self.config.get_override_for(&identity)
    }
}