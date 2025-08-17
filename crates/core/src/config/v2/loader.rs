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
        let mut found_linked_projects_at: Option<PathBuf> = None;
        
        // If we have a root config, add it first (lowest priority)
        if let Some(config) = root_config {
            tracing::debug!("Adding root config from PROJECT_ROOT");
            if config.linked_projects.is_some() {
                found_linked_projects_at = project_root.clone();
            }
            configs.push(config);
        }
        
        loop {
            // Check for v2 config at current level
            if let Some(v2_config) = Self::try_load_v2(check_path)? {
                tracing::debug!("Found v2 config at: {:?}, has linked_projects: {}", 
                    check_path, v2_config.linked_projects.is_some());
                
                // Track where we found linked_projects
                if v2_config.linked_projects.is_some() && found_linked_projects_at.is_none() {
                    found_linked_projects_at = Some(check_path.to_path_buf());
                    tracing::debug!("Found linked_projects at: {:?}", check_path);
                }
                
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
            
            // If we found linked_projects, stop here to avoid walking into parent workspaces
            if found_linked_projects_at == Some(check_path.to_path_buf()) {
                tracing::debug!("Stopping at workspace root with linked_projects");
                break;
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
            tracing::debug!("Starting with config that has linked_projects: {:?}", merged.linked_projects.is_some());
            
            // Merge in remaining configs in order of specificity
            for config in iter {
                tracing::debug!("Merging config with {} layers, has linked_projects: {:?}", 
                    config.layers_count(), config.linked_projects.is_some());
                merged.merge(config);
            }
            
            tracing::debug!("Final merged config has {} layers, linked_projects: {:?}", 
                merged.layers_count(), merged.linked_projects.is_some());
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
                let contents = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to read config file {:?}: {}", path, e);
                        return Err(e.into());
                    }
                };
                
                let json_config = match JsonConfig::from_json(&contents) {
                    Ok(jc) => jc,
                    Err(e) => {
                        tracing::error!("Failed to parse v2 config from {:?}: {}", path, e);
                        return Err(crate::error::Error::Other(format!("Failed to parse v2 config: {}", e)));
                    }
                };
                
                let config = json_config.to_config();
                tracing::debug!("Loaded config from {:?} with linked_projects: {:?}", 
                    path, config.linked_projects.is_some());
                return Ok(Some(config));
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
        // Should return default config with workspace layer
        assert_eq!(config.layers.len(), 1);
        assert!(matches!(config.layers[0].scope, crate::config::v2::Scope::Workspace));
    }
}