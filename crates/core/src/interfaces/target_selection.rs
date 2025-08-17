//! Target selection interface
//! 
//! Provides abstraction for selecting and building cargo target arguments.

use std::path::Path;
use crate::types::Runnable;
use crate::config::v2::target_detection::TargetType;

/// Trait for target selection and argument building
pub trait TargetSelection: Send + Sync {
    /// Select the appropriate target type for a runnable
    fn select_target(
        &self,
        runnable: &Runnable,
        file_path: &Path,
        package_name: Option<&str>,
    ) -> TargetType;
    
    /// Build cargo arguments for a target
    fn build_target_args(
        &self,
        target: &TargetType,
    ) -> Vec<String>;
    
    /// Check if a file is in a benchmark directory
    fn is_benchmark_file(&self, file_path: &Path) -> bool;
    
    /// Check if a file is in an examples directory
    fn is_example_file(&self, file_path: &Path) -> bool;
    
    /// Check if a file is a library file
    fn is_lib_file(&self, file_path: &Path) -> bool;
    
    /// Check if a file is a binary file
    fn is_bin_file(&self, file_path: &Path) -> bool;
    
    /// Extract target name from file path
    fn extract_target_name(&self, file_path: &Path) -> Option<String>;
}