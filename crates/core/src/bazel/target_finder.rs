//! Find Bazel targets for source files

use std::path::{Path, PathBuf};
use std::fs;
use crate::error::Result;
use super::{StarlarkParser, RuleExtractor, TargetAnalyzer, BazelTarget, BazelTargetKind};

/// Finds Bazel targets for source files
pub struct BazelTargetFinder {
    parser: StarlarkParser,
    analyzer: TargetAnalyzer,
}

impl BazelTargetFinder {
    /// Create a new target finder
    pub fn new() -> Result<Self> {
        Ok(Self {
            parser: StarlarkParser::new()?,
            analyzer: TargetAnalyzer::new(),
        })
    }
    
    /// Find all targets in a BUILD file
    pub fn find_targets_in_build_file(&mut self, build_file: &Path) -> Result<Vec<BazelTarget>> {
        tracing::debug!("find_targets_in_build_file: {:?}", build_file);
        
        let content = fs::read_to_string(build_file)
            .map_err(|e| crate::error::Error::IoError(e))?;
        
        let ast = self.parser.parse_build_file(&content)?;
        let rules = RuleExtractor::extract_rules(&ast)?;
        tracing::debug!("Extracted {} rules from BUILD file", rules.len());
        
        // Get the package path from the BUILD file location
        let package_path = self.get_package_path(build_file);
        tracing::debug!("Package path: {}", package_path);
        
        // Analyze all rules
        let mut targets = Vec::new();
        for rule in &rules {
            tracing::debug!("Analyzing rule: {} (type: {})", rule.name, rule.rule_type);
            if let Some(mut target) = self.analyzer.analyze_rule(&rule) {
                // Update the label with the full package path
                target.label = format!("{}:{}", package_path, target.name);
                tracing::debug!("Created target: {} ({:?})", target.label, target.kind);
                targets.push(target);
            }
        }
        
        tracing::debug!("Found {} targets in BUILD file", targets.len());
        Ok(targets)
    }
    
    /// Find targets that include a specific source file
    pub fn find_targets_for_file(
        &mut self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Vec<BazelTarget>> {
        tracing::debug!("find_targets_for_file: file_path={:?}, workspace_root={:?}", file_path, workspace_root);
        
        // Find the BUILD file for this source file
        let build_file = self.find_build_file(file_path, workspace_root)?;
        tracing::debug!("Using BUILD file: {:?}", build_file);
        
        // Get all targets from the BUILD file
        let all_targets = self.find_targets_in_build_file(&build_file)?;
        tracing::debug!("Found {} total targets in BUILD file", all_targets.len());
        
        // Get the relative path from the BUILD file directory
        let build_dir = build_file.parent()
            .ok_or_else(|| crate::error::Error::ParseError("Invalid BUILD file path".to_string()))?;
        tracing::debug!("BUILD directory: {:?}", build_dir);
        
        let relative_path = file_path.strip_prefix(build_dir)
            .map_err(|e| {
                tracing::debug!("Failed to strip prefix: file_path={:?}, build_dir={:?}, error={:?}", 
                             file_path, build_dir, e);
                crate::error::Error::ParseError("File not under BUILD directory".to_string())
            })?;
        let relative_str = relative_path.to_str()
            .ok_or_else(|| crate::error::Error::ParseError("Invalid file path".to_string()))?;
        tracing::debug!("Relative path from BUILD: {}", relative_str);
        
        // Filter targets that include this source file
        let mut matching_targets = Vec::new();
        
        // First, find targets that directly include this file
        let mut library_name = None;
        for target in &all_targets {
            if self.target_includes_file(&target, relative_str) {
                matching_targets.push(target.clone());
                
                // If this is a library, remember its name
                if matches!(target.kind, BazelTargetKind::Library) {
                    library_name = Some(target.name.clone());
                }
            }
        }
        
        // Second, find targets that reference the library containing this file
        if let Some(lib_name) = library_name {
            for target in &all_targets {
                // Check if this target references the library
                if let Some(crate_ref) = &target.attributes.crate_ref {
                    let crate_name = crate_ref.strip_prefix(':').unwrap_or(crate_ref);
                    if crate_name == lib_name && !matching_targets.iter().any(|t| t.name == target.name) {
                        matching_targets.push(target.clone());
                    }
                }
            }
        }
        
        Ok(matching_targets)
    }
    
    /// Find a runnable target for a file
    pub fn find_runnable_target(
        &mut self,
        file_path: &Path,
        workspace_root: &Path,
        kind_filter: Option<BazelTargetKind>,
    ) -> Result<Option<BazelTarget>> {
        let targets = self.find_targets_for_file(file_path, workspace_root)?;
        
        // Special case: if looking for tests, check if there's a rust_test that references
        // a binary/library containing this file
        if let Some(BazelTargetKind::Test) = kind_filter {
            // Find any binary or library target that contains this file
            let bin_or_lib = targets.iter().find(|t| 
                matches!(t.kind, BazelTargetKind::Binary | BazelTargetKind::Library)
            );
            
            if let Some(bin_or_lib_target) = bin_or_lib {
                tracing::debug!("Found binary/library target {} containing file", bin_or_lib_target.name);
                
                // Now find rust_test targets that reference this binary/library
                let build_file = self.find_build_file(file_path, workspace_root)?;
                let all_targets = self.find_targets_in_build_file(&build_file)?;
                
                for target in all_targets {
                    if matches!(target.kind, BazelTargetKind::Test) {
                        // Check if this test references our binary/library
                        if let Some(crate_ref) = &target.attributes.crate_ref {
                            let crate_name = crate_ref.strip_prefix(':').unwrap_or(crate_ref);
                            if crate_name == bin_or_lib_target.name {
                                tracing::debug!("Found rust_test target {} that tests {}", 
                                             target.label, bin_or_lib_target.name);
                                return Ok(Some(target));
                            }
                        }
                    }
                }
            }
        }
        
        // Filter to runnable targets
        let runnable_targets: Vec<_> = targets.into_iter()
            .filter(|t| t.kind.is_runnable())
            .filter(|t| {
                if let Some(ref kind) = kind_filter {
                    &t.kind == kind
                } else {
                    true
                }
            })
            .collect();
        
        // Prioritize targets by specificity
        if let Some(target) = self.find_most_specific_target(runnable_targets, file_path) {
            Ok(Some(target))
        } else {
            Ok(None)
        }
    }
    
    /// Find integration test target for a file
    pub fn find_integration_test_target(
        &mut self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Option<BazelTarget>> {
        tracing::debug!("find_integration_test_target: file_path={:?}, workspace_root={:?}", 
                      file_path, workspace_root);
        
        // Integration tests are typically in tests/ directory
        if !file_path.components().any(|c| c.as_os_str() == "tests") {
            tracing::debug!("File is not in a tests/ directory");
            return Ok(None);
        }
        
        let targets = self.find_targets_for_file(file_path, workspace_root)?;
        tracing::debug!("Found {} targets for file", targets.len());
        
        // Log all targets found
        for target in &targets {
            tracing::debug!("  Target: {} ({:?}) - label: {}", target.name, target.kind, target.label);
        }
        
        let result = targets
            .into_iter()
            .find(|t| matches!(t.kind, BazelTargetKind::TestSuite));
            
        if let Some(ref target) = result {
            tracing::debug!("Found integration test target: {}", target.label);
        } else {
            tracing::debug!("No TestSuite target found among the targets");
        }
        
        Ok(result)
    }
    
    /// Find doc test target for a library file
    pub fn find_doc_test_target(
        &mut self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Option<BazelTarget>> {
        let targets = self.find_targets_for_file(file_path, workspace_root)?;
        
        // First find the library that contains this file
        let library_target = targets.iter()
            .find(|t| matches!(t.kind, BazelTargetKind::Library));
        
        if let Some(lib) = library_target {
            // Find doc test target that references this library
            let build_file = self.find_build_file(file_path, workspace_root)?;
            let all_targets = self.find_targets_in_build_file(&build_file)?;
            
            Ok(all_targets.into_iter()
                .find(|t| {
                    if matches!(t.kind, BazelTargetKind::DocTest) {
                        if let Some(crate_ref) = &t.attributes.crate_ref {
                            let crate_name = crate_ref.strip_prefix(':').unwrap_or(crate_ref);
                            return crate_name == lib.name;
                        }
                    }
                    false
                }))
        } else {
            Ok(None)
        }
    }
    
    /// Find the BUILD file for a source file
    fn find_build_file(&self, file_path: &Path, workspace_root: &Path) -> Result<PathBuf> {
        let mut current_dir = file_path.parent()
            .ok_or_else(|| crate::error::Error::ParseError("Invalid file path".to_string()))?;
        
        tracing::debug!("find_build_file: starting from {:?}, workspace_root={:?}", current_dir, workspace_root);
        
        loop {
            let build_bazel = current_dir.join("BUILD.bazel");
            if build_bazel.exists() {
                tracing::debug!("Found BUILD.bazel at {:?}", build_bazel);
                return Ok(build_bazel);
            }
            
            let build = current_dir.join("BUILD");
            if build.exists() {
                tracing::debug!("Found BUILD at {:?}", build);
                return Ok(build);
            }
            
            // Stop at workspace root
            if current_dir == workspace_root {
                tracing::debug!("Reached workspace root without finding BUILD file");
                return Err(crate::error::Error::ParseError(
                    "No BUILD file found".to_string()
                ));
            }
            
            // Go up one directory
            current_dir = current_dir.parent()
                .ok_or_else(|| crate::error::Error::ParseError("Reached filesystem root".to_string()))?;
            tracing::debug!("Moving up to {:?}", current_dir);
        }
    }
    
    /// Get the Bazel package path from a BUILD file location
    fn get_package_path(&self, build_file: &Path) -> String {
        // Find workspace root by looking for MODULE.bazel
        let mut workspace_root = None;
        let mut current = build_file.parent();
        
        while let Some(dir) = current {
            if dir.join("MODULE.bazel").exists() || dir.join("WORKSPACE").exists() {
                workspace_root = Some(dir);
                break;
            }
            current = dir.parent();
        }
        
        if let Some(root) = workspace_root {
            if let Some(package_dir) = build_file.parent() {
                if let Ok(rel_path) = package_dir.strip_prefix(root) {
                    if rel_path.as_os_str().is_empty() {
                        return "//".to_string();
                    } else {
                        return format!("//{}", rel_path.display());
                    }
                }
            }
        }
        
        // Fallback
        "//".to_string()
    }
    
    /// Check if a target includes a specific file
    fn target_includes_file(&self, target: &BazelTarget, file_path: &str) -> bool {
        tracing::debug!("target_includes_file: checking if target {} includes file {}", target.name, file_path);
        tracing::debug!("  Target sources: {:?}", target.sources);
        
        // Check direct source files
        if target.sources.iter().any(|src| src == file_path) {
            tracing::debug!("  Found direct match");
            return true;
        }
        
        // Check glob patterns
        for src in &target.sources {
            if src.contains('*') && self.matches_glob_pattern(file_path, src) {
                tracing::debug!("  Matched glob pattern: {}", src);
                return true;
            }
        }
        
        tracing::debug!("  No match found");
        false
    }
    
    /// Simple glob pattern matching
    fn matches_glob_pattern(&self, file_path: &str, pattern: &str) -> bool {
        tracing::debug!("matches_glob_pattern: file_path={}, pattern={}", file_path, pattern);
        
        // Handle tests/** pattern (matches all files under tests/ at any depth)
        if pattern == "tests/**" {
            let result = file_path.starts_with("tests/");
            tracing::debug!("  Pattern 'tests/**' => {}", result);
            return result;
        }
        
        // Handle **/*.rs pattern (matches any .rs file at any depth)
        if pattern == "**/*.rs" {
            return file_path.ends_with(".rs");
        }
        
        // Handle *.rs pattern (matches .rs files in current directory only)
        if pattern == "*.rs" {
            return file_path.ends_with(".rs") && !file_path.contains('/');
        }
        
        // Handle patterns like tests/*.rs
        if pattern.contains("/*.") {
            let parts: Vec<&str> = pattern.split("/*.").collect();
            if parts.len() == 2 {
                let dir_pattern = parts[0];
                let extension = parts[1];
                
                // Check if file is in the specified directory and has the right extension
                if file_path.starts_with(&format!("{}/", dir_pattern)) && 
                   file_path.ends_with(&format!(".{}", extension)) {
                    // Make sure it's a direct child (no subdirectories)
                    let after_dir = &file_path[dir_pattern.len() + 1..];
                    return !after_dir[..after_dir.len() - extension.len() - 1].contains('/');
                }
            }
        }
        
        // Handle patterns like tests/**/*.rs (any depth under tests/)
        if pattern.contains("/**/") {
            let parts: Vec<&str> = pattern.split("/**/").collect();
            if parts.len() == 2 {
                let dir_pattern = parts[0];
                let file_pattern = parts[1];
                
                if file_path.starts_with(&format!("{}/", dir_pattern)) {
                    if file_pattern == "*.rs" {
                        return file_path.ends_with(".rs");
                    }
                }
            }
        }
        
        // Handle dir/** patterns (matches everything under dir/)
        if pattern.ends_with("/**") {
            let dir = &pattern[..pattern.len() - 3];
            let result = file_path.starts_with(&format!("{}/", dir));
            tracing::debug!("  Pattern '{}/**' => {}", dir, result);
            return result;
        }
        
        false
    }
    
    /// Find the most specific target for a file
    fn find_most_specific_target(&self, targets: Vec<BazelTarget>, _file_path: &Path) -> Option<BazelTarget> {
        // Prioritize by target kind
        let priority_order = [
            BazelTargetKind::Test,
            BazelTargetKind::TestSuite,
            BazelTargetKind::DocTest,
            BazelTargetKind::Benchmark,
            BazelTargetKind::Binary,
        ];
        
        for kind in &priority_order {
            if let Some(target) = targets.iter().find(|t| &t.kind == kind) {
                return Some(target.clone());
            }
        }
        
        targets.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_find_targets_in_build_file() {
        let temp_dir = TempDir::new().unwrap();
        let build_file = temp_dir.path().join("BUILD.bazel");
        
        let content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "mylib",
    srcs = ["src/lib.rs"],
)

rust_test(
    name = "mylib_test",
    crate = ":mylib",
)
"#;
        
        fs::write(&build_file, content).unwrap();
        
        let mut finder = BazelTargetFinder::new().unwrap();
        let targets = finder.find_targets_in_build_file(&build_file).unwrap();
        
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "mylib");
        assert_eq!(targets[0].kind, BazelTargetKind::Library);
        assert_eq!(targets[1].name, "mylib_test");
        assert_eq!(targets[1].kind, BazelTargetKind::Test);
    }
}