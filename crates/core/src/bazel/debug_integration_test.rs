//! Debug test for integration test issues

#[cfg(test)]
mod tests {
    use crate::bazel::BazelTargetFinder;
    use std::path::PathBuf;
    
    #[test]
    fn test_debug_relative_paths() {
        // Test various path scenarios
        let test_cases = vec![
            ("server/tests/just_test.rs", "server", "tests/just_test.rs"),
            ("server/tests/just_test.rs", "server", "tests/*.rs"),
            ("tests/just_test.rs", ".", "tests/just_test.rs"),
            ("tests/just_test.rs", ".", "tests/*.rs"),
        ];
        
        for (file_path, build_dir, expected_relative) in test_cases {
            println!("\nTest case:");
            println!("  File path: {}", file_path);
            println!("  BUILD dir: {}", build_dir);
            println!("  Expected relative: {}", expected_relative);
            
            let file = PathBuf::from(file_path);
            let build = PathBuf::from(build_dir);
            
            if let Ok(relative) = file.strip_prefix(&build) {
                println!("  Actual relative: {}", relative.display());
            } else {
                println!("  Failed to compute relative path");
            }
        }
    }
    
    #[test] 
    fn test_glob_patterns_detailed() {
        let _finder = BazelTargetFinder::new().unwrap();
        
        // Test patterns that might be used in BUILD files
        let patterns = vec![
            ("tests/*.rs", "tests/just_test.rs", true),
            ("tests/*.rs", "tests/subfolder/test.rs", false),
            ("tests/**/*.rs", "tests/just_test.rs", true),
            ("tests/**/*.rs", "tests/subfolder/test.rs", true),
            ("**/*.rs", "tests/just_test.rs", true),
            ("**/*.rs", "server/tests/just_test.rs", true),
        ];
        
        println!("\nGlob pattern matching tests:");
        for (pattern, file_path, expected) in patterns {
            // Use the private method through a workaround
            // In real code, we'd make this testable differently
            println!("  Pattern: {} vs File: {} => Expected: {}", pattern, file_path, expected);
        }
    }
}