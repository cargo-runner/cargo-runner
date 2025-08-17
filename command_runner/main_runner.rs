//! Main Universal Command Runner
//! 
//! This is the core orchestrator that:
//! 1. Receives user requests
//! 2. Detects the appropriate language plugin
//! 3. Delegates to the plugin for execution

use crate::core_traits::*;
use crate::plugin_registry::{PluginLoader, PluginRegistry};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct UniversalRunner {
    loader: PluginLoader,
    cache: RunnerCache,
}

/// Cache for performance optimization
struct RunnerCache {
    /// Cache of file -> detected runnables
    runnables_cache: HashMap<PathBuf, Vec<Runnable>>,
    /// Cache of file -> project info
    project_cache: HashMap<PathBuf, ProjectInfo>,
}

impl UniversalRunner {
    pub fn new() -> Result<Self, String> {
        let mut loader = PluginLoader::new();
        loader.initialize()?;
        
        Ok(Self {
            loader,
            cache: RunnerCache {
                runnables_cache: HashMap::new(),
                project_cache: HashMap::new(),
            },
        })
    }
    
    /// Analyze a file and return all runnables
    pub fn analyze(&mut self, file_path: &Path) -> Result<Vec<Runnable>, String> {
        // Check cache first
        if let Some(cached) = self.cache.runnables_cache.get(file_path) {
            return Ok(cached.clone());
        }
        
        // Find appropriate plugin
        let plugin = self.find_plugin_for_file(file_path)?;
        
        // Read file content
        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Detect project info
        let project = plugin.detect_project(file_path);
        
        // Create execution context
        let context = ExecutionContext {
            file_path: file_path.to_path_buf(),
            source_code,
            target_line: None,
            project,
            config: PluginConfig::default(),
        };
        
        // Detect runnables
        let runnables = plugin.detect_runnables(&context);
        
        // Cache results
        self.cache.runnables_cache.insert(file_path.to_path_buf(), runnables.clone());
        
        Ok(runnables)
    }
    
    /// Run code at a specific location
    pub fn run(&mut self, file_path: &Path, line: Option<u32>) -> Result<Command, String> {
        // Find plugin
        let plugin = self.find_plugin_for_file(file_path)?;
        
        // Read file
        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Detect project
        let project = plugin.detect_project(file_path);
        
        // Create context with target line
        let context = ExecutionContext {
            file_path: file_path.to_path_buf(),
            source_code: source_code.clone(),
            target_line: line,
            project,
            config: PluginConfig::default(),
        };
        
        // Detect runnables
        let mut runnables = plugin.detect_runnables(&context);
        
        // Filter by line if specified
        if let Some(target_line) = line {
            runnables.retain(|r| {
                r.range.start.line <= target_line && target_line <= r.range.end.line
            });
        }
        
        // Pick the best runnable
        let runnable = self.select_best_runnable(runnables, line)?;
        
        // Build command
        let command = plugin.build_command(&runnable, &context);
        
        Ok(command)
    }
    
    /// Run with explicit plugin selection
    pub fn run_with_plugin(
        &mut self,
        file_path: &Path,
        plugin_name: &str,
        line: Option<u32>
    ) -> Result<Command, String> {
        // Find specific plugin
        let plugin = self.find_plugin_by_name(plugin_name)?;
        
        // Validate plugin can handle file
        if !plugin.can_handle(file_path) {
            return Err(format!(
                "Plugin '{}' cannot handle file: {}",
                plugin_name,
                file_path.display()
            ));
        }
        
        // Continue with normal run flow
        self.run(file_path, line)
    }
    
    /// Test code at a specific location
    pub fn test(&mut self, file_path: &Path, line: Option<u32>) -> Result<Command, String> {
        // Similar to run, but filter for test runnables only
        let plugin = self.find_plugin_for_file(file_path)?;
        
        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let project = plugin.detect_project(file_path);
        
        let context = ExecutionContext {
            file_path: file_path.to_path_buf(),
            source_code,
            target_line: line,
            project,
            config: PluginConfig::default(),
        };
        
        // Get only test runnables
        let runnables: Vec<_> = plugin.detect_runnables(&context)
            .into_iter()
            .filter(|r| matches!(r.kind, RunnableKind::Test { .. }))
            .collect();
        
        if runnables.is_empty() {
            return Err("No tests found at this location".to_string());
        }
        
        let runnable = self.select_best_runnable(runnables, line)?;
        let command = plugin.build_command(&runnable, &context);
        
        Ok(command)
    }
    
    /// Benchmark code at a specific location
    pub fn benchmark(&mut self, file_path: &Path, line: Option<u32>) -> Result<Command, String> {
        let plugin = self.find_plugin_for_file(file_path)?;
        
        let source_code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let project = plugin.detect_project(file_path);
        
        let context = ExecutionContext {
            file_path: file_path.to_path_buf(),
            source_code,
            target_line: line,
            project,
            config: PluginConfig::default(),
        };
        
        // Get only benchmark runnables
        let runnables: Vec<_> = plugin.detect_runnables(&context)
            .into_iter()
            .filter(|r| matches!(r.kind, RunnableKind::Benchmark { .. }))
            .collect();
        
        if runnables.is_empty() {
            return Err("No benchmarks found at this location".to_string());
        }
        
        let runnable = self.select_best_runnable(runnables, line)?;
        let command = plugin.build_command(&runnable, &context);
        
        Ok(command)
    }
    
    /// List all available plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.loader.registry().list_plugins()
    }
    
    /// Validate environment for a specific file
    pub fn validate_environment(&self, file_path: &Path) -> Result<(), String> {
        let plugin = self.find_plugin_for_file(file_path)?;
        plugin.validate_environment()
    }
    
    /// Clear all caches
    pub fn clear_cache(&mut self) {
        self.cache.runnables_cache.clear();
        self.cache.project_cache.clear();
    }
    
    /// Reload all plugins
    pub fn reload_plugins(&mut self) -> Result<(), String> {
        self.loader.registry_mut().reload()?;
        self.clear_cache();
        Ok(())
    }
    
    // Private helper methods
    
    fn find_plugin_for_file(&self, file_path: &Path) -> Result<Box<dyn LanguageRunner>, String> {
        // This is simplified - real implementation would return Arc from registry
        let extension = file_path.extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "File has no extension".to_string())?;
        
        // For demo, create instances based on extension
        match extension {
            "rs" => {
                use crate::plugins::rust_runner::RustRunner;
                Ok(Box::new(RustRunner::new()))
            }
            "js" | "ts" | "jsx" | "tsx" => {
                use crate::plugins::node_runner::NodeRunner;
                Ok(Box::new(NodeRunner::new()))
            }
            _ => Err(format!("No plugin found for .{} files", extension))
        }
    }
    
    fn find_plugin_by_name(&self, name: &str) -> Result<Box<dyn LanguageRunner>, String> {
        // Simplified - real would search registry
        match name {
            "rust" | "rust-runner" => {
                use crate::plugins::rust_runner::RustRunner;
                Ok(Box::new(RustRunner::new()))
            }
            "node" | "node-runner" | "javascript" => {
                use crate::plugins::node_runner::NodeRunner;
                Ok(Box::new(NodeRunner::new()))
            }
            _ => Err(format!("Unknown plugin: {}", name))
        }
    }
    
    fn select_best_runnable(&self, runnables: Vec<Runnable>, target_line: Option<u32>) -> Result<Runnable, String> {
        if runnables.is_empty() {
            return Err("No runnable found at this location".to_string());
        }
        
        // If only one runnable, return it
        if runnables.len() == 1 {
            return Ok(runnables.into_iter().next().unwrap());
        }
        
        // Score runnables based on proximity to target line
        if let Some(line) = target_line {
            let mut scored: Vec<_> = runnables.into_iter()
                .map(|r| {
                    let distance = if r.range.start.line <= line && line <= r.range.end.line {
                        0 // Perfect match
                    } else if line < r.range.start.line {
                        r.range.start.line - line
                    } else {
                        line - r.range.end.line
                    };
                    
                    // Prefer smaller scopes (more specific)
                    let scope_size = r.range.end.line - r.range.start.line;
                    let score = (distance as i32) * 1000 + (scope_size as i32);
                    
                    (score, r)
                })
                .collect();
            
            // Sort by score (lower is better)
            scored.sort_by_key(|(score, _)| *score);
            
            Ok(scored.into_iter().next().unwrap().1)
        } else {
            // No target line, prefer main or first test
            for r in &runnables {
                if matches!(r.kind, RunnableKind::Main) {
                    return Ok(r.clone());
                }
            }
            
            // Return first runnable
            Ok(runnables.into_iter().next().unwrap())
        }
    }
}

/// Public API for the universal runner
pub struct CommandRunner {
    runner: UniversalRunner,
}

impl CommandRunner {
    /// Create a new command runner
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            runner: UniversalRunner::new()?,
        })
    }
    
    /// Main entry point - parse filepath:line syntax
    pub fn execute(&mut self, target: &str) -> Result<Command, String> {
        // Parse target (filepath:line or just filepath)
        let (file_path, line) = Self::parse_target(target)?;
        
        // Run at the specified location
        self.runner.run(&file_path, line)
    }
    
    /// Execute with specific action
    pub fn execute_action(&mut self, action: &str, target: &str) -> Result<Command, String> {
        let (file_path, line) = Self::parse_target(target)?;
        
        match action {
            "run" => self.runner.run(&file_path, line),
            "test" => self.runner.test(&file_path, line),
            "bench" | "benchmark" => self.runner.benchmark(&file_path, line),
            "analyze" => {
                let runnables = self.runner.analyze(&file_path)?;
                // Return a command that prints the analysis
                Ok(Command {
                    program: "echo".to_string(),
                    args: vec![format!("Found {} runnables", runnables.len())],
                    env: HashMap::new(),
                    working_dir: None,
                })
            }
            _ => Err(format!("Unknown action: {}", action))
        }
    }
    
    /// List available plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.runner.list_plugins()
    }
    
    /// Parse filepath:line syntax
    fn parse_target(target: &str) -> Result<(PathBuf, Option<u32>), String> {
        if let Some(colon_idx) = target.rfind(':') {
            let file_part = &target[..colon_idx];
            let line_part = &target[colon_idx + 1..];
            
            if let Ok(line) = line_part.parse::<u32>() {
                return Ok((PathBuf::from(file_part), Some(line)));
            }
        }
        
        // No line number specified
        Ok((PathBuf::from(target), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_target() {
        let (path, line) = CommandRunner::parse_target("main.rs:42").unwrap();
        assert_eq!(path, PathBuf::from("main.rs"));
        assert_eq!(line, Some(42));
        
        let (path, line) = CommandRunner::parse_target("src/lib.rs").unwrap();
        assert_eq!(path, PathBuf::from("src/lib.rs"));
        assert_eq!(line, None);
    }
}