//! Bazel target detection utilities

use std::fs;
use std::path::Path;

/// Target information extracted from BUILD.bazel
#[derive(Debug, Clone)]
pub struct BazelTarget {
    pub name: String,
    pub kind: BazelTargetKind,
    pub srcs: Vec<String>,
    pub crate_ref: Option<String>, // For rust_test, the crate it tests
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

    // Get the file path relative to the package directory
    let relative_file_path = file_path.strip_prefix(package_dir).ok()?;
    let relative_file_str = relative_file_path.to_str()?;

    // Find appropriate target
    if is_test {
        // For test files, we need to find which rust_library contains this file
        // and then find which rust_test references that library
        
        // First, find if any rust_library contains this file
        let mut library_name = None;
        for target in &targets {
            if target.kind == BazelTargetKind::RustLibrary {
                if target.srcs.contains(&relative_file_str.to_string()) {
                    library_name = Some(&target.name);
                    break;
                }
            }
        }
        
        // If we found a library, look for a test that references it
        if let Some(lib_name) = library_name {
            for target in &targets {
                if target.kind == BazelTargetKind::RustTest {
                    if let Some(crate_ref) = &target.crate_ref {
                        // Check if the crate reference matches the library
                        // Handle both :lib_name and lib_name formats
                        let crate_name = crate_ref.strip_prefix(':').unwrap_or(crate_ref);
                        if crate_name == lib_name {
                            let result = if package_path == "//" {
                                format!(":{}", target.name)
                            } else {
                                format!("{}:{}", package_path, target.name)
                            };
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        // Fall back to any rust_test target
        for target in &targets {
            if target.kind == BazelTargetKind::RustTest {
                let result = if package_path == "//" {
                    format!(":{}", target.name)
                } else {
                    format!("{}:{}", package_path, target.name)
                };
                return Some(result);
            }
        }
    } else {
        // Look for binary targets
        for target in &targets {
            if target.kind == BazelTargetKind::RustBinary {
                let result = if package_path == "//" {
                    format!(":{}", target.name)
                } else {
                    format!("{}:{}", package_path, target.name)
                };
                return Some(result);
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
        // Split into rule blocks
        let rule_blocks = extract_rule_blocks(&content);
        
        for block in rule_blocks {
            if let Some(rule_type) = get_rule_type(&block) {
                if let Some(name) = extract_attribute(&block, "name") {
                    let kind = match rule_type {
                        "rust_test" => BazelTargetKind::RustTest,
                        "rust_binary" => BazelTargetKind::RustBinary,
                        "rust_library" => BazelTargetKind::RustLibrary,
                        _ => continue,
                    };
                    
                    let srcs = extract_srcs(&block);
                    let crate_ref = extract_attribute(&block, "crate");
                    
                    targets.push(BazelTarget {
                        name,
                        kind,
                        srcs,
                        crate_ref,
                    });
                }
            }
        }
    }

    targets
}

/// Extract rule blocks from BUILD.bazel content
fn extract_rule_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = String::new();
    let mut in_rule = false;
    let mut paren_count = 0;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Check if we're starting a new rule
        if !in_rule && (trimmed.starts_with("rust_") || trimmed.starts_with("load(")) {
            in_rule = true;
            current_block.clear();
        }
        
        if in_rule {
            current_block.push_str(line);
            current_block.push('\n');
            
            // Count parentheses to determine when rule ends
            for ch in line.chars() {
                match ch {
                    '(' => paren_count += 1,
                    ')' => {
                        paren_count -= 1;
                        if paren_count == 0 {
                            in_rule = false;
                            blocks.push(current_block.clone());
                            current_block.clear();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    blocks
}

/// Get the rule type from a rule block
fn get_rule_type(block: &str) -> Option<&str> {
    let trimmed = block.trim_start();
    if trimmed.starts_with("rust_test") {
        Some("rust_test")
    } else if trimmed.starts_with("rust_binary") {
        Some("rust_binary")
    } else if trimmed.starts_with("rust_library") {
        Some("rust_library")
    } else {
        None
    }
}

/// Extract an attribute value from a rule block
fn extract_attribute(block: &str, attr_name: &str) -> Option<String> {
    // Look for attr_name = "value" or attr_name = :value
    let pattern = format!("{} =", attr_name);
    if let Some(pos) = block.find(&pattern) {
        let after_attr = &block[pos + pattern.len()..];
        let value_part = after_attr.trim_start();
        
        if value_part.starts_with('"') {
            // String value
            if let Some(end_pos) = value_part[1..].find('"') {
                return Some(value_part[1..1 + end_pos].to_string());
            }
        } else if value_part.starts_with('\'') {
            // Single quoted string
            if let Some(end_pos) = value_part[1..].find('\'') {
                return Some(value_part[1..1 + end_pos].to_string());
            }
        } else if value_part.starts_with(':') {
            // Label reference like :corex_lib
            let end_pos = value_part[1..]
                .find(|c: char| c == ',' || c == ')' || c.is_whitespace())
                .unwrap_or(value_part.len() - 1);
            return Some(value_part[0..1 + end_pos].to_string());
        }
    }
    None
}

/// Extract srcs list from a rule block
fn extract_srcs(block: &str) -> Vec<String> {
    let mut srcs = Vec::new();
    
    if let Some(srcs_pos) = block.find("srcs =") {
        let after_srcs = &block[srcs_pos + 6..];
        let trimmed = after_srcs.trim_start();
        
        if trimmed.starts_with('[') {
            // Find the closing bracket
            if let Some(end_pos) = trimmed.find(']') {
                let list_content = &trimmed[1..end_pos];
                // Extract quoted strings
                let mut current = 0;
                while current < list_content.len() {
                    if let Some(quote_start) = list_content[current..].find('"') {
                        let start = current + quote_start + 1;
                        if let Some(quote_end) = list_content[start..].find('"') {
                            srcs.push(list_content[start..start + quote_end].to_string());
                            current = start + quote_end + 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }
    
    srcs
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
        assert_eq!(targets[0].srcs, vec!["src/lib.rs"]);

        assert_eq!(targets[1].name, "corex_tests");
        assert_eq!(targets[1].kind, BazelTargetKind::RustTest);
        assert_eq!(targets[1].crate_ref, Some(":corex_lib".to_string()));
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
