//! Configuration loader for v2 configs
//! 
//! This module provides the loader for v2 configuration files.

use std::path::{Path, PathBuf};
use crate::error::Result;
use super::{Config as V2Config, JsonConfig};

/// V2 configuration loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load v2 configuration from the current directory or environment
    pub fn load() -> Result<V2Config> {
        // Try to load from current directory
        if let Ok(cwd) = std::env::current_dir() {
            Self::load_from_path(&cwd)
        } else {
            // Return empty v2 config if we can't get current directory
            Ok(V2Config::new(vec![]))
        }
    }
    
    /// Load v2 configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<V2Config> {
        // Get PROJECT_ROOT and home directory for boundaries
        let project_root = std::env::var("PROJECT_ROOT").ok().map(PathBuf::from);
        let home_dir = std::env::var("HOME").ok().map(PathBuf::from);
        
        // First, try to load from PROJECT_ROOT if set
        // This is the primary config with linked_projects
        let root_config = if let Some(ref root) = project_root {
            Self::try_load_v2(root)?
        } else {
            None
        };
        
        // Walk up directory tree looking for config files
        let mut check_path = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        
        let mut configs = Vec::new();
        
        // If we have a root config, add it first (lowest priority)
        if let Some(config) = root_config {
            tracing::debug!("Adding root config from PROJECT_ROOT");
            configs.push(config);
        }
        
        loop {
            // Check for v2 config at current level
            if let Some(v2_config) = Self::try_load_v2(check_path)? {
                tracing::debug!("Found v2 config at: {:?}", check_path);
                configs.push(v2_config);
            } else {
                tracing::debug!("No v2 config at: {:?}", check_path);
            }
            
            // Stop at PROJECT_ROOT or HOME directory
            if let Some(ref root) = project_root {
                if check_path == root {
                    break;
                }
            } else if let Some(ref home) = home_dir {
                if check_path == home {
                    break;
                }
            }
            
            // Check parent directory
            match check_path.parent() {
                Some(parent) => check_path = parent,
                None => break,
            }
        }
        
        // Merge configs (later configs override earlier ones)
        if configs.is_empty() {
            tracing::debug!("No v2 configs found, using default with detected build system");
            // No configs found - detect build system based on the path
            Ok(V2Config::default_with_detected_build_system(path))
        } else {
            tracing::debug!("Found {} v2 configs to merge", configs.len());
            // Start with the first config (least specific)
            let mut iter = configs.into_iter();
            let mut merged = iter.next().unwrap();
            
            // Merge in remaining configs in order of specificity
            for config in iter {
                tracing::debug!("Merging config with {} layers", config.layers_count());
                merged.merge(config);
            }
            
            tracing::debug!("Final merged config has {} layers", merged.layers_count());
            Ok(merged)
        }
    }
    
    /// Try to load v2 config from a directory
    fn try_load_v2(dir: &Path) -> Result<Option<V2Config>> {
        let paths = [
            dir.join(".cargo-runner-v2.json"),
            dir.join("cargo-runner-v2.json"),
        ];
        
        for path in &paths {
            if path.exists() {
                tracing::debug!("Found v2 config at {:?}", path);
                let contents = std::fs::read_to_string(path)?;
                let json_config = JsonConfig::from_json(&contents)
                    .map_err(|e| crate::error::Error::Other(format!("Failed to parse v2 config: {}", e)))?;
                return Ok(Some(json_config.to_config()));
            }
        }
        
        Ok(None)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_load_v2_config() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();
        
        let v2_json = r#"{
            "workspace": {
                "build_system": "Cargo",
                "frameworks": {
                    "test": "cargo-test"
                }
            }
        }"#;
        
        fs::write(dir_path.join(".cargo-runner-v2.json"), v2_json).unwrap();
        
        let config = ConfigLoader::load_from_path(dir_path).unwrap();
        // The config should have been loaded successfully
        assert_eq!(config.layers.len(), 1); // workspace layer
    }
    
    #[test]
    fn test_empty_config() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();
        
        // No config file exists
        let config = ConfigLoader::load_from_path(dir_path).unwrap();
        // Should return empty config
        assert_eq!(config.layers.len(), 0);
    }
}