//! Target detection utilities for determining cargo target flags from file paths

use std::path::Path;

/// Target type detected from file path
#[derive(Debug, Clone, PartialEq)]
pub enum TargetType {
    /// Binary target (--bin name)
    Bin(String),
    /// Library target (--lib)
    Lib,
    /// Example target (--example name)
    Example(String),
    /// Benchmark target (--bench name)
    Bench(String),
    /// No specific target
    NoTarget,
}

impl TargetType {
    /// Add target flags to args vector
    pub fn add_to_args(&self, args: &mut Vec<String>) {
        match self {
            TargetType::Bin(name) => {
                args.push("--bin".into());
                args.push(name.clone());
            }
            TargetType::Lib => {
                args.push("--lib".into());
            }
            TargetType::Example(name) => {
                args.push("--example".into());
                args.push(name.clone());
            }
            TargetType::Bench(name) => {
                args.push("--bench".into());
                args.push(name.clone());
            }
            TargetType::NoTarget => {}
        }
    }
}

/// Detect target type from file path
pub fn detect_target_from_path(file_path: &str, package_name: Option<&str>) -> TargetType {
    let path = Path::new(file_path);
    
    // Check for benchmark files
    if file_path.contains("/benches/") || file_path.contains("\\benches\\") 
        || file_path.starts_with("benches/") || file_path.starts_with("benches\\") {
        if let Some(stem) = path.file_stem() {
            return TargetType::Bench(stem.to_string_lossy().to_string());
        }
    }
    
    // Check for example files
    if file_path.contains("/examples/") || file_path.contains("\\examples\\") 
        || file_path.starts_with("examples/") || file_path.starts_with("examples\\") {
        if let Some(stem) = path.file_stem() {
            return TargetType::Example(stem.to_string_lossy().to_string());
        }
    }
    
    // Check for src/main.rs
    if file_path.ends_with("src/main.rs") || file_path.ends_with("/src/main.rs") || file_path.ends_with("\\src\\main.rs") {
        if let Some(pkg) = package_name {
            return TargetType::Bin(pkg.to_string());
        }
    }
    
    // Check for src/bin/*.rs files
    if (file_path.contains("/src/bin/") || file_path.contains("\\src\\bin\\") 
        || file_path.starts_with("src/bin/") || file_path.starts_with("src\\bin\\")) 
        && file_path.ends_with(".rs") {
        if let Some(stem) = path.file_stem() {
            return TargetType::Bin(stem.to_string_lossy().to_string());
        }
    }
    
    // Check for src/lib.rs
    if file_path.ends_with("src/lib.rs") || file_path.ends_with("/src/lib.rs") || file_path.ends_with("\\src\\lib.rs") {
        return TargetType::Lib;
    }
    
    TargetType::NoTarget
}

/// Check if a file is in a benchmark directory
pub fn is_benchmark_file(file_path: &str) -> bool {
    file_path.contains("/benches/") || file_path.contains("\\benches\\")
        || file_path.starts_with("benches/") || file_path.starts_with("benches\\")
}

/// Check if a file is in an examples directory
pub fn is_example_file(file_path: &str) -> bool {
    file_path.contains("/examples/") || file_path.contains("\\examples\\")
        || file_path.starts_with("examples/") || file_path.starts_with("examples\\")
}

/// Build a fully qualified test path from module and test name
/// The module path should already be correctly resolved by ModuleResolver
pub fn build_test_path(module_path: Option<&str>, test_name: &str) -> String {
    if let Some(module) = module_path {
        if !module.is_empty() {
            format!("{}::{}", module, test_name)
        } else {
            test_name.to_string()
        }
    } else {
        test_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_benchmark() {
        // Test simple path
        let result = detect_target_from_path("benches/my_bench.rs", None);
        println!("Result for 'benches/my_bench.rs': {:?}", result);
        assert_eq!(
            result,
            TargetType::Bench("my_bench".to_string())
        );
        
        // Test nested path
        assert_eq!(
            detect_target_from_path("crate/benches/perf.rs", None),
            TargetType::Bench("perf".to_string())
        );
    }

    #[test]
    fn test_detect_example() {
        assert_eq!(
            detect_target_from_path("examples/demo.rs", None),
            TargetType::Example("demo".to_string())
        );
    }

    #[test]
    fn test_detect_bin() {
        assert_eq!(
            detect_target_from_path("src/main.rs", Some("myapp")),
            TargetType::Bin("myapp".to_string())
        );
        assert_eq!(
            detect_target_from_path("src/bin/tool.rs", None),
            TargetType::Bin("tool".to_string())
        );
    }

    #[test]
    fn test_detect_lib() {
        assert_eq!(
            detect_target_from_path("src/lib.rs", None),
            TargetType::Lib
        );
    }

    #[test]
    fn test_build_test_path() {
        // Regular test
        assert_eq!(
            build_test_path(Some("tests"), "my_test"),
            "tests::my_test"
        );
        
        // Test with no module
        assert_eq!(
            build_test_path(None, "my_test"),
            "my_test"
        );
        
        // Test with empty module
        assert_eq!(
            build_test_path(Some(""), "my_test"),
            "my_test"
        );
    }
}