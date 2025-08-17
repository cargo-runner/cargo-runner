//! Python Language Runner Plugin
//! 
//! Supports Python 2/3, various test frameworks, and package managers.

use crate::core_traits::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub struct PythonRunner {
    config: PluginConfig,
}

impl PythonRunner {
    pub fn new() -> Self {
        Self {
            config: PluginConfig::default(),
        }
    }
    
    /// Detect Python version
    fn detect_python_version(&self) -> Option<String> {
        // Try python3 first, then python
        for cmd in &["python3", "python"] {
            if let Ok(output) = std::process::Command::new(cmd)
                .arg("--version")
                .output()
            {
                let version = String::from_utf8_lossy(&output.stdout);
                if version.contains("Python") {
                    return Some(cmd.to_string());
                }
            }
        }
        None
    }
    
    /// Detect virtual environment
    fn detect_venv(&self, project_root: &Path) -> Option<PathBuf> {
        // Check common virtual environment locations
        let venv_dirs = vec![
            "venv",
            ".venv",
            "env",
            ".env",
            "virtualenv",
        ];
        
        for dir_name in venv_dirs {
            let venv_path = project_root.join(dir_name);
            if venv_path.join("bin/python").exists() || 
               venv_path.join("Scripts/python.exe").exists() {
                return Some(venv_path);
            }
        }
        
        None
    }
    
    /// Detect package manager
    fn detect_package_manager(&self, project_root: &Path) -> PackageManager {
        if project_root.join("poetry.lock").exists() || 
           project_root.join("pyproject.toml").exists() {
            PackageManager::Poetry
        } else if project_root.join("Pipfile").exists() {
            PackageManager::Pipenv
        } else if project_root.join("requirements.txt").exists() {
            PackageManager::Pip
        } else {
            PackageManager::None
        }
    }
    
    /// Detect test framework
    fn detect_test_framework(&self, project_root: &Path) -> TestFramework {
        // Check configuration files
        if project_root.join("pytest.ini").exists() || 
           project_root.join("setup.cfg").exists() && 
           self.file_contains(project_root.join("setup.cfg"), "[tool:pytest]") {
            return TestFramework::Pytest;
        }
        
        if project_root.join(".noserc").exists() || 
           project_root.join("nose.cfg").exists() {
            return TestFramework::Nose;
        }
        
        // Check installed packages (simplified)
        if let Ok(output) = std::process::Command::new("pip")
            .args(&["list"])
            .output()
        {
            let packages = String::from_utf8_lossy(&output.stdout);
            if packages.contains("pytest") {
                return TestFramework::Pytest;
            }
            if packages.contains("nose") {
                return TestFramework::Nose;
            }
        }
        
        // Default to unittest (built-in)
        TestFramework::Unittest
    }
    
    fn file_contains(&self, path: PathBuf, text: &str) -> bool {
        std::fs::read_to_string(path)
            .map(|content| content.contains(text))
            .unwrap_or(false)
    }
    
    fn build_test_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command {
        let project = context.project.as_ref();
        let project_root = project.map(|p| &p.root);
        
        let test_framework = project_root
            .map(|root| self.detect_test_framework(root))
            .unwrap_or(TestFramework::Unittest);
        
        let python_cmd = self.detect_python_version()
            .unwrap_or_else(|| "python".to_string());
        
        let mut args = vec![];
        
        match test_framework {
            TestFramework::Pytest => {
                args.push("-m".to_string());
                args.push("pytest".to_string());
                
                if let RunnableKind::Test { name } = &runnable.kind {
                    // Run specific test
                    args.push(format!("{}::{}", 
                        context.file_path.to_string_lossy(),
                        name
                    ));
                } else {
                    // Run all tests in file
                    args.push(context.file_path.to_string_lossy().to_string());
                }
                
                // Add common pytest args
                args.push("-v".to_string());
            }
            
            TestFramework::Unittest => {
                args.push("-m".to_string());
                args.push("unittest".to_string());
                
                if let RunnableKind::Test { name } = &runnable.kind {
                    // Run specific test method
                    let module_path = self.get_module_path(&context.file_path, project_root);
                    args.push(format!("{}.{}", module_path, name));
                } else {
                    // Discover tests in file
                    args.push("discover".to_string());
                    args.push("-s".to_string());
                    args.push(context.file_path.parent()
                        .unwrap()
                        .to_string_lossy()
                        .to_string());
                }
            }
            
            TestFramework::Nose => {
                args.push("-m".to_string());
                args.push("nose".to_string());
                
                if let RunnableKind::Test { name } = &runnable.kind {
                    let module_path = self.get_module_path(&context.file_path, project_root);
                    args.push(format!("{}:{}", module_path, name));
                } else {
                    args.push(context.file_path.to_string_lossy().to_string());
                }
            }
        }
        
        // Handle virtual environment
        let mut env = HashMap::new();
        if let Some(venv) = project_root.and_then(|r| self.detect_venv(r)) {
            let venv_python = if cfg!(windows) {
                venv.join("Scripts/python.exe")
            } else {
                venv.join("bin/python")
            };
            
            if venv_python.exists() {
                // Use venv Python directly
                return Command {
                    program: venv_python.to_string_lossy().to_string(),
                    args: args[1..].to_vec(), // Skip -m
                    env,
                    working_dir: project_root.map(|p| p.to_path_buf()),
                };
            }
        }
        
        Command {
            program: python_cmd,
            args,
            env,
            working_dir: project_root.map(|p| p.to_path_buf()),
        }
    }
    
    fn build_run_command(&self, context: &ExecutionContext) -> Command {
        let python_cmd = self.detect_python_version()
            .unwrap_or_else(|| "python".to_string());
        
        let mut env = HashMap::new();
        
        // Check for virtual environment
        if let Some(project) = &context.project {
            if let Some(venv) = self.detect_venv(&project.root) {
                let venv_python = if cfg!(windows) {
                    venv.join("Scripts/python.exe")
                } else {
                    venv.join("bin/python")
                };
                
                if venv_python.exists() {
                    return Command {
                        program: venv_python.to_string_lossy().to_string(),
                        args: vec![context.file_path.to_string_lossy().to_string()],
                        env,
                        working_dir: Some(project.root.clone()),
                    };
                }
            }
        }
        
        Command {
            program: python_cmd,
            args: vec![context.file_path.to_string_lossy().to_string()],
            env,
            working_dir: context.project.as_ref().map(|p| p.root.clone()),
        }
    }
    
    fn get_module_path(&self, file_path: &Path, project_root: Option<&Path>) -> String {
        let base = project_root.unwrap_or_else(|| file_path.parent().unwrap());
        
        file_path
            .strip_prefix(base)
            .unwrap_or(file_path)
            .with_extension("")
            .to_string_lossy()
            .replace('/', ".")
            .replace('\\', ".")
    }
}

#[derive(Debug, Clone)]
enum PackageManager {
    Poetry,
    Pipenv,
    Pip,
    None,
}

#[derive(Debug, Clone)]
enum TestFramework {
    Pytest,
    Unittest,
    Nose,
}

impl LanguageRunner for PythonRunner {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "python-runner".to_string(),
            version: "1.0.0".to_string(),
            language: "python".to_string(),
            file_extensions: vec![
                "py".to_string(),
                "pyw".to_string(),
                "py3".to_string(),
            ],
            author: Some("Universal Runner Team".to_string()),
            description: Some("Python runner supporting pytest, unittest, nose, and virtual environments".to_string()),
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
            .map(|ext| matches!(ext, "py" | "pyw" | "py3"))
            .unwrap_or(false)
    }
    
    fn detect_project(&self, path: &Path) -> Option<ProjectInfo> {
        let mut current = if path.is_file() {
            path.parent()?
        } else {
            path
        };
        
        // Walk up to find project markers
        loop {
            // Check for Python project files
            let markers = vec![
                "setup.py",
                "pyproject.toml",
                "requirements.txt",
                "Pipfile",
                ".python-version",
            ];
            
            for marker in &markers {
                if current.join(marker).exists() {
                    let name = self.get_project_name(current)
                        .unwrap_or_else(|| current.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string());
                    
                    let pkg_manager = self.detect_package_manager(current);
                    
                    return Some(ProjectInfo {
                        root: current.to_path_buf(),
                        name,
                        build_system: BuildSystem {
                            name: format!("{:?}", pkg_manager).to_lowercase(),
                            version: None,
                            config_file: current.join(markers[0]),
                        },
                        dependencies: vec![], // Could parse requirements
                        metadata: HashMap::new(),
                    });
                }
            }
            
            // Check for git root as fallback
            if current.join(".git").exists() {
                return Some(ProjectInfo {
                    root: current.to_path_buf(),
                    name: "python-project".to_string(),
                    build_system: BuildSystem {
                        name: "none".to_string(),
                        version: None,
                        config_file: current.to_path_buf(),
                    },
                    dependencies: vec![],
                    metadata: HashMap::new(),
                });
            }
            
            current = current.parent()?;
        }
    }
    
    fn detect_runnables(&self, context: &ExecutionContext) -> Vec<Runnable> {
        let mut runnables = Vec::new();
        
        // Parse Python code to find runnables
        for (line_num, line) in context.source_code.lines().enumerate() {
            let trimmed = line.trim();
            
            // Detect test functions (unittest)
            if trimmed.starts_with("def test_") || 
               (trimmed.starts_with("def ") && trimmed.contains("test")) {
                if let Some(name) = Self::extract_function_name(trimmed) {
                    runnables.push(Runnable {
                        label: format!("Test: {}", name),
                        kind: RunnableKind::Test { name: name.clone() },
                        range: SourceRange {
                            start: Position { line: line_num as u32, column: 0 },
                            end: Position { line: (line_num + 10) as u32, column: 0 },
                        },
                        metadata: HashMap::new(),
                    });
                }
            }
            
            // Detect test classes
            if trimmed.starts_with("class ") && trimmed.contains("Test") {
                if let Some(name) = Self::extract_class_name(trimmed) {
                    runnables.push(Runnable {
                        label: format!("Test Class: {}", name),
                        kind: RunnableKind::Test { name: name.clone() },
                        range: SourceRange {
                            start: Position { line: line_num as u32, column: 0 },
                            end: Position { line: (line_num + 50) as u32, column: 0 },
                        },
                        metadata: HashMap::new(),
                    });
                }
            }
            
            // Detect main block
            if trimmed == "if __name__ == \"__main__\":" || 
               trimmed == "if __name__ == '__main__':" {
                runnables.push(Runnable {
                    label: "Run script".to_string(),
                    kind: RunnableKind::Main,
                    range: SourceRange {
                        start: Position { line: line_num as u32, column: 0 },
                        end: Position { line: (line_num + 1) as u32, column: 0 },
                    },
                    metadata: HashMap::new(),
                });
            }
        }
        
        // If no specific runnables found, add generic run option
        if runnables.is_empty() && !context.source_code.is_empty() {
            runnables.push(Runnable {
                label: "Run Python file".to_string(),
                kind: RunnableKind::Main,
                range: SourceRange {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 1, column: 0 },
                },
                metadata: HashMap::new(),
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
        if self.detect_python_version().is_none() {
            return Err("Python is not installed. Please install from https://python.org".to_string());
        }
        Ok(())
    }
}

impl PythonRunner {
    fn get_project_name(&self, project_root: &Path) -> Option<String> {
        // Try setup.py
        if let Ok(content) = std::fs::read_to_string(project_root.join("setup.py")) {
            for line in content.lines() {
                if line.contains("name=") || line.contains("name =") {
                    if let Some(name) = Self::extract_quoted_value(line) {
                        return Some(name);
                    }
                }
            }
        }
        
        // Try pyproject.toml
        if let Ok(content) = std::fs::read_to_string(project_root.join("pyproject.toml")) {
            // Simple parsing - real would use toml crate
            for line in content.lines() {
                if line.starts_with("name = ") {
                    if let Some(name) = Self::extract_quoted_value(line) {
                        return Some(name);
                    }
                }
            }
        }
        
        None
    }
    
    fn extract_function_name(line: &str) -> Option<String> {
        if line.trim().starts_with("def ") {
            let after_def = &line.trim()[4..];
            if let Some(paren_idx) = after_def.find('(') {
                return Some(after_def[..paren_idx].trim().to_string());
            }
        }
        None
    }
    
    fn extract_class_name(line: &str) -> Option<String> {
        if line.trim().starts_with("class ") {
            let after_class = &line.trim()[6..];
            let end_idx = after_class.find('(')
                .or_else(|| after_class.find(':'))
                .unwrap_or(after_class.len());
            return Some(after_class[..end_idx].trim().to_string());
        }
        None
    }
    
    fn extract_quoted_value(line: &str) -> Option<String> {
        // Extract value between quotes
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = line.find('\'') {
            if let Some(end) = line[start + 1..].find('\'') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
        None
    }
}