use crate::command::CargoCommand;
use crate::types::RunnableKind;
use crate::Result;
use std::path::Path;
use std::fs;

pub struct BazelCommandBuilder;

impl BazelCommandBuilder {
    /// Build a Bazel test command with optional test filtering
    /// Example: bazel test :test --test_output streamed --test_arg --exact --test_arg tests::example
    pub fn build_test_command(
        test_filter: Option<&str>,
        extra_test_binary_args: &[String],
    ) -> Result<CargoCommand> {
        let mut args = vec![
            "test".to_string(),
            ":test".to_string(), // Default test target
            "--test_output".to_string(),
            "streamed".to_string(),
        ];
        
        // Add test filter if provided
        if let Some(filter) = test_filter {
            args.push("--test_arg".to_string());
            args.push("--exact".to_string());
            args.push("--test_arg".to_string());
            args.push(filter.to_string());
        }
        
        // Add extra test binary args
        for arg in extra_test_binary_args {
            args.push("--test_arg".to_string());
            args.push(arg.clone());
        }
        
        Ok(CargoCommand::new_bazel(args))
    }
    
    /// Build a Bazel run command for binaries
    /// Example: bazel run //:server
    pub fn build_run_command(
        binary_name: Option<&str>,
        extra_test_binary_args: &[String],
    ) -> Result<CargoCommand> {
        let target = binary_name.unwrap_or("server");
        let mut args = vec![
            "run".to_string(),
            format!("//:{}", target),
        ];
        
        // Add extra args for binary after --
        if !extra_test_binary_args.is_empty() {
            args.push("--".to_string());
            args.extend(extra_test_binary_args.iter().cloned());
        }
        
        Ok(CargoCommand::new_bazel(args))
    }
    
    /// Build command for a runnable based on its kind
    pub fn build_command_for_runnable(
        runnable_kind: &RunnableKind,
        test_filter: Option<&str>,
        extra_test_binary_args: &[String],
    ) -> Result<Option<CargoCommand>> {
        match runnable_kind {
            RunnableKind::Test { .. } => {
                Ok(Some(Self::build_test_command(
                    test_filter,
                    extra_test_binary_args,
                )?))
            }
            RunnableKind::Binary { .. } => {
                Ok(Some(Self::build_run_command(
                    None, // Use default binary name
                    extra_test_binary_args,
                )?))
            }
            _ => Ok(None), // Bazel doesn't support other runnable kinds yet
        }
    }
}

/// Check if a directory contains Bazel build files
pub fn is_bazel_project(path: &Path) -> bool {
    path.join("BUILD.bazel").exists() 
        || path.join("BUILD").exists() 
        || path.join("MODULE.bazel").exists() 
        || path.join("WORKSPACE").exists() 
        || path.join("WORKSPACE.bazel").exists()
}

/// Check if a specific directory is a Bazel workspace root
/// This does NOT walk up directories - it only checks the given path
pub fn find_bazel_workspace_root(start_path: &Path) -> Option<std::path::PathBuf> {
    // We should only check the PROJECT_ROOT or CWD, not walk up directories
    // The caller (CargoRunner) already has the project root from env or CWD
    
    // If it's a file, we don't look for Bazel at all
    // Bazel detection should only happen at the project root level
    if start_path.is_file() {
        return None;
    }
    
    // Only return this directory if it has Bazel files AND no Cargo.toml
    if is_bazel_project(start_path) && !start_path.join("Cargo.toml").exists() {
        return Some(start_path.to_path_buf());
    }
    
    None
}

/// Target information extracted from BUILD.bazel
#[derive(Debug, Clone)]
pub struct BazelTarget {
    pub name: String,
    pub kind: BazelTargetKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BazelTargetKind {
    RustTest,
    RustBinary,
    RustLibrary,
}

/// Simple parser to extract rust targets from BUILD.bazel files
/// This is a basic implementation that looks for rust_test, rust_binary, and rust_library
pub fn parse_bazel_targets(build_file_path: &Path) -> Vec<BazelTarget> {
    let mut targets = Vec::new();
    
    if let Ok(content) = fs::read_to_string(build_file_path) {
        // Simple regex-based parsing for common patterns
        // Look for rust_test(name = "...")
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Check for rust_test
            if trimmed.starts_with("rust_test(") || (trimmed == "rust_test" && content.contains("name")) {
                if let Some(name) = extract_target_name(&content, line) {
                    targets.push(BazelTarget {
                        name,
                        kind: BazelTargetKind::RustTest,
                    });
                }
            }
            // Check for rust_binary
            else if trimmed.starts_with("rust_binary(") || (trimmed == "rust_binary" && content.contains("name")) {
                if let Some(name) = extract_target_name(&content, line) {
                    targets.push(BazelTarget {
                        name,
                        kind: BazelTargetKind::RustBinary,
                    });
                }
            }
            // Check for rust_library
            else if trimmed.starts_with("rust_library(") || (trimmed == "rust_library" && content.contains("name")) {
                if let Some(name) = extract_target_name(&content, line) {
                    targets.push(BazelTarget {
                        name,
                        kind: BazelTargetKind::RustLibrary,
                    });
                }
            }
        }
    }
    
    targets
}

/// Extract target name from a BUILD.bazel rule
fn extract_target_name(content: &str, start_line: &str) -> Option<String> {
    // Find the position of the current line
    let start_pos = content.find(start_line)?;
    let rule_content = &content[start_pos..];
    
    // Look for the end of this rule (next rule or end of file)
    let rule_end = rule_content.find("\nrust_").unwrap_or(rule_content.len());
    let rule_section = &rule_content[..rule_end];
    
    // Look for name = "..." pattern within this rule
    if let Some(name_pos) = rule_section.find("name") {
        let after_name = &rule_section[name_pos + 4..];
        if let Some(eq_pos) = after_name.find('=') {
            let after_eq = &after_name[eq_pos + 1..].trim_start();
            
            // Handle both quoted and unquoted names
            if after_eq.starts_with('"') {
                // Find closing quote
                if let Some(end_quote) = after_eq[1..].find('"') {
                    return Some(after_eq[1..1+end_quote].to_string());
                }
            } else if after_eq.starts_with('\'') {
                // Handle single quotes
                if let Some(end_quote) = after_eq[1..].find('\'') {
                    return Some(after_eq[1..1+end_quote].to_string());
                }
            }
        }
    }
    
    None
}

/// Determine the Bazel package path from file path relative to workspace root
pub fn get_bazel_package_path(file_path: &Path, workspace_root: &Path) -> Option<String> {
    // Get the directory containing the file
    let file_dir = file_path.parent()?;
    
    // Get relative path from workspace root
    let relative_path = file_dir.strip_prefix(workspace_root).ok()?;
    
    // Convert to Bazel package format
    if relative_path.as_os_str().is_empty() {
        Some("//".to_string())
    } else {
        Some(format!("//{}", relative_path.display()))
    }
}

/// Find the appropriate Bazel target for a given file
pub fn find_bazel_target_for_file(
    file_path: &Path,
    workspace_root: &Path,
    is_test: bool,
) -> Option<String> {
    // Walk up from the file's directory to find BUILD.bazel
    let mut current_dir = file_path.parent()?;
    let (build_file, package_dir) = loop {
        let build_bazel = current_dir.join("BUILD.bazel");
        if build_bazel.exists() {
            break (build_bazel, current_dir);
        }
        
        let build = current_dir.join("BUILD");
        if build.exists() {
            break (build, current_dir);
        }
        
        // Stop at workspace root
        if current_dir == workspace_root {
            return None;
        }
        
        // Go up one directory
        current_dir = current_dir.parent()?;
    };
    
    // Calculate package path based on where we found the BUILD file
    let package_path = if package_dir == workspace_root {
        "//".to_string()
    } else {
        let relative_path = package_dir.strip_prefix(workspace_root).ok()?;
        format!("//{}", relative_path.display())
    };
    
    // Parse targets from BUILD file
    let targets = parse_bazel_targets(&build_file);
    
    // Find appropriate target
    if is_test {
        // Look for test targets
        for target in &targets {
            if target.kind == BazelTargetKind::RustTest {
                if package_path == "//" {
                    return Some(format!(":{}", target.name));
                } else {
                    return Some(format!("{}:{}", package_path, target.name));
                }
            }
        }
    } else {
        // Look for binary targets
        for target in &targets {
            if target.kind == BazelTargetKind::RustBinary {
                if package_path == "//" {
                    return Some(format!(":{}", target.name));
                } else {
                    return Some(format!("{}:{}", package_path, target.name));
                }
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_test_command_simple() {
        let cmd = BazelCommandBuilder::build_test_command(None, &[]).unwrap();
        
        assert_eq!(cmd.command_type, crate::command::CommandType::Bazel);
        assert_eq!(cmd.args, vec![
            "test",
            ":test",
            "--test_output",
            "streamed"
        ]);
    }

    #[test]
    fn test_build_test_command_with_filter() {
        let cmd = BazelCommandBuilder::build_test_command(
            Some("tests::example"),
            &["--nocapture".to_string()]
        ).unwrap();
        
        assert_eq!(cmd.command_type, crate::command::CommandType::Bazel);
        assert_eq!(cmd.args, vec![
            "test",
            ":test",
            "--test_output",
            "streamed",
            "--test_arg",
            "--exact",
            "--test_arg",
            "tests::example",
            "--test_arg",
            "--nocapture"
        ]);
    }

    #[test]
    fn test_build_run_command() {
        let cmd = BazelCommandBuilder::build_run_command(None, &[]).unwrap();
        
        assert_eq!(cmd.command_type, crate::command::CommandType::Bazel);
        assert_eq!(cmd.args, vec!["run", "//:server"]);
    }

    #[test]
    fn test_build_run_command_with_args() {
        let cmd = BazelCommandBuilder::build_run_command(
            Some("my_binary"),
            &["--port".to_string(), "8080".to_string()]
        ).unwrap();
        
        assert_eq!(cmd.command_type, crate::command::CommandType::Bazel);
        assert_eq!(cmd.args, vec!["run", "//:my_binary", "--", "--port", "8080"]);
    }
    
    #[test]
    fn test_parse_bazel_targets() {
        let build_content = r#"
load("@corex_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "corex_lib",
    srcs = ["src/lib.rs"],
    deps = all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "corex",
)

rust_test(
    name = "corex_tests",
    crate = ":corex_lib",
    deps = all_crate_deps(normal_dev = True),
)
"#;
        
        let temp_dir = TempDir::new().unwrap();
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&build_file, build_content).unwrap();
        
        let targets = parse_bazel_targets(&build_file);
        assert_eq!(targets.len(), 2);
        
        assert_eq!(targets[0].name, "corex_lib");
        assert_eq!(targets[0].kind, BazelTargetKind::RustLibrary);
        
        assert_eq!(targets[1].name, "corex_tests");
        assert_eq!(targets[1].kind, BazelTargetKind::RustTest);
    }
    
    #[test]
    fn test_find_bazel_target_for_file() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create workspace file
        fs::write(workspace_root.join("WORKSPACE"), "").unwrap();
        
        // Create corex directory
        let corex_dir = workspace_root.join("corex");
        fs::create_dir(&corex_dir).unwrap();
        
        // Create BUILD.bazel
        let build_content = r#"
rust_library(
    name = "corex_lib",
    srcs = ["src/lib.rs"],
)

rust_test(
    name = "corex_tests",
    crate = ":corex_lib",
)
"#;
        fs::write(corex_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create src directory and lib.rs
        let src_dir = corex_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let lib_file = src_dir.join("lib.rs");
        fs::write(&lib_file, "").unwrap();
        
        // Test finding test target
        let target = find_bazel_target_for_file(&lib_file, workspace_root, true);
        assert_eq!(target, Some("//corex:corex_tests".to_string()));
        
        // Test finding library target (no binary in this case)
        let target = find_bazel_target_for_file(&lib_file, workspace_root, false);
        assert_eq!(target, None);
    }
}