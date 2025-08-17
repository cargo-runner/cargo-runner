//! Rust Language Runner Plugin
//! 
//! This is extracted from windrunner to show how it would work as a plugin
//! in the universal command runner framework.

use crate::core_traits::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct RustRunner {
    config: PluginConfig,
    parser: RustAstParser,
}

impl RustRunner {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
            parser: RustAstParser::new(),
        }
    }
    
    /// Detect which Rust build system to use
    fn detect_build_system(&self, path: &Path) -> Option<BuildSystem> {
        // Check for Bazel first (highest priority)
        if path.join("BUILD.bazel").exists() || path.join("BUILD").exists() {
            return Some(BuildSystem {
                name: "bazel".to_string(),
                version: None,
                config_file: path.join("BUILD.bazel"),
            });
        }
        
        // Check for Cargo
        if path.join("Cargo.toml").exists() {
            return Some(BuildSystem {
                name: "cargo".to_string(),
                version: self.get_cargo_version(),
                config_file: path.join("Cargo.toml"),
            });
        }
        
        // Fallback to rustc for standalone files
        if path.extension().map_or(false, |ext| ext == "rs") {
            return Some(BuildSystem {
                name: "rustc".to_string(),
                version: self.get_rustc_version(),
                config_file: path.to_path_buf(),
            });
        }
        
        None
    }
    
    fn get_cargo_version(&self) -> Option<String> {
        std::process::Command::new("cargo")
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.split_whitespace().nth(1).map(String::from))
    }
    
    fn get_rustc_version(&self) -> Option<String> {
        std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.split_whitespace().nth(1).map(String::from))
    }
    
    fn build_cargo_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        let mut args = vec![];
        
        match &runnable.kind {
            RunnableKind::Test { name } => {
                args.push("test".to_string());
                
                // Add package name if in workspace
                if let Some(pkg) = runnable.metadata.get("package") {
                    args.push("--package".to_string());
                    args.push(pkg.clone());
                }
                
                // Add test filter
                if let Some(module_path) = runnable.metadata.get("module_path") {
                    args.push("--".to_string());
                    args.push(format!("{}::{}", module_path, name));
                    args.push("--exact".to_string());
                }
            }
            
            RunnableKind::Benchmark { name } => {
                args.push("bench".to_string());
                
                if let Some(pkg) = runnable.metadata.get("package") {
                    args.push("--package".to_string());
                    args.push(pkg.clone());
                }
                
                args.push("--".to_string());
                args.push(name.clone());
            }
            
            RunnableKind::Main => {
                args.push("run".to_string());
                
                // Add binary name if specified
                if let Some(bin) = runnable.metadata.get("binary") {
                    args.push("--bin".to_string());
                    args.push(bin.clone());
                }
            }
            
            RunnableKind::Example { name } => {
                args.push("run".to_string());
                args.push("--example".to_string());
                args.push(name.clone());
            }
            
            _ => {}
        }
        
        Command {
            program: "cargo".to_string(),
            args,
            env: HashMap::new(),
            working_dir: context.project.as_ref().map(|p| p.root.clone()),
        }
    }
    
    fn build_bazel_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        let mut args = vec![];
        
        match &runnable.kind {
            RunnableKind::Test { .. } => {
                args.push("test".to_string());
                if let Some(target) = runnable.metadata.get("bazel_target") {
                    args.push(target.clone());
                }
            }
            
            RunnableKind::Main => {
                args.push("run".to_string());
                if let Some(target) = runnable.metadata.get("bazel_target") {
                    args.push(target.clone());
                }
            }
            
            RunnableKind::Benchmark { .. } => {
                args.push("run".to_string());
                args.push("-c".to_string());
                args.push("opt".to_string());
                if let Some(target) = runnable.metadata.get("bazel_target") {
                    args.push(target.clone());
                }
            }
            
            _ => {}
        }
        
        Command {
            program: "bazel".to_string(),
            args,
            env: HashMap::new(),
            working_dir: context.project.as_ref().map(|p| p.root.clone()),
        }
    }
    
    fn build_rustc_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        let mut args = vec![
            context.file_path.to_string_lossy().to_string(),
        ];
        
        match runnable.kind {
            RunnableKind::Test { .. } => {
                args.push("--test".to_string());
            }
            RunnableKind::Main => {
                // Just compile and run
                args.push("-o".to_string());
                args.push("/tmp/rust_runner_output".to_string());
            }
            _ => {}
        }
        
        Command {
            program: "rustc".to_string(),
            args,
            env: HashMap::new(),
            working_dir: Some(context.file_path.parent().unwrap().to_path_buf()),
        }
    }
}

impl LanguageRunner for RustRunner {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "rust-runner".to_string(),
            version: "1.0.0".to_string(),
            language: "rust".to_string(),
            file_extensions: vec!["rs".to_string()],
            author: Some("Extracted from windrunner".to_string()),
            description: Some("Rust language runner supporting cargo, bazel, and rustc".to_string()),
            capabilities: PluginCapabilities {
                parse_ast: true,
                detect_tests: true,
                detect_benchmarks: true,
                detect_binaries: true,
                detect_examples: true,
                incremental_parsing: false,
                lsp_support: false,
            },
        }
    }
    
    fn can_handle(&self, file_path: &Path) -> bool {
        file_path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rs")
            .unwrap_or(false)
    }
    
    fn detect_project(&self, path: &Path) -> Option<ProjectInfo> {
        let mut current = if path.is_file() {
            path.parent()?
        } else {
            path
        };
        
        // Walk up to find project root
        loop {
            if let Some(build_system) = self.detect_build_system(current) {
                let name = if build_system.name == "cargo" {
                    self.get_package_name_from_cargo_toml(&build_system.config_file)
                        .unwrap_or_else(|| "unknown".to_string())
                } else {
                    current.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                };
                
                return Some(ProjectInfo {
                    root: current.to_path_buf(),
                    name,
                    build_system,
                    dependencies: vec![], // Could parse Cargo.toml for deps
                    metadata: HashMap::new(),
                });
            }
            
            current = current.parent()?;
        }
    }
    
    fn detect_runnables(&self, context: &ExecutionContext) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        // Use the AST parser to find runnables
        // This is simplified - real implementation would use tree-sitter
        
        // Find test functions (#[test])
        if let Some(test_ranges) = self.parser.find_test_functions(&context.source_code) {
            for (name, range) in test_ranges {
                runnables.push(Runnable {
                    label: format!("Test: {}", name),
                    kind: RunnableKind::Test { name: name.clone() },
                    range,
                    metadata: self.build_metadata(context, &name),
                });
            }
        }
        
        // Find benchmark functions (#[bench])
        if let Some(bench_ranges) = self.parser.find_bench_functions(&context.source_code) {
            for (name, range) in bench_ranges {
                runnables.push(Runnable {
                    label: format!("Benchmark: {}", name),
                    kind: RunnableKind::Benchmark { name: name.clone() },
                    range,
                    metadata: self.build_metadata(context, &name),
                });
            }
        }
        
        // Find main function
        if let Some(main_range) = self.parser.find_main_function(&context.source_code) {
            runnables.push(Runnable {
                label: "Run main".to_string(),
                kind: RunnableKind::Main,
                range: main_range,
                metadata: self.build_metadata(context, "main"),
            });
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
        let build_system = context.project
            .as_ref()
            .map(|p| &p.build_system.name)
            .map(String::as_str);
        
        match build_system {
            Some("cargo") => self.build_cargo_command(runnable, context),
            Some("bazel") => self.build_bazel_command(runnable, context),
            Some("rustc") | None => self.build_rustc_command(runnable, context),
            _ => Command {
                program: "echo".to_string(),
                args: vec!["Unknown build system".to_string()],
                env: HashMap::new(),
                working_dir: None,
            }
        }
    }
    
    fn validate_environment(&self) -> Result<(), String> {
        // Check if Rust toolchain is installed
        if std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .is_err()
        {
            return Err("Rust is not installed. Please install from https://rustup.rs".to_string());
        }
        
        Ok(())
    }
}

impl RustRunner {
    fn get_package_name_from_cargo_toml(&self, path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        // Simple parsing - real implementation would use toml crate
        for line in content.lines() {
            if line.trim().starts_with("name") && line.contains('=') {
                if let Some(name_part) = line.split('=').nth(1) {
                    let name = name_part.trim().trim_matches('"').trim_matches('\'');
                    return Some(name.to_string());
                }
            }
        }
        None
    }
    
    fn build_metadata(&self, context: &ExecutionContext, name: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        
        // Add package name if available
        if let Some(project) = &context.project {
            metadata.insert("package".to_string(), project.name.clone());
        }
        
        // Add module path (simplified - real would use ModuleResolver)
        let module_path = self.get_module_path(&context.file_path);
        metadata.insert("module_path".to_string(), module_path);
        
        // Add file path
        metadata.insert("file".to_string(), context.file_path.to_string_lossy().to_string());
        
        metadata
    }
    
    fn get_module_path(&self, file_path: &Path) -> String {
        // Simplified module path extraction
        let path_str = file_path.to_string_lossy();
        
        if let Some(src_idx) = path_str.find("/src/") {
            let after_src = &path_str[src_idx + 5..];
            let without_ext = after_src.trim_end_matches(".rs");
            without_ext.replace('/', "::")
        } else {
            String::new()
        }
    }
}

// Simplified AST parser (real implementation would use tree-sitter)
struct RustAstParser;

impl RustAstParser {
    fn new() -> Self {
        Self
    }
    
    fn find_test_functions(&self, source: &str) -> Option<Vec<(String, SourceRange)>> {
        // Simplified regex-based detection
        // Real implementation would use tree-sitter
        let mut results = Vec::new();
        
        for (line_num, line) in source.lines().enumerate() {
            if line.trim().starts_with("#[test]") {
                // Look for function on next line
                if let Some(next_line) = source.lines().nth(line_num + 1) {
                    if let Some(name) = Self::extract_function_name(next_line) {
                        results.push((
                            name,
                            SourceRange {
                                start: Position { line: line_num as u32, column: 0 },
                                end: Position { line: (line_num + 5) as u32, column: 0 },
                            }
                        ));
                    }
                }
            }
        }
        
        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }
    
    fn find_bench_functions(&self, source: &str) -> Option<Vec<(String, SourceRange)>> {
        // Similar to find_test_functions but for #[bench]
        None // Simplified
    }
    
    fn find_main_function(&self, source: &str) -> Option<SourceRange> {
        for (line_num, line) in source.lines().enumerate() {
            if line.contains("fn main()") || line.contains("fn main(") {
                return Some(SourceRange {
                    start: Position { line: line_num as u32, column: 0 },
                    end: Position { line: (line_num + 1) as u32, column: 0 },
                });
            }
        }
        None
    }
    
    fn extract_function_name(line: &str) -> Option<String> {
        // Extract function name from "fn test_name() {" or "async fn test_name()"
        let parts: Vec<&str> = line.split_whitespace().collect();
        for (i, part) in parts.iter().enumerate() {
            if *part == "fn" && i + 1 < parts.len() {
                let name_part = parts[i + 1];
                if let Some(paren_idx) = name_part.find('(') {
                    return Some(name_part[..paren_idx].to_string());
                }
            }
        }
        None
    }
}