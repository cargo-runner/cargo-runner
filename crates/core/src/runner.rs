//! Main runner that coordinates parsing, detection, and command generation

use crate::{
    cache::RunnableCache,
    command::{builder::CommandBuilder, CargoCommand},
    config::Config,
    error::Result,
    parser::{module_resolver::ModuleResolver, RustParser},
    patterns::detector::RunnableDetector,
    types::Runnable,
};
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct CargoRunner {
    detector: RunnableDetector,
    parser: RustParser,
    cache: RunnableCache,
    config: Config,
    project_root: Option<PathBuf>,
}

impl CargoRunner {
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;
        let cache_dir = config.cache_dir.clone();
        let mut cache = RunnableCache::new(cache_dir);

        if config.cache_enabled {
            let _ = cache.load_from_disk();
        }

        Ok(Self {
            detector: RunnableDetector::new()?,
            parser: RustParser::new()?,
            cache,
            config,
            project_root: None,
        })
    }

    pub fn with_config(config: Config) -> Result<Self> {
        let cache_dir = config.cache_dir.clone();
        let mut cache = RunnableCache::new(cache_dir);

        if config.cache_enabled {
            let _ = cache.load_from_disk();
        }

        Ok(Self {
            detector: RunnableDetector::new()?,
            parser: RustParser::new()?,
            cache,
            config,
            project_root: None,
        })
    }

    pub fn detect_runnables_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Vec<Runnable>> {
        debug!(
            "detect_runnables_at_line: file={:?}, line={}",
            file_path, line
        );
        self.ensure_project_root(file_path)?;

        // Check cache first
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(file_path) {
                debug!("Checking {} cached runnables", cached.len());
                let filtered: Vec<Runnable> = cached
                    .iter()
                    .filter(|r| {
                        let contains = r.scope.contains_line(line);
                        debug!(
                            "  Runnable '{}' scope {}-{} contains line {}? {} (module_path: '{}')",
                            r.label,
                            r.scope.start.line,
                            r.scope.end.line,
                            line,
                            contains,
                            r.module_path
                        );
                        contains
                    })
                    .cloned()
                    .collect();
                if !filtered.is_empty() {
                    debug!("Found {} runnables in cache", filtered.len());
                    return Ok(filtered);
                }
            }
        }

        // Detect runnables
        debug!("Detecting runnables from file");
        let mut runnables = self.detector.detect_runnables(file_path, Some(line))?;

        // Resolve module paths
        self.resolve_module_paths(file_path, &mut runnables)?;

        // Update cache
        if self.config.cache_enabled {
            let mut all_runnables = self.detector.detect_runnables(file_path, None)?;
            self.resolve_module_paths(file_path, &mut all_runnables)?;
            let _ = self.cache.insert(file_path.to_path_buf(), all_runnables);
        }

        Ok(runnables)
    }

    pub fn get_best_runnable_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Option<Runnable>> {
        let runnables = self.detect_runnables_at_line(file_path, line)?;
        Ok(runnables.into_iter().next())
    }

    pub fn build_command(&mut self, file_path: &Path, line: u32) -> Result<Option<CargoCommand>> {
        if let Some(runnable) = self.get_best_runnable_at_line(file_path, line)? {
            self.build_command_for_runnable(&runnable)
        } else {
            // No runnable found, try fallback command
            self.ensure_project_root(file_path)?;
            let package_name = self.get_package_name(file_path)?;
            let project_root = self.project_root.as_deref();
            crate::command::fallback::generate_fallback_command(
                file_path,
                package_name.as_deref(),
                project_root,
            )
        }
    }

    pub fn build_command_for_runnable(&self, runnable: &Runnable) -> Result<Option<CargoCommand>> {
        let package_name = self.get_package_name(&runnable.file_path)?;
        let project_root = self
            .project_root
            .as_deref()
            .unwrap_or_else(|| Path::new("."));

        let builder = CommandBuilder::new(self.config.clone());
        let command = builder.build_command(runnable, package_name.as_deref(), project_root)?;

        Ok(Some(command))
    }

    pub fn get_fallback_command(&mut self, file_path: &Path) -> Result<Option<CargoCommand>> {
        self.ensure_project_root(file_path)?;
        let package_name = self.get_package_name(file_path)?;
        let project_root = self.project_root.as_deref();
        crate::command::fallback::generate_fallback_command(
            file_path,
            package_name.as_deref(),
            project_root,
        )
    }

    pub fn get_file_command(&mut self, file_path: &Path) -> Result<Option<CargoCommand>> {
        // This is essentially the same as fallback command, but always returns it
        // regardless of whether there are runnables in the file
        self.get_fallback_command(file_path)
    }

    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.ensure_project_root(file_path)?;

        // Check cache first
        if self.config.cache_enabled {
            if let Some(cached) = self.cache.get(file_path) {
                return Ok(cached.clone());
            }
        }

        // Detect runnables
        let mut runnables = self.detector.detect_runnables(file_path, None)?;

        // Resolve module paths
        self.resolve_module_paths(file_path, &mut runnables)?;

        // Update cache
        if self.config.cache_enabled {
            let _ = self
                .cache
                .insert(file_path.to_path_buf(), runnables.clone());
        }

        Ok(runnables)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    fn ensure_project_root(&mut self, file_path: &Path) -> Result<()> {
        if self.project_root.is_none() {
            if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path) {
                self.project_root = cargo_toml.parent().map(|p| p.to_path_buf());
            }
        }
        Ok(())
    }

    fn resolve_module_paths(&mut self, file_path: &Path, runnables: &mut [Runnable]) -> Result<()> {
        let source = std::fs::read_to_string(file_path)?;
        let scopes = self.parser.get_scopes(&source, file_path)?;

        let package_name = self.get_package_name(file_path)?;
        let resolver = if let Some(pkg) = package_name {
            ModuleResolver::with_package_name(pkg)
        } else {
            ModuleResolver::new()
        };

        for runnable in runnables {
            runnable.module_path =
                resolver.resolve_module_path(file_path, &scopes, &runnable.scope)?;
        }

        Ok(())
    }

    fn get_package_name(&self, file_path: &Path) -> Result<Option<String>> {
        if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path) {
            Ok(Some(ModuleResolver::get_package_name_from_cargo_toml(
                &cargo_toml,
            )?))
        } else {
            Ok(None)
        }
    }

    fn load_config() -> Result<Config> {
        if let Ok(cwd) = std::env::current_dir() {
            if let Some(config_path) = Config::find_config_file(&cwd) {
                return Config::load_from_file(&config_path);
            }
        }
        Ok(Config::default())
    }

    /// Analyze a file and return all runnables as JSON
    pub fn analyze(&mut self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_all_runnables(path)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }

    /// Get command for a specific position in a file
    pub fn get_command_at_position(
        &mut self,
        file_path: &str,
        line: Option<usize>,
    ) -> Result<String> {
        let path = Path::new(file_path);

        let command = if let Some(line_num) = line {
            // Line is already 0-based from the CLI
            self.build_command(path, line_num as u32)?
        } else {
            self.get_file_command(path)?
        };

        if let Some(cmd) = command {
            Ok(cmd.to_shell_command())
        } else {
            Err(crate::error::Error::NoRunnableFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cargo_runner_basic() -> Result<()> {
        let source = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}

fn main() {
    println!("Hello, world!");
}
"#;

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{source}")?;

        let mut runner = CargoRunner::new()?;

        // Test detection at test line
        let runnables = runner.detect_runnables_at_line(temp_file.path(), 3)?;
        assert_eq!(runnables.len(), 1);
        assert_eq!(runnables[0].label, "Run test 'test_addition'");

        // Test detection at main line
        let runnables = runner.detect_runnables_at_line(temp_file.path(), 7)?;
        assert_eq!(runnables.len(), 1);
        assert!(runnables[0].label.contains("Run"));

        // Test command building
        let command = runner.build_command(temp_file.path(), 3)?;
        assert!(command.is_some());

        Ok(())
    }

    #[test]
    fn test_detect_all_runnables() -> Result<()> {
        let source = r#"
#[test]
fn test_one() {
    assert!(true);
}

#[test]
fn test_two() {
    assert!(true);
}

fn main() {
    println!("Hello!");
}
"#;

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{source}")?;

        let mut runner = CargoRunner::new()?;
        let runnables = runner.detect_all_runnables(temp_file.path())?;

        assert_eq!(runnables.len(), 3);

        let test_runnables: Vec<_> = runnables
            .iter()
            .filter(|r| matches!(r.kind, crate::RunnableKind::Test { .. }))
            .collect();
        assert_eq!(test_runnables.len(), 2);

        let binary_runnables: Vec<_> = runnables
            .iter()
            .filter(|r| matches!(r.kind, crate::RunnableKind::Binary { .. }))
            .collect();
        assert_eq!(binary_runnables.len(), 1);

        Ok(())
    }
}
