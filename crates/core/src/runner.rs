//! Main runner that coordinates parsing, detection, and command generation

use crate::{
    command::CargoCommand,
    config::{Config, ConfigMerger},
    error::Result,
    parser::{module_resolver::ModuleResolver, RustParser},
    patterns::detector::RunnableDetector,
    types::{Runnable, RunnableKind},
};
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct CargoRunner {
    detector: RunnableDetector,
    parser: RustParser,
    config: Config,
    project_root: Option<PathBuf>,
}

impl CargoRunner {
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;

        let project_root = std::env::var("PROJECT_ROOT")
            .ok()
            .map(PathBuf::from);

        Ok(Self {
            detector: RunnableDetector::new()?,
            parser: RustParser::new()?,
            config,
            project_root,
        })
    }

    pub fn with_config(config: Config) -> Result<Self> {
        Ok(Self {
            detector: RunnableDetector::new()?,
            parser: RustParser::new()?,
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

        // Detect runnables
        debug!("Detecting runnables from file");
        let mut runnables = self.detector.detect_runnables(file_path, Some(line))?;

        // Resolve module paths
        self.resolve_module_paths(file_path, &mut runnables)?;

        Ok(runnables)
    }

    pub fn get_best_runnable_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Option<Runnable>> {
        let mut runnables = self.detect_runnables_at_line(file_path, line)?;
        
        // If we have multiple runnables, pick the most specific one
        if runnables.len() > 1 {
            // Sort by scope size (smaller is more specific)
            runnables.sort_by(|a, b| {
                // For doc tests, use extended scope size if available
                let a_size = if matches!(a.kind, RunnableKind::DocTest { .. }) {
                    if let Some(ref extended) = a.extended_scope {
                        extended.scope.end.line - extended.scope.start.line
                    } else {
                        a.scope.end.line - a.scope.start.line
                    }
                } else {
                    a.scope.end.line - a.scope.start.line
                };
                
                let b_size = if matches!(b.kind, RunnableKind::DocTest { .. }) {
                    if let Some(ref extended) = b.extended_scope {
                        extended.scope.end.line - extended.scope.start.line
                    } else {
                        b.scope.end.line - b.scope.start.line
                    }
                } else {
                    b.scope.end.line - b.scope.start.line
                };
                
                a_size.cmp(&b_size)
            });
        }
        
        Ok(runnables.into_iter().next())
    }

    pub fn build_command(&mut self, file_path: &Path, line: u32) -> Result<Option<CargoCommand>> {
        debug!("build_command: file_path={:?}, line={}", file_path, line);
        if let Some(runnable) = self.get_best_runnable_at_line(file_path, line)? {
            debug!("Found runnable at line {}: {:?}", line, runnable.kind);
            self.build_command_for_runnable(&runnable)
        } else {
            debug!("No runnable found at line {}, using fallback", line);
            // No runnable found, use get_fallback_command which handles everything
            self.get_fallback_command(file_path)
        }
    }

    pub fn build_command_for_runnable(&self, runnable: &Runnable) -> Result<Option<CargoCommand>> {
        let package_name = self.get_package_name(&runnable.file_path)?;
        
        // Use the new clean API with the current config from runner
        let command = crate::command::builder::CommandBuilder::for_runnable(runnable)
            .with_package(package_name.unwrap_or_default())
            .with_project_root(self.project_root.as_deref().unwrap_or_else(|| Path::new(".")))
            .with_config(self.config.clone())
            .build()?;

        Ok(Some(command))
    }

    pub fn get_fallback_command(&mut self, file_path: &Path) -> Result<Option<CargoCommand>> {
        debug!("get_fallback_command: file_path={:?}", file_path);
        
        // Resolve the actual file path first
        let (resolved_path, _) = self.resolve_file_path(file_path)?;
        debug!("get_fallback_command: resolved_path={:?}", resolved_path);
        
        // Ensure project root and reload config for the resolved path
        self.ensure_project_root(&resolved_path)?;
        
        let package_name = self.get_package_name(&resolved_path)?;
        let project_root = self.project_root.as_deref();
        
        debug!("get_fallback_command: cargo.binary_framework={:?}", self.config.cargo.as_ref().and_then(|c| c.binary_framework.as_ref()));
        
        crate::command::fallback::generate_fallback_command(
            &resolved_path,
            package_name.as_deref(),
            project_root,
            Some(self.config.clone()),
        )
    }

    pub fn get_file_command(&mut self, file_path: &Path) -> Result<Option<CargoCommand>> {
        debug!("get_file_command called for: {:?}", file_path);
        // This is essentially the same as fallback command, but always returns it
        // regardless of whether there are runnables in the file
        self.get_fallback_command(file_path)
    }

    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.ensure_project_root(file_path)?;

        // Detect runnables using normal pattern detection
        let mut runnables = self.detector.detect_runnables(file_path, None)?;
        
        // Additionally, check if this is a cargo script file and add a runnable for the whole file
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Some(first_line) = content.lines().next() {
                    if first_line.starts_with("#!") && first_line.contains("cargo") && first_line.contains("-Zscript") {
                        // It's a cargo script file, add a runnable for running the entire script
                        let line_count = content.lines().count();
                        let scope = crate::types::Scope {
                            kind: crate::types::ScopeKind::Function,
                            name: Some("main".to_string()),
                            start: crate::types::Position { line: 0, character: 0 },
                            end: crate::types::Position { 
                                line: line_count.saturating_sub(1) as u32, 
                                character: 0 
                            },
                        };
                        
                        let script_runnable = Runnable {
                            label: "Run cargo script".to_string(),
                            scope,
                            kind: RunnableKind::SingleFileScript { 
                                shebang: first_line.to_string() 
                            },
                            module_path: String::new(),
                            file_path: file_path.to_path_buf(),
                            extended_scope: None,
                        };
                        
                        // Insert at the beginning so it appears first
                        runnables.insert(0, script_runnable);
                    }
                }
            }
        }

        // Resolve module paths
        self.resolve_module_paths(file_path, &mut runnables)?;

        Ok(runnables)
    }


    fn ensure_project_root(&mut self, file_path: &Path) -> Result<()> {
        if self.project_root.is_none() {
            if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path) {
                self.project_root = cargo_toml.parent().map(|p| p.to_path_buf());
            }
        }
        
        // Always reload config for the file path to get proper merged config
        // This is important when the file is in a different project than CWD
        self.reload_config_for_path(file_path)?;
        Ok(())
    }

    fn reload_config_for_path(&mut self, file_path: &Path) -> Result<()> {
        let mut merger = ConfigMerger::new();
        merger.load_configs_for_path(file_path)?;
        self.config = merger.get_merged_config();
        debug!("Reloaded config for path {:?}: cargo.binary_framework={:?}", file_path, self.config.cargo.as_ref().and_then(|c| c.binary_framework.as_ref()));
        Ok(())
    }

    /// Resolve a file path using linked_projects if necessary
    fn resolve_file_path(&self, file_path: &Path) -> Result<(PathBuf, Option<PathBuf>)> {
        debug!("Resolving file path: {:?}", file_path);
        debug!("Current config has linked_projects: {:?}", self.config.cargo.as_ref().and_then(|c| c.linked_projects.as_ref()).is_some());
        
        // If it's already an absolute path and exists, use it directly
        if file_path.is_absolute() && file_path.exists() {
            let project_dir = ModuleResolver::find_cargo_toml(file_path)
                .and_then(|p| p.parent().map(|p| p.to_path_buf()));
            return Ok((file_path.to_path_buf(), project_dir));
        }

        // Try relative to current directory first, but only if it's not under PROJECT_ROOT
        // This prevents finding files in temp dirs when we should use linked_projects
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join(file_path);
            if candidate.exists() {
                // Check if we're in a temp directory and have linked_projects
                if self.config.cargo.as_ref().and_then(|c| c.linked_projects.as_ref()).is_some() && self.project_root.is_some() {
                    // Skip current directory resolution if we have linked projects
                    debug!("Skipping current directory resolution due to linked_projects");
                } else {
                    let project_dir = ModuleResolver::find_cargo_toml(&candidate)
                        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
                    return Ok((candidate, project_dir));
                }
            }
        }

        // Try to find in linked_projects
        if let Some(cargo) = &self.config.cargo {
            if let Some(linked_projects) = &cargo.linked_projects {
                debug!("Checking {} linked projects", linked_projects.len());
                for linked_project in linked_projects {
                    let cargo_toml_path = Path::new(linked_project);
                    if let Some(project_dir) = cargo_toml_path.parent() {
                        let candidate = project_dir.join(file_path);
                        debug!("Checking candidate: {:?}", candidate);
                        if candidate.exists() {
                            debug!("Found match in linked project: {:?}", project_dir);
                            return Ok((candidate, Some(project_dir.to_path_buf())));
                        }
                    }
                }
            }
        }

        // If we have PROJECT_ROOT, try resolving from there
        if let Some(project_root) = &self.project_root {
            let candidate = project_root.join(file_path);
            if candidate.exists() {
                let project_dir = ModuleResolver::find_cargo_toml(&candidate)
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()));
                // If no specific project dir found, use PROJECT_ROOT as working dir
                return Ok((candidate, project_dir.or_else(|| Some(project_root.clone()))));
            }
            
            // Even if file doesn't exist, if we have PROJECT_ROOT and no linked_projects,
            // use PROJECT_ROOT as the working directory
            if self.config.cargo.as_ref().and_then(|c| c.linked_projects.as_ref()).as_ref().map(|lp| lp.is_empty()).unwrap_or(true) {
                return Ok((file_path.to_path_buf(), Some(project_root.clone())));
            }
        }

        // Return the original path if we couldn't resolve it
        Ok((file_path.to_path_buf(), None))
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
        // Use the new config merger to load and merge configs
        let mut merger = ConfigMerger::new();
        
        // Always load from current directory to get package-specific configs
        if let Ok(cwd) = std::env::current_dir() {
            merger.load_configs_for_path(&cwd)?;
        }
        
        // The merger will automatically pick up PROJECT_ROOT config from env var
        let config = merger.get_merged_config();
        
        Ok(config)
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
    
    /// Get the override configuration for a specific runnable
    pub fn get_override_for_runnable(&self, runnable: &Runnable) -> Option<&crate::config::Override> {
        // Determine file type
        let file_type = match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => crate::types::FileType::SingleFileScript,
            RunnableKind::Standalone { .. } => crate::types::FileType::Standalone,
            _ => crate::types::FileType::CargoProject,
        };
        
        // Create a FunctionIdentity from the runnable
        let identity = crate::types::FunctionIdentity {
            package: self.get_package_name(&runnable.file_path).ok().flatten(),
            module_path: if runnable.module_path.is_empty() { None } else { Some(runnable.module_path.clone()) },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
                RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
                RunnableKind::DocTest { struct_or_module_name, method_name } => {
                    if let Some(method) = method_name {
                        Some(format!("{}::{}", struct_or_module_name, method))
                    } else {
                        Some(struct_or_module_name.clone())
                    }
                },
                _ => None,
            },
            file_type: Some(file_type),
        };
        
        self.config.get_override_for(&identity)
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

    /// Get command for a specific position in a file, with proper working directory
    pub fn get_command_at_position_with_dir(
        &mut self,
        file_path: &str,
        line: Option<usize>,
    ) -> Result<CargoCommand> {
        let path = Path::new(file_path);
        
        
        // Resolve the actual file path and project directory
        let (resolved_path, project_dir) = self.resolve_file_path(path)?;
        debug!("Resolved path: {:?}, project_dir: {:?}", resolved_path, project_dir);
        
        // Ensure config is loaded for the resolved path
        self.ensure_project_root(&resolved_path)?;
        
        debug!("After ensure_project_root: cargo.binary_framework={:?}", self.config.cargo.as_ref().and_then(|c| c.binary_framework.as_ref()));

        let command = if let Some(line_num) = line {
            // Line is already 0-based from the CLI
            self.build_command(&resolved_path, line_num as u32)?
        } else {
            self.get_file_command(&resolved_path)?
        };

        if let Some(mut cmd) = command {
            // Set the working directory to the project root
            if let Some(dir) = project_dir {
                cmd.working_dir = Some(dir.to_string_lossy().to_string());
            }
            Ok(cmd)
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
