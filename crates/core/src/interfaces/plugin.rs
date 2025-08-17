//! Plugin interface for windrunner
//! 
//! Defines the main plugin trait and metadata structures.

use crate::{
    command::CargoCommand,
    error::Result,
    config::v2::FrameworkKind,
};
use super::{ExecutionContext, PluginRequest, PluginResponse};

/// Metadata about a plugin
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,
    
    /// Plugin version
    pub version: String,
    
    /// Plugin author
    pub author: Option<String>,
    
    /// Plugin description
    pub description: Option<String>,
    
    /// Supported framework kinds
    pub supported_frameworks: Vec<FrameworkKind>,
    
    /// Supported build systems
    pub supported_build_systems: Vec<crate::build_system::BuildSystem>,
    
    /// Whether this plugin can run in WASM
    pub wasm_compatible: bool,
}

/// Main plugin interface
pub trait WindrunnerPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Check if this plugin can handle the given request
    fn can_handle(&self, request: &PluginRequest) -> bool;
    
    /// Build a command for the given request
    fn build_command(&self, request: &PluginRequest) -> Result<PluginResponse>;
    
    /// Pre-process hook (optional)
    /// Called before the main command building
    fn pre_process(&self, _context: &mut ExecutionContext) -> Result<()> {
        Ok(())
    }
    
    /// Post-process hook (optional)
    /// Called after command building to allow modifications
    fn post_process(&self, _command: &mut CargoCommand) -> Result<()> {
        Ok(())
    }
    
    /// Validate the plugin's configuration (optional)
    fn validate_config(&self, _config: &crate::config::v2::LayerConfig) -> Result<()> {
        Ok(())
    }
}