//! Path resolution interface
//! 
//! Provides abstraction for path operations to enable WASM compatibility
//! where direct file system access may not be available.

use std::path::{Path, PathBuf};

/// Trait for path resolution operations
pub trait PathResolver: Send + Sync {
    /// Resolve a relative path against a base path
    fn resolve_relative(&self, base: &Path, relative: &Path) -> PathBuf;
    
    /// Find the project root from a given path
    /// (looks for Cargo.toml, BUILD.bazel, etc.)
    fn find_project_root(&self, from: &Path) -> Option<PathBuf>;
    
    /// Normalize a path (remove .., ., etc.)
    fn normalize(&self, path: &Path) -> PathBuf;
    
    /// Check if a path exists (may return false in WASM)
    fn exists(&self, path: &Path) -> bool;
    
    /// Check if a path is a file
    fn is_file(&self, path: &Path) -> bool;
    
    /// Check if a path is a directory
    fn is_dir(&self, path: &Path) -> bool;
    
    /// Get the parent directory of a path
    fn parent(&self, path: &Path) -> Option<PathBuf>;
    
    /// Get the file name from a path
    fn file_name(&self, path: &Path) -> Option<String>;
    
    /// Get the file stem (name without extension)
    fn file_stem(&self, path: &Path) -> Option<String>;
    
    /// Get the file extension
    fn extension(&self, path: &Path) -> Option<String>;
}