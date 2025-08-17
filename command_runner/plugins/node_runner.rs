//! Node.js/JavaScript/TypeScript Runner Plugin
//! 
//! Example of how a different language plugin would work

use crate::core_traits::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct NodeRunner {
    config: PluginConfig,
}

impl NodeRunner {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
        }
    }
    
    fn detect_package_manager(&self, path: &Path) -> Option<String> {
        // Priority: pnpm > yarn > npm
        if path.join("pnpm-lock.yaml").exists() {
            return Some("pnpm".to_string());
        }
        if path.join("yarn.lock").exists() {
            return Some("yarn".to_string());
        }
        if path.join("package-lock.json").exists() {
            return Some("npm".to_string());
        }
        if path.join("package.json").exists() {
            return Some("npm".to_string()); // Default to npm
        }
        None
    }
    
    fn detect_test_framework(&self, project_path: &Path) -> Option<String> {
        // Read package.json to detect test framework
        let package_json = project_path.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&package_json) {
            // Simplified detection - real would parse JSON
            if content.contains("\"jest\"") {
                return Some("jest".to_string());
            }
            if content.contains("\"mocha\"") {
                return Some("mocha".to_string());
            }
            if content.contains("\"vitest\"") {
                return Some("vitest".to_string());
            }
            if content.contains("\"ava\"") {
                return Some("ava".to_string());
            }
            if content.contains("\"tape\"") {
                return Some("tape".to_string());
            }
        }
        None
    }
    
    fn is_typescript(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "ts" || ext == "tsx")
            .unwrap_or(false)
    }
    
    fn build_test_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        let pkg_manager = context.project
            .as_ref()
            .and_then(|p| self.detect_package_manager(&p.root))
            .unwrap_or_else(|| "npm".to_string());
        
        let test_framework = context.project
            .as_ref()
            .and_then(|p| self.detect_test_framework(&p.root));
        
        let mut args = vec![];
        
        // Package manager specific run command
        match pkg_manager.as_str() {
            "yarn" => args.push("test".to_string()),
            "pnpm" => {
                args.push("run".to_string());
                args.push("test".to_string());
            }
            _ => {
                args.push("run".to_string());
                args.push("test".to_string());
            }
        }
        
        // Add test framework specific arguments
        if let RunnableKind::Test { name } = &runnable.kind {
            match test_framework.as_deref() {
                Some("jest") => {
                    args.push("--".to_string());
                    args.push("-t".to_string());
                    args.push(name.clone());
                    
                    // Add file path
                    args.push(context.file_path.to_string_lossy().to_string());
                }
                Some("mocha") => {
                    args.push("--".to_string());
                    args.push("--grep".to_string());
                    args.push(name.clone());
                    args.push(context.file_path.to_string_lossy().to_string());
                }
                Some("vitest") => {
                    args.push("--".to_string());
                    args.push("-t".to_string());
                    args.push(name.clone());
                    args.push(context.file_path.to_string_lossy().to_string());
                }
                _ => {
                    // Generic test command
                    args.push("--".to_string());
                    args.push(context.file_path.to_string_lossy().to_string());
                }
            }
        }
        
        Command {
            program: pkg_manager,
            args,
            env: HashMap::new(),
            working_dir: context.project.as_ref().map(|p| p.root.clone()),
        }
    }
    
    fn build_run_command(&self, context: &ExecutionContext) -> Command {
        let is_ts = self.is_typescript(&context.file_path);
        
        if is_ts {
            // Use ts-node for TypeScript files
            Command {
                program: "npx".to_string(),
                args: vec![
                    "ts-node".to_string(),
                    context.file_path.to_string_lossy().to_string(),
                ],
                env: HashMap::new(),
                working_dir: context.project.as_ref().map(|p| p.root.clone()),
            }
        } else {
            // Use node directly for JavaScript
            Command {
                program: "node".to_string(),
                args: vec![context.file_path.to_string_lossy().to_string()],
                env: HashMap::new(),
                working_dir: context.project.as_ref().map(|p| p.root.clone()),
            }
        }
    }
}

impl LanguageRunner for NodeRunner {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "node-runner".to_string(),
            version: "1.0.0".to_string(),
            language: "javascript".to_string(),
            file_extensions: vec![
                "js".to_string(),
                "jsx".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "mjs".to_string(),
                "cjs".to_string(),
            ],
            author: Some("Universal Runner Team".to_string()),
            description: Some("JavaScript/TypeScript runner supporting npm, yarn, pnpm".to_string()),
            capabilities: PluginCapabilities {
                parse_ast: true,
                detect_tests: true,
                detect_benchmarks: false,
                detect_binaries: true,
                detect_examples: false,
                incremental_parsing: false,
                lsp_support: false,
            },
        }
    }
    
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs"))
            .unwrap_or(false)
    }
    
    fn detect_project(&self, path: &Path) -> Option<ProjectInfo> {
        let mut current = if path.is_file() {
            path.parent()?
        } else {
            path
        };
        
        // Walk up to find package.json
        loop {
            let package_json = current.join("package.json");
            if package_json.exists() {
                let name = self.get_package_name(&package_json)
                    .unwrap_or_else(|| "unknown".to_string());
                
                let pkg_manager = self.detect_package_manager(current)
                    .unwrap_or_else(|| "npm".to_string());
                
                return Some(ProjectInfo {
                    root: current.to_path_buf(),
                    name,
                    build_system: BuildSystem {
                        name: pkg_manager,
                        version: None,
                        config_file: package_json,
                    },
                    dependencies: vec![], // Could parse package.json
                    metadata: HashMap::new(),
                });
            }
            
            current = current.parent()?;
        }
    }
    
    fn detect_runnables(&self, context: &ExecutionContext) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        // Detect test functions (it(), test(), describe())
        for (line_num, line) in context.source_code.lines().enumerate() {
            let trimmed = line.trim();
            
            // Detect test/it blocks
            if trimmed.starts_with("test(") || trimmed.starts_with("it(") {
                if let Some(name) = Self::extract_test_name(trimmed) {
                    runnables.push(Runnable {
                        label: format!("Test: {}", name),
                        kind: RunnableKind::Test { name: name.clone() },
                        range: SourceRange {
                            start: Position { line: line_num as u32, column: 0 },
                            end: Position { line: (line_num + 1) as u32, column: 0 },
                        },
                        metadata: HashMap::new(),
                    });
                }
            }
            
            // Detect describe blocks
            if trimmed.starts_with("describe(") {
                if let Some(name) = Self::extract_test_name(trimmed) {
                    runnables.push(Runnable {
                        label: format!("Test Suite: {}", name),
                        kind: RunnableKind::Test { name: name.clone() },
                        range: SourceRange {
                            start: Position { line: line_num as u32, column: 0 },
                            end: Position { line: (line_num + 10) as u32, column: 0 }, // Estimate
                        },
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        // If no tests found, check if this is a runnable script
        if runnables.is_empty() {
            // Check for Node.js shebang or module.exports
            let first_line = context.source_code.lines().next().unwrap_or("");
            if first_line.starts_with("#!/usr/bin/env node") 
                || context.source_code.contains("module.exports")
                || context.source_code.contains("export default")
                || context.source_code.contains("console.log") {
                
                runnables.push(Runnable {
                    label: "Run script".to_string(),
                    kind: RunnableKind::Main,
                    range: SourceRange {
                        start: Position { line: 0, column: 0 },
                        end: Position { line: 1, column: 0 },
                    },
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Filter by target line if specified
        if let Some(line) = context.target_line {
            runnables.retain(|r| {
                r.range.start.line <= line && line <= r.range.end.line
            });
        }
        
        runnables
    }
    
    fn build_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        match &runnable.kind {
            RunnableKind::Test { .. } => self.build_test_command(runnable, context),
            RunnableKind::Main => self.build_run_command(context),
            _ => Command {
                program: "echo".to_string(),
                args: vec!["Unsupported runnable type".to_string()],
                env: HashMap::new(),
                working_dir: None,
            }
        }
    }
    
    fn validate_environment(&self) -> Result<(), String> {
        // Check if Node.js is installed
        if std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_err()
        {
            return Err("Node.js is not installed. Please install from https://nodejs.org".to_string());
        }
        
        Ok(())
    }
}

impl NodeRunner {
    fn get_package_name(&self, package_json: &Path) -> Option<String> {
        let content = std::fs::read_to_string(package_json).ok()?;
        // Simplified - real would use serde_json
        for line in content.lines() {
            if line.contains("\"name\"") && line.contains(':') {
                if let Some(name_part) = line.split(':').nth(1) {
                    let name = name_part.trim().trim_matches('"').trim_matches(',');
                    return Some(name.to_string());
                }
            }
        }
        None
    }
    
    fn extract_test_name(line: &str) -> Option<String> {
        // Extract test name from test('name', ...) or it('name', ...)
        if let Some(start) = line.find('\'') {
            if let Some(end) = line[start + 1..].find('\'') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = line.find('`') {
            if let Some(end) = line[start + 1..].find('`') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }
}