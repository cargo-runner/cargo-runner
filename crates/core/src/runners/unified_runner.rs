//! Unified runner for v2 configuration system

use std::path::{Path, PathBuf};

use crate::{
    command::CargoCommand,
    config::v2::{ConfigLoader, ScopeContext, V2Config},
    error::Result,
    patterns::RunnableDetector,
    types::{Runnable, RunnableKind},
};

/// Unified runner that uses v2 configuration system
pub struct UnifiedRunner {
    v2_config: V2Config,
    detector: RunnableDetector,
}

impl UnifiedRunner {
    /// Create a new unified runner with v2 config
    pub fn new() -> Result<Self> {
        let v2_config = ConfigLoader::load()
            .unwrap_or_else(|_| V2Config::default_with_build_system());

        Ok(Self {
            v2_config,
            detector: RunnableDetector::new()?,
        })
    }

    /// Create a unified runner with v2 config loaded from a specific path
    pub fn with_path(path: &Path) -> Result<Self> {
        let v2_config = ConfigLoader::load_from_path(path)
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to load v2 config: {}, using default", e);
                V2Config::default_with_detected_build_system(path)
            });

        Ok(Self {
            v2_config,
            detector: RunnableDetector::new()?,
        })
    }
    
    /// Create a unified runner with a specific v2 config (useful for testing)
    pub fn with_config(v2_config: V2Config) -> Result<Self> {
        Ok(Self {
            v2_config,
            detector: RunnableDetector::new()?,
        })
    }

    /// Detect all runnables in a file
    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        let mut runnables = self.detector.detect_runnables(file_path, None)?;

        // Resolve module paths
        crate::runners::common::resolve_module_paths(
            &mut runnables,
            file_path,
        )?;

        Ok(runnables)
    }

    /// Detect runnables at a specific line
    pub fn detect_runnables_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Vec<Runnable>> {
        let mut runnables = self.detector.detect_runnables(file_path, Some(line))?;

        // Resolve module paths
        crate::runners::common::resolve_module_paths(
            &mut runnables,
            file_path,
        )?;

        Ok(runnables)
    }

    /// Get the best runnable at a specific line
    pub fn get_best_runnable_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Option<Runnable>> {
        let runnables = self.detect_runnables_at_line(file_path, line)?;
        Ok(runnables.into_iter().next())
    }

    /// Build a command for a runnable
    pub fn build_command(&self, runnable: &Runnable) -> Result<CargoCommand> {
        // Create scope context
        let context = ScopeContext {
            file_path: Some(runnable.file_path.clone()),
            crate_name: self.get_package_name(&runnable.file_path).ok(),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            function_name: runnable.get_function_name(),
            type_name: None,
            method_name: None,
            scope_kind: None,
        };
        
        tracing::debug!("Building command for runnable: {:?} with file path: {:?}", 
                       runnable.kind, runnable.file_path);

        // Create resolver and resolve command
        let resolver = self.v2_config.resolver();
        resolver
            .resolve_command(&context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))
    }

    /// Build command at a specific position
    pub fn build_command_at_position(
        &mut self,
        file_path: &Path,
        line: Option<u32>,
    ) -> Result<CargoCommand> {
        let runnable = if let Some(line) = line {
            self.get_best_runnable_at_line(file_path, line)?
                .ok_or_else(|| crate::Error::NoRunnableFound)?
        } else {
            // Get file-level runnable
            let runnables = self.detect_all_runnables(file_path)?;
            runnables
                .into_iter()
                .next()
                .ok_or_else(|| crate::Error::NoRunnableFound)?
        };

        self.build_command(&runnable)
    }

    /// Get command at position with directory resolution
    pub fn get_command_at_position_with_dir(
        &mut self,
        filepath: &Path,
        line: Option<u32>,
    ) -> Result<CargoCommand> {
        self.build_command_at_position(filepath, line)
    }

    /// Get file-level command
    pub fn get_file_command(&mut self, file_path: &Path) -> Result<Option<CargoCommand>> {
        // Check if this is a benchmark file or example file
        let is_benchmark_file = file_path.components().any(|c| c.as_os_str() == "benches");
        let is_example_file = file_path.components().any(|c| c.as_os_str() == "examples");

        // Get all runnables
        let mut runnables = self.detect_all_runnables(file_path)?;

        // Sort runnables to prioritize based on file type
        runnables.sort_by(|a, b| {
            use crate::types::RunnableKind;
            match (&a.kind, &b.kind) {
                // For benchmark files, prioritize Binary/Benchmark over tests
                (RunnableKind::Binary { .. }, RunnableKind::Test { .. }) if is_benchmark_file => {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Binary { .. }, RunnableKind::ModuleTests { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Benchmark { .. }, RunnableKind::Test { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Benchmark { .. }, RunnableKind::ModuleTests { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Test { .. }, RunnableKind::Binary { .. }) if is_benchmark_file => {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::ModuleTests { .. }, RunnableKind::Binary { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::Test { .. }, RunnableKind::Benchmark { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::ModuleTests { .. }, RunnableKind::Benchmark { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                // Deprioritize doc tests for file-level commands
                (RunnableKind::DocTest { .. }, _) => std::cmp::Ordering::Greater,
                (_, RunnableKind::DocTest { .. }) => std::cmp::Ordering::Less,
                // For non-benchmark files, prefer module tests over individual tests
                (RunnableKind::ModuleTests { .. }, RunnableKind::Test { .. })
                    if !is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Test { .. }, RunnableKind::ModuleTests { .. })
                    if !is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        // For benchmark files, create a benchmark command if no Binary/Benchmark runnable found
        if is_benchmark_file
            && !runnables.iter().any(|r| {
                matches!(
                    r.kind,
                    RunnableKind::Binary { .. } | RunnableKind::Benchmark { .. }
                )
            })
        {
            // Create a file-level benchmark runnable
            if let Some(stem) = file_path.file_stem() {
                let bench_name = stem.to_string_lossy().to_string();
                let file_runnable = Runnable {
                    scope: crate::types::Scope {
                        start: crate::types::Position::new(0, 0),
                        end: crate::types::Position::new(u32::MAX, 0),
                        kind: crate::types::ScopeKind::File(crate::types::FileScope::Bench {
                            name: Some(bench_name.clone()),
                        }),
                        name: None,
                    },
                    kind: crate::types::RunnableKind::Benchmark {
                        bench_name: bench_name.clone(),
                    },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None,
                    label: format!("Run benchmark '{}'", bench_name),
                };

                return Ok(Some(self.build_command(&file_runnable)?));
            }
        }

        // For example files, ensure we select the right runnable or create one
        if is_example_file
            && runnables
                .iter()
                .any(|r| matches!(r.kind, RunnableKind::Binary { .. }))
        {
            // We have a binary runnable for an example - it will be handled correctly by CargoRunStrategy
            // Just make sure we prioritize it
            runnables.sort_by(|a, b| match (&a.kind, &b.kind) {
                (RunnableKind::Binary { .. }, _) => std::cmp::Ordering::Less,
                (_, RunnableKind::Binary { .. }) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            });
        }

        if let Some(runnable) = runnables.into_iter().next() {
            Ok(Some(self.build_command(&runnable)?))
        } else {
            Ok(None)
        }
    }

    /// Analyze a file and return all runnables as JSON
    pub fn analyze(&mut self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_all_runnables(path)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }

    /// Analyze a file at a specific line and return runnables as JSON
    pub fn analyze_at_line(&mut self, file_path: &str, line: usize) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_runnables_at_line(path, line as u32)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }

    /// Get the v2 configuration
    pub fn v2_config(&self) -> &V2Config {
        &self.v2_config
    }

    /// Get package name for a file
    fn get_package_name(&self, file_path: &Path) -> Result<String> {
        // Find Cargo.toml
        let mut current = file_path.parent();
        while let Some(dir) = current {
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                let manifest = cargo_toml::Manifest::from_path(&cargo_toml).map_err(|e| {
                    crate::Error::ConfigError(format!("Failed to parse Cargo.toml: {}", e))
                })?;

                if let Some(package) = manifest.package {
                    return Ok(package.name);
                }
            }
            current = dir.parent();
        }

        Err(crate::Error::ConfigError("No Cargo.toml found".to_string()))
    }

    /// Resolve file path (for compatibility)
    pub fn resolve_file_path(&self, filepath: &str) -> Result<PathBuf> {
        let path = Path::new(filepath);
        if path.is_absolute() {
            Ok(path.to_path_buf())
        } else {
            Ok(std::env::current_dir()?.join(path))
        }
    }

    /// Get package name as string (for compatibility)
    pub fn get_package_name_str(&self, file_path: &Path) -> Result<String> {
        self.get_package_name(file_path)
    }
}
