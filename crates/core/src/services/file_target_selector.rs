//! File-based target selector implementation
//!
//! Implements target selection based on file paths and conventions.

use crate::{
    config::v2::target_detection::{TargetType, detect_target_from_path},
    interfaces::TargetSelection,
    types::Runnable,
};
use std::path::Path;

/// File-based implementation of TargetSelection
pub struct FileTargetSelector;

impl FileTargetSelector {
    pub fn new() -> Self {
        Self
    }
}

impl TargetSelection for FileTargetSelector {
    fn select_target(
        &self,
        _runnable: &Runnable,
        file_path: &Path,
        package_name: Option<&str>,
    ) -> TargetType {
        detect_target_from_path(&file_path.to_string_lossy(), package_name)
    }

    fn build_target_args(&self, target: &TargetType) -> Vec<String> {
        let mut args = Vec::new();
        target.add_to_args(&mut args);
        args
    }

    fn is_benchmark_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        crate::config::v2::target_detection::is_benchmark_file(&path_str)
    }

    fn is_example_file(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        crate::config::v2::target_detection::is_example_file(&path_str)
    }

    fn is_lib_file(&self, file_path: &Path) -> bool {
        file_path.ends_with("src/lib.rs")
    }

    fn is_bin_file(&self, file_path: &Path) -> bool {
        file_path.ends_with("src/main.rs") || file_path.components().any(|c| c.as_os_str() == "bin")
    }

    fn extract_target_name(&self, file_path: &Path) -> Option<String> {
        file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }
}
