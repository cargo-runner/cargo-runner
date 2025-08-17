//! Plugin Registry and Loader
//! 
//! Manages plugin discovery, loading, and routing requests to appropriate plugins.

use crate::core_traits::*;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Plugin manifest that describes a plugin's capabilities
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub language: String,
    pub file_extensions: Vec<String>,
    pub wasm_path: PathBuf,
    pub priority: i32, // Higher priority plugins are checked first
}

/// Manages all loaded plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<Vec<LoadedPlugin>>>,
    extension_map: Arc<RwLock<HashMap<String, Vec<String>>>>, // ext -> plugin names
}

struct LoadedPlugin {
    manifest: PluginManifest,
    instance: Box<dyn LanguageRunner>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            extension_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Discover plugins from standard locations
    pub fn discover_plugins(&mut self) -> Result<Vec<PluginManifest>, String> {
        let mut manifests = Vec::new();
        
        // Check standard plugin directories
        let plugin_dirs = vec![
            // System-wide plugins
            PathBuf::from("/usr/local/lib/command-runner/plugins"),
            // User plugins
            dirs::config_dir()
                .map(|d| d.join("command-runner/plugins"))
                .unwrap_or_default(),
            // Local project plugins
            PathBuf::from("./.command-runner/plugins"),
        ];
        
        for dir in plugin_dirs {
            if dir.exists() {
                manifests.extend(self.scan_directory(&dir)?);
            }
        }
        
        // Sort by priority
        manifests.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(manifests)
    }
    
    /// Scan a directory for plugin manifests
    fn scan_directory(&self, dir: &Path) -> Result<Vec<PluginManifest>, String> {
        let mut manifests = Vec::new();
        
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if path.is_dir() {
                // Look for plugin.toml in subdirectory
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    if let Ok(manifest) = self.load_manifest(&manifest_path) {
                        manifests.push(manifest);
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                // Direct manifest file
                if let Ok(manifest) = self.load_manifest(&path) {
                    manifests.push(manifest);
                }
            }
        }
        
        Ok(manifests)
    }
    
    /// Load a plugin manifest from a TOML file
    fn load_manifest(&self, path: &Path) -> Result<PluginManifest, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        
        // Parse TOML (simplified - real would use toml crate)
        let mut manifest = PluginManifest {
            name: String::new(),
            version: String::new(),
            language: String::new(),
            file_extensions: Vec::new(),
            wasm_path: path.parent().unwrap().join("plugin.wasm"),
            priority: 0,
        };
        
        // Simple parsing logic (real implementation would use toml crate)
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("name =") {
                manifest.name = Self::extract_string_value(line);
            } else if line.starts_with("version =") {
                manifest.version = Self::extract_string_value(line);
            } else if line.starts_with("language =") {
                manifest.language = Self::extract_string_value(line);
            } else if line.starts_with("file_extensions =") {
                manifest.file_extensions = Self::extract_array_value(line);
            } else if line.starts_with("priority =") {
                manifest.priority = Self::extract_number_value(line);
            }
        }
        
        Ok(manifest)
    }
    
    /// Load a plugin from its manifest
    pub fn load_plugin(&mut self, manifest: PluginManifest) -> Result<(), String> {
        // Check if plugin already loaded
        {
            let plugins = self.plugins.read().unwrap();
            if plugins.iter().any(|p| p.manifest.name == manifest.name) {
                return Ok(()); // Already loaded
            }
        }
        
        // Load the WASM module or native plugin
        let instance = if manifest.wasm_path.exists() {
            self.load_wasm_plugin(&manifest)?
        } else {
            self.load_native_plugin(&manifest)?
        };
        
        // Register file extensions
        {
            let mut ext_map = self.extension_map.write().unwrap();
            for ext in &manifest.file_extensions {
                ext_map.entry(ext.clone())
                    .or_insert_with(Vec::new)
                    .push(manifest.name.clone());
            }
        }
        
        // Add to loaded plugins
        {
            let mut plugins = self.plugins.write().unwrap();
            plugins.push(LoadedPlugin {
                manifest,
                instance,
            });
        }
        
        Ok(())
    }
    
    /// Load a WASM plugin
    fn load_wasm_plugin(&self, manifest: &PluginManifest) -> Result<Box<dyn LanguageRunner>, String> {
        // This would use wasmtime or wasmer to load the WASM module
        // For now, we'll create native instances based on plugin name
        match manifest.name.as_str() {
            "rust-runner" => {
                // Would load from WASM, but for demo use native
                use crate::plugins::rust_runner::RustRunner;
                Ok(Box::new(RustRunner::new()))
            }
            "node-runner" => {
                use crate::plugins::node_runner::NodeRunner;
                Ok(Box::new(NodeRunner::new()))
            }
            _ => Err(format!("Unknown plugin: {}", manifest.name))
        }
    }
    
    /// Load a native plugin (for development/debugging)
    fn load_native_plugin(&self, manifest: &PluginManifest) -> Result<Box<dyn LanguageRunner>, String> {
        // Load native plugins (same as WASM for now)
        self.load_wasm_plugin(manifest)
    }
    
    /// Find a plugin that can handle the given file
    pub fn find_plugin(&self, file_path: &Path) -> Option<Arc<dyn LanguageRunner>> {
        let extension = file_path.extension()
            .and_then(|s| s.to_str())?;
        
        // Check extension map for quick lookup
        let plugin_names = {
            let ext_map = self.extension_map.read().unwrap();
            ext_map.get(extension).cloned()
        };
        
        if let Some(names) = plugin_names {
            let plugins = self.plugins.read().unwrap();
            for name in names {
                if let Some(plugin) = plugins.iter().find(|p| p.manifest.name == name) {
                    // Double-check with can_handle
                    if plugin.instance.can_handle(file_path) {
                        // Create Arc wrapper for sharing
                        // In real implementation, would return Arc directly
                        return None; // Simplified
                    }
                }
            }
        }
        
        // Fallback: check all plugins
        let plugins = self.plugins.read().unwrap();
        for plugin in plugins.iter() {
            if plugin.instance.can_handle(file_path) {
                return None; // Simplified - would return Arc
            }
        }
        
        None
    }
    
    /// Get all loaded plugins
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().unwrap();
        plugins.iter()
            .map(|p| p.instance.metadata())
            .collect()
    }
    
    /// Reload all plugins
    pub fn reload(&mut self) -> Result<(), String> {
        // Clear current plugins
        {
            let mut plugins = self.plugins.write().unwrap();
            plugins.clear();
        }
        {
            let mut ext_map = self.extension_map.write().unwrap();
            ext_map.clear();
        }
        
        // Rediscover and load
        let manifests = self.discover_plugins()?;
        for manifest in manifests {
            self.load_plugin(manifest)?;
        }
        
        Ok(())
    }
    
    // Helper methods for simple TOML parsing
    fn extract_string_value(line: &str) -> String {
        line.split('=')
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string()
    }
    
    fn extract_array_value(line: &str) -> Vec<String> {
        line.split('=')
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_matches('[')
            .trim_matches(']')
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect()
    }
    
    fn extract_number_value(line: &str) -> i32 {
        line.split('=')
            .nth(1)
            .unwrap_or("0")
            .trim()
            .parse()
            .unwrap_or(0)
    }
}

/// Plugin loader with WASM sandbox support
pub struct PluginLoader {
    registry: PluginRegistry,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
        }
    }
    
    /// Initialize and load all plugins
    pub fn initialize(&mut self) -> Result<(), String> {
        // Discover available plugins
        let manifests = self.registry.discover_plugins()?;
        
        println!("Found {} plugins", manifests.len());
        
        // Load each plugin
        for manifest in manifests {
            println!("Loading plugin: {} v{}", manifest.name, manifest.version);
            if let Err(e) = self.registry.load_plugin(manifest) {
                eprintln!("Failed to load plugin: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Get the registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }
    
    /// Get mutable registry
    pub fn registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.registry
    }
}

// Optional: Use with directories crate for standard paths
mod dirs {
    use std::path::PathBuf;
    
    pub fn config_dir() -> Option<PathBuf> {
        // Simplified - real would use dirs crate
        home_dir().map(|h| h.join(".config"))
    }
    
    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}