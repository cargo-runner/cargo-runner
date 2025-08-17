//! Universal Command Runner Library

use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Core types
#[derive(Debug, Clone)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub working_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Runnable {
    pub label: String,
    pub kind: RunnableKind,
    pub line_start: u32,
    pub line_end: u32,
}

#[derive(Debug, Clone)]
pub enum RunnableKind {
    Test { name: String },
    Benchmark { name: String },
    Main,
    Example { name: String },
}

// Main runner
pub struct CommandRunner {
    plugins: Vec<Box<dyn LanguagePlugin>>,
}

impl CommandRunner {
    pub fn new() -> Result<Self, String> {
        let mut runner = Self {
            plugins: Vec::new(),
        };
        
        // Register built-in plugins
        runner.register_plugin(Box::new(RustPlugin::new()));
        runner.register_plugin(Box::new(PythonPlugin::new()));
        runner.register_plugin(Box::new(JavaScriptPlugin::new()));
        
        Ok(runner)
    }
    
    pub fn register_plugin(&mut self, plugin: Box<dyn LanguagePlugin>) {
        self.plugins.push(plugin);
    }
    
    pub fn analyze(&self, file_path: &Path) -> Result<Vec<Runnable>, String> {
        // Find plugin for file
        let plugin = self.find_plugin(file_path)?;
        
        // Read file
        let source = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        // Detect runnables
        Ok(plugin.detect_runnables(&source))
    }
    
    pub fn run(&self, file_path: &Path, line: Option<u32>) -> Result<Command, String> {
        let plugin = self.find_plugin(file_path)?;
        let source = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let mut runnables = plugin.detect_runnables(&source);
        
        // Filter by line if specified
        if let Some(target_line) = line {
            runnables.retain(|r| r.line_start <= target_line && target_line <= r.line_end);
        }
        
        if runnables.is_empty() {
            return Err("No runnable found at this location".to_string());
        }
        
        // Take first matching runnable
        let runnable = runnables.into_iter().next().unwrap();
        Ok(plugin.build_command(&runnable, file_path))
    }
    
    fn find_plugin(&self, file_path: &Path) -> Result<&dyn LanguagePlugin, String> {
        for plugin in &self.plugins {
            if plugin.can_handle(file_path) {
                return Ok(plugin.as_ref());
            }
        }
        
        Err(format!("No plugin found for file: {}", file_path.display()))
    }
    
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.iter()
            .map(|p| p.name().to_string())
            .collect()
    }
}

// Plugin trait
pub trait LanguagePlugin {
    fn name(&self) -> &str;
    fn can_handle(&self, file_path: &Path) -> bool;
    fn detect_runnables(&self, source: &str) -> Vec<Runnable>;
    fn build_command(&self, runnable: &Runnable, file_path: &Path) -> Command;
}

// Rust plugin
struct RustPlugin;

impl RustPlugin {
    fn new() -> Self {
        Self
    }
}

impl LanguagePlugin for RustPlugin {
    fn name(&self) -> &str {
        "rust"
    }
    
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }
    
    fn detect_runnables(&self, source: &str) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed == "#[test]" {
                // Look for function on next line
                if let Some(next_line) = source.lines().nth(line_num + 1) {
                    if let Some(name) = extract_function_name(next_line) {
                        runnables.push(Runnable {
                            label: format!("Test: {}", name),
                            kind: RunnableKind::Test { name: name.clone() },
                            line_start: line_num as u32,
                            line_end: (line_num + 5) as u32,
                        });
                    }
                }
            }
            
            if trimmed.starts_with("fn main()") {
                runnables.push(Runnable {
                    label: "Run main".to_string(),
                    kind: RunnableKind::Main,
                    line_start: line_num as u32,
                    line_end: (line_num + 1) as u32,
                });
            }
        }
        
        runnables
    }
    
    fn build_command(&self, runnable: &Runnable, file_path: &Path) -> Command {
        match &runnable.kind {
            RunnableKind::Test { name } => Command {
                program: "cargo".to_string(),
                args: vec!["test".to_string(), name.clone(), "--".to_string(), "--exact".to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            },
            RunnableKind::Main => Command {
                program: "cargo".to_string(),
                args: vec!["run".to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            },
            _ => Command {
                program: "echo".to_string(),
                args: vec!["Not implemented".to_string()],
                env: HashMap::new(),
                working_dir: None,
            }
        }
    }
}

// Python plugin
struct PythonPlugin;

impl PythonPlugin {
    fn new() -> Self {
        Self
    }
}

impl LanguagePlugin for PythonPlugin {
    fn name(&self) -> &str {
        "python"
    }
    
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "py" || ext == "pyw")
            .unwrap_or(false)
    }
    
    fn detect_runnables(&self, source: &str) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("def test_") {
                if let Some(name) = extract_function_name(trimmed) {
                    runnables.push(Runnable {
                        label: format!("Test: {}", name),
                        kind: RunnableKind::Test { name: name.clone() },
                        line_start: line_num as u32,
                        line_end: (line_num + 10) as u32,
                    });
                }
            }
            
            if trimmed == "if __name__ == \"__main__\":" {
                runnables.push(Runnable {
                    label: "Run script".to_string(),
                    kind: RunnableKind::Main,
                    line_start: line_num as u32,
                    line_end: (line_num + 1) as u32,
                });
            }
        }
        
        runnables
    }
    
    fn build_command(&self, runnable: &Runnable, file_path: &Path) -> Command {
        match &runnable.kind {
            RunnableKind::Test { name } => Command {
                program: "python".to_string(),
                args: vec!["-m".to_string(), "pytest".to_string(), 
                          format!("{}::{}", file_path.display(), name), "-v".to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            },
            RunnableKind::Main => Command {
                program: "python".to_string(),
                args: vec![file_path.to_string_lossy().to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            },
            _ => Command {
                program: "echo".to_string(),
                args: vec!["Not implemented".to_string()],
                env: HashMap::new(),
                working_dir: None,
            }
        }
    }
}

// JavaScript plugin
struct JavaScriptPlugin;

impl JavaScriptPlugin {
    fn new() -> Self {
        Self
    }
}

impl LanguagePlugin for JavaScriptPlugin {
    fn name(&self) -> &str {
        "javascript"
    }
    
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "js" | "jsx" | "ts" | "tsx"))
            .unwrap_or(false)
    }
    
    fn detect_runnables(&self, source: &str) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            
            if trimmed.starts_with("test(") || trimmed.starts_with("it(") {
                runnables.push(Runnable {
                    label: "Test".to_string(),
                    kind: RunnableKind::Test { name: format!("line_{}", line_num) },
                    line_start: line_num as u32,
                    line_end: (line_num + 5) as u32,
                });
            }
        }
        
        runnables
    }
    
    fn build_command(&self, runnable: &Runnable, file_path: &Path) -> Command {
        match &runnable.kind {
            RunnableKind::Test { .. } => Command {
                program: "npm".to_string(),
                args: vec!["test".to_string(), "--".to_string(), 
                          file_path.to_string_lossy().to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            },
            _ => Command {
                program: "node".to_string(),
                args: vec![file_path.to_string_lossy().to_string()],
                env: HashMap::new(),
                working_dir: file_path.parent().map(|p| p.to_path_buf()),
            }
        }
    }
}

// Helper function
fn extract_function_name(line: &str) -> Option<String> {
    if line.trim().starts_with("fn ") || line.trim().starts_with("def ") {
        let after_keyword = if line.trim().starts_with("fn ") {
            &line.trim()[3..]
        } else {
            &line.trim()[4..]
        };
        
        if let Some(paren_idx) = after_keyword.find('(') {
            return Some(after_keyword[..paren_idx].trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_function_name() {
        assert_eq!(extract_function_name("fn test_foo() {"), Some("test_foo".to_string()));
        assert_eq!(extract_function_name("def test_bar():"), Some("test_bar".to_string()));
    }
    
    #[test]
    fn test_rust_plugin() {
        let plugin = RustPlugin::new();
        assert!(plugin.can_handle(Path::new("test.rs")));
        assert!(!plugin.can_handle(Path::new("test.py")));
    }
}