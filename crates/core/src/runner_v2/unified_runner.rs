//! Unified runner that manages all specific runners

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{
    build_system::{BuildSystem, BuildSystemDetector, DefaultBuildSystemDetector},
    config::Config,
    error::Result,
    parser::module_resolver::ModuleResolver,
    types::{FileType, Runnable, RunnableKind},
};

use super::{bazel_runner::BazelRunner, cargo_runner::CargoRunner, traits::CommandRunner};

/// Unified runner that manages multiple command runners
pub struct UnifiedRunner {
    runners: HashMap<
        BuildSystem,
        Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
    >,
    config: Config,
}

impl UnifiedRunner {
    /// Create a new unified runner with all available runners
    pub fn new() -> Result<Self> {
        let mut runners = HashMap::new();

        // Initialize all runners
        runners.insert(
            BuildSystem::Cargo,
            Box::new(CargoRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
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
            Box::new(CargoRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );

        Ok(Self { runners, config })
    }

    /// Detect the build system for a given path
    pub fn detect_build_system(&self, path: &Path) -> Result<BuildSystem> {
        // Convert to absolute path to ensure consistent detection
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| crate::error::Error::IoError(e))?
                .join(path)
        };
        
        // Start from the file's directory and walk up to find build files
        let start_path = if abs_path.is_file() {
            abs_path.parent().unwrap_or(&abs_path)
        } else {
            &abs_path
        };
        
        let mut check_path = start_path;

        // Walk up the directory tree looking for build files
        loop {
            if let Some(build_system) = DefaultBuildSystemDetector::detect(check_path) {
                return Ok(build_system);
            }

            // Go up one directory
            match check_path.parent() {
                Some(parent) => check_path = parent,
                None => break,
            }
        }

        Err(crate::error::Error::Other(format!(
            "No build system detected for path: {}",
            path.display()
        )))
    }

    /// Detect build system with fallback to standalone rustc
    pub fn detect_build_system_with_fallback(&self, path: &Path) -> BuildSystem {
        match self.detect_build_system(path) {
            Ok(bs) => bs,
            Err(_) => {
                // For now, default to Cargo when no build system is detected
                // This allows standalone files to be handled by CargoRunner
                BuildSystem::Cargo
            }
        }
    }

    /// Get the appropriate runner for a build system
    pub fn get_runner(
        &self,
        build_system: &BuildSystem,
    ) -> Result<&dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>> {
        self.runners
            .get(build_system)
            .map(|r| r.as_ref())
            .ok_or_else(|| {
                crate::error::Error::Other(format!(
                    "No runner available for build system: {:?}",
                    build_system
                ))
            })
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
        // Write debug to file to ensure we see it
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/cargo-runner-debug.log") {
            writeln!(f, "DEBUG UnifiedRunner::build_command called").ok();
            writeln!(f, "  runnable.kind: {:?}", runnable.kind).ok();
            writeln!(f, "  runnable.file_path: {:?}", runnable.file_path).ok();
        }
        
        let build_system = self.detect_build_system_with_fallback(&runnable.file_path);
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open("/tmp/cargo-runner-debug.log") {
            writeln!(f, "  build_system: {:?}", build_system).ok();
        }
        
        let runner = self.get_runner(&build_system)?;

        // Determine file type based on build system and runnable kind
        let file_type = match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => FileType::SingleFileScript,
            RunnableKind::Standalone { .. } => FileType::Standalone,
            _ => FileType::CargoProject,
        };

        let command = runner.build_command(runnable, &self.config, file_type)?;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open("/tmp/cargo-runner-debug.log") {
            writeln!(f, "  final command: {}", command.to_shell_command()).ok();
        }
        runner.validate_command(&command)?;

        Ok(command)
    }

    /// Build a command for a position in a file
    pub fn build_command_at_position(
        &self,
        file_path: &Path,
        line: Option<u32>,
    ) -> Result<crate::command::CargoCommand> {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/cargo-runner-debug.log") {
            writeln!(f, "DEBUG build_command_at_position called").ok();
            writeln!(f, "  file_path: {:?}", file_path).ok();
            writeln!(f, "  line: {:?}", line).ok();
        }
        
        let runnable = if let Some(line_num) = line {
            // Try to get runnable at specific line
            if let Some(runnable) = self.get_runnable_at_line(file_path, line_num)? {
                runnable
            } else {
                // No runnable found at the specific line - fail fast with helpful error
                let all_runnables = self.detect_runnables(file_path)?;
                if all_runnables.is_empty() {
                    return Err(crate::error::Error::NoRunnableFound);
                }
                
                // Provide helpful error message showing available lines
                let available_lines: Vec<String> = all_runnables
                    .iter()
                    .map(|r| {
                        if matches!(r.kind, crate::types::RunnableKind::DocTest { .. }) {
                            // For doc tests, show the extended range if available
                            if let Some(ext) = &r.extended_scope {
                                let doc_start = ext.scope.start.line.saturating_sub(ext.doc_comment_lines);
                                format!("{}-{} ({})", doc_start + 1, ext.scope.end.line + 1, r.label)
                            } else {
                                format!("{}-{} ({})", r.scope.start.line + 1, r.scope.end.line + 1, r.label)
                            }
                        } else {
                            format!("{}-{} ({})", r.scope.start.line + 1, r.scope.end.line + 1, r.label)
                        }
                    })
                    .collect();
                
                return Err(crate::error::Error::Other(format!(
                    "No runnable found at line {}. Available runnables at lines: {}",
                    line_num + 1,
                    available_lines.join(", ")
                )));
            }
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
        line: Option<u32>,
    ) -> Result<crate::command::CargoCommand> {
        self.build_command_at_position(filepath, line)
    }

    /// Build command for a specific runnable (backward compatibility)
    pub fn build_command_for_runnable(
        &self,
        runnable: &Runnable,
    ) -> Result<Option<crate::command::CargoCommand>> {
        Ok(Some(self.build_command(runnable)?))
    }

    /// Detect all runnables (backward compatibility)
    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.detect_runnables(file_path)
    }

    /// Detect runnables at a specific line (backward compatibility)
    pub fn detect_runnables_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Vec<Runnable>> {
        let all_runnables = self.detect_runnables(file_path)?;

        // Filter to runnables that contain the line
        let runnables: Vec<_> = all_runnables
            .into_iter()
            .filter(|r| r.scope.contains_line(line))
            .collect();

        Ok(runnables)
    }

    /// Get file command (backward compatibility)
    pub fn get_file_command(
        &mut self,
        file_path: &Path,
    ) -> Result<Option<crate::command::CargoCommand>> {
        // For lib.rs files, we should prefer test commands over doc tests
        let runnables = self.detect_runnables(file_path)?;
        
        tracing::debug!("get_file_command: found {} runnables for {:?}", runnables.len(), file_path);
        for (i, runnable) in runnables.iter().enumerate() {
            tracing::debug!("  [{}] {:?} - {:?}", i, runnable.kind, runnable.label);
        }
        
        // Sort runnables to prioritize non-doc test runnables for file-level commands
        let mut sorted_runnables = runnables;
        sorted_runnables.sort_by(|a, b| {
            use crate::types::RunnableKind;
            match (&a.kind, &b.kind) {
                // Deprioritize doc tests for file-level commands
                (RunnableKind::DocTest { .. }, _) => std::cmp::Ordering::Greater,
                (_, RunnableKind::DocTest { .. }) => std::cmp::Ordering::Less,
                // Prefer module tests over individual tests for file-level
                (RunnableKind::ModuleTests { .. }, RunnableKind::Test { .. }) => std::cmp::Ordering::Less,
                (RunnableKind::Test { .. }, RunnableKind::ModuleTests { .. }) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });
        
        tracing::debug!("get_file_command: after sorting:");
        for (i, runnable) in sorted_runnables.iter().enumerate() {
            tracing::debug!("  [{}] {:?} - {:?}", i, runnable.kind, runnable.label);
        }
        
        if let Some(runnable) = sorted_runnables.into_iter().next() {
            tracing::debug!("get_file_command: selected runnable: {:?} - {:?}", runnable.kind, runnable.label);
            Ok(Some(self.build_command(&runnable)?))
        } else {
            // No runnables found, try to build a generic command
            tracing::debug!("get_file_command: no runnables found, trying generic command");
            self.build_command_at_position(file_path, None)
                .map(Some)
                .or(Ok(None))
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

    /// Resolve a file path, handling relative and absolute paths
    pub fn resolve_file_path(&mut self, file_path: &str) -> Result<PathBuf> {
        let path = Path::new(file_path);
        
        // If it's already an absolute path and exists, use it directly
        if path.is_absolute() && path.exists() {
            return Ok(path.to_path_buf());
        }
        
        // Try relative to current directory
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        
        // Return the original path if we can't resolve it
        Ok(path.to_path_buf())
    }

    /// Detect the file type based on the file path and content
    pub fn detect_file_type(&self, file_path: &Path) -> Result<crate::types::FileType> {
        // Check for single-file script first (cargo script)
        if let Ok(content) = std::fs::read_to_string(file_path) {
            if content.trim_start().starts_with("#!/usr/bin/env -S cargo") {
                return Ok(crate::types::FileType::SingleFileScript);
            }
        }

        // Check if it's part of a cargo project
        if ModuleResolver::find_cargo_toml(file_path).is_some() {
            // Check if it's a library, binary, test, etc.
            let file_name = file_path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            
            if file_name == "main.rs" || file_name == "lib.rs" {
                Ok(crate::types::FileType::CargoProject)
            } else if file_path.components().any(|c| c.as_os_str() == "tests") {
                Ok(crate::types::FileType::CargoProject)
            } else if file_path.components().any(|c| c.as_os_str() == "examples") {
                Ok(crate::types::FileType::CargoProject)
            } else if file_path.components().any(|c| c.as_os_str() == "benches") {
                Ok(crate::types::FileType::CargoProject)
            } else {
                Ok(crate::types::FileType::CargoProject)
            }
        } else {
            // Standalone file
            Ok(crate::types::FileType::Standalone)
        }
    }

    /// Get the package name for a file path
    pub fn get_package_name_str(&self, file_path: &Path) -> Result<String> {
        // Find the Cargo.toml file
        if let Some(cargo_toml_path) = ModuleResolver::find_cargo_toml(file_path) {
            // Read and parse the Cargo.toml
            let content = std::fs::read_to_string(&cargo_toml_path)
                .map_err(|e| crate::error::Error::IoError(e))?;
            
            // Simple TOML parsing to get package name
            for line in content.lines() {
                if let Some(name) = line.strip_prefix("name").and_then(|s| s.trim().strip_prefix("=")) {
                    let name = name.trim().trim_matches('"');
                    return Ok(name.to_string());
                }
            }
        }
        
        Err(crate::error::Error::Other("No package name found".to_string()))
    }

    /// Find the config file path for a given file
    pub fn find_config_path(&self, file_path: &Path) -> Result<Option<PathBuf>> {
        let mut current_dir = if file_path.is_file() {
            file_path.parent().map(|p| p.to_path_buf())
        } else {
            Some(file_path.to_path_buf())
        };

        while let Some(dir) = current_dir {
            // Check for .cargo-runner.json
            let config_path = dir.join(".cargo-runner.json");
            if config_path.exists() {
                return Ok(Some(config_path));
            }

            // Check for cargo-runner.json
            let alt_config_path = dir.join("cargo-runner.json");
            if alt_config_path.exists() {
                return Ok(Some(alt_config_path));
            }

            // Move to parent directory
            current_dir = dir.parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }
}
