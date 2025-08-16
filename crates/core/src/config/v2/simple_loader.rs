//! Simplified v2-only config loader

use std::path::{Path, PathBuf};
use crate::error::Result;
use super::{Config, JsonConfig};

/// Load v2 configuration from the filesystem
pub fn load_v2_config() -> Result<Config> {
    // Look for config file
    if let Some(config_path) = find_config_file() {
        load_from_path(&config_path)
    } else {
        // Return empty config if no file found
        Ok(Config::new(vec![]))
    }
}

/// Load v2 config from a specific path
pub fn load_from_path(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::error::Error::Other(format!("Failed to read config: {}", e)))?;
    
    let json_config: JsonConfig = serde_json::from_str(&content)
        .map_err(|e| crate::error::Error::Other(format!("Failed to parse config: {}", e)))?;
    
    Ok(json_config.to_config())
}

/// Find v2 config file in the current directory or parent directories
fn find_config_file() -> Option<PathBuf> {
    let mut current_dir = std::env::current_dir().ok()?;
    
    loop {
        // Check for .cargo-runner-v2.json
        let v2_path = current_dir.join(".cargo-runner-v2.json");
        if v2_path.exists() {
            return Some(v2_path);
        }
        
        // Check for cargo-runner-v2.json
        let alt_v2_path = current_dir.join("cargo-runner-v2.json");
        if alt_v2_path.exists() {
            return Some(alt_v2_path);
        }
        
        // Move to parent directory
        if !current_dir.pop() {
            break;
        }
    }
    
    None
}