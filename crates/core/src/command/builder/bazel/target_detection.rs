//! Bazel target detection utilities

use std::fs;
use std::path::Path;

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
            if trimmed.starts_with("rust_test(")
                || (trimmed == "rust_test" && content.contains("name"))
            {
                if let Some(name) = extract_target_name(&content, line) {
                    targets.push(BazelTarget {
                        name,
                        kind: BazelTargetKind::RustTest,
                    });
                }
            }
            // Check for rust_binary
            else if trimmed.starts_with("rust_binary(")
                || (trimmed == "rust_binary" && content.contains("name"))
            {
                if let Some(name) = extract_target_name(&content, line) {
                    targets.push(BazelTarget {
                        name,
                        kind: BazelTargetKind::RustBinary,
                    });
                }
            }
            // Check for rust_library
            else if trimmed.starts_with("rust_library(")
                || (trimmed == "rust_library" && content.contains("name"))
            {
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
                    return Some(after_eq[1..1 + end_quote].to_string());
                }
            } else if after_eq.starts_with('\'') {
                // Handle single quotes
                if let Some(end_quote) = after_eq[1..].find('\'') {
                    return Some(after_eq[1..1 + end_quote].to_string());
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
