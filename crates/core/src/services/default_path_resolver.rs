//! Default path resolver implementation
//!
//! Provides standard file system based path resolution.

use crate::interfaces::PathResolver;
use std::path::{Path, PathBuf};

/// Default implementation of PathResolver using std::fs
pub struct DefaultPathResolver;

impl DefaultPathResolver {
    pub fn new() -> Self {
        Self
    }
}

impl PathResolver for DefaultPathResolver {
    fn resolve_relative(&self, base: &Path, relative: &Path) -> PathBuf {
        if relative.is_absolute() {
            relative.to_path_buf()
        } else {
            base.join(relative)
        }
    }

    fn find_project_root(&self, from: &Path) -> Option<PathBuf> {
        let mut current = if from.is_file() {
            from.parent()?.to_path_buf()
        } else {
            from.to_path_buf()
        };

        loop {
            // Check for Cargo.toml
            if current.join("Cargo.toml").exists() {
                return Some(current);
            }

            // Check for BUILD.bazel or BUILD
            if current.join("BUILD.bazel").exists() || current.join("BUILD").exists() {
                return Some(current);
            }

            // Move to parent directory
            if !current.pop() {
                break;
            }
        }

        None
    }

    fn normalize(&self, path: &Path) -> PathBuf {
        // Simple normalization - in production, use path-clean or similar
        let mut components = Vec::new();
        for component in path.components() {
            use std::path::Component;
            match component {
                Component::ParentDir => {
                    components.pop();
                }
                Component::CurDir => {
                    // Skip
                }
                c => {
                    components.push(c);
                }
            }
        }

        components.iter().collect()
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn parent(&self, path: &Path) -> Option<PathBuf> {
        path.parent().map(|p| p.to_path_buf())
    }

    fn file_name(&self, path: &Path) -> Option<String> {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    fn file_stem(&self, path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }

    fn extension(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string())
    }
}
