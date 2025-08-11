use std::path::Path;

/// Determine the type of file for display purposes
pub fn determine_file_type(path: &Path) -> String {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    match file_name {
        "Cargo.toml" => "📦 Cargo.toml".to_string(),
        "main.rs" => "🎯 main.rs (binary)".to_string(),
        "lib.rs" => "📚 lib.rs (library)".to_string(),
        "mod.rs" => "📂 mod.rs (module)".to_string(),
        "build.rs" => "🔨 build.rs (build script)".to_string(),
        _ => {
            if file_name.ends_with("_test.rs") || file_name.ends_with("_tests.rs") {
                "🧪 test file".to_string()
            } else if path.to_string_lossy().contains("/tests/") {
                "🧪 test file".to_string()
            } else if path.to_string_lossy().contains("/examples/") {
                "📝 example file".to_string()
            } else if path.to_string_lossy().contains("/benches/") {
                "⚡ benchmark file".to_string()
            } else if file_name.ends_with(".rs") {
                "📄 Rust file".to_string()
            } else {
                "📄 file".to_string()
            }
        }
    }
}

/// Check if a path is likely a test file
pub fn is_test_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    path_str.contains("/tests/")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with("_tests.rs")
}

/// Check if a path is likely a benchmark file
pub fn is_bench_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/benches/") || path_str.contains("/bench/")
}

/// Check if a path is likely an example file
pub fn is_example_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    path_str.contains("/examples/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_determine_file_type() {
        assert_eq!(
            determine_file_type(&PathBuf::from("Cargo.toml")),
            "📦 Cargo.toml"
        );

        assert_eq!(
            determine_file_type(&PathBuf::from("src/main.rs")),
            "🎯 main.rs (binary)"
        );

        assert_eq!(
            determine_file_type(&PathBuf::from("src/lib.rs")),
            "📚 lib.rs (library)"
        );

        assert_eq!(
            determine_file_type(&PathBuf::from("tests/integration_test.rs")),
            "🧪 test file"
        );
    }

    #[test]
    fn test_is_test_file() {
        assert!(is_test_file(&PathBuf::from("tests/test.rs")));
        assert!(is_test_file(&PathBuf::from("src/module_test.rs")));
        assert!(!is_test_file(&PathBuf::from("src/main.rs")));
    }
}
