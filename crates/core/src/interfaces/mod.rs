//! Core interfaces for composable architecture
//! 
//! This module defines the trait interfaces that enable a plugin-based
//! architecture suitable for WASM and other extensibility mechanisms.

use std::path::PathBuf;
use crate::{
    command::CargoCommand,
    types::{Runnable, Scope, ExtendedScope},
    config::v2::{LayerConfig, FrameworkKind},
};

pub mod path_resolver;
pub mod module_resolver;
pub mod runnable_detector;
pub mod target_selection;
pub mod plugin;

pub use path_resolver::PathResolver;
pub use module_resolver::ModuleResolver;
pub use runnable_detector::RunnableDetector;
pub use target_selection::TargetSelection;
pub use plugin::{WindrunnerPlugin, PluginMetadata};

/// Core execution context provided by the host to plugins
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Environment Information
    pub project_root: PathBuf,
    pub working_directory: PathBuf,
    pub file_path: PathBuf,
    pub line_number: Option<u32>,
    
    /// Project Metadata
    pub package_name: Option<String>,
    pub crate_name: Option<String>,
    pub build_system: crate::build_system::BuildSystem,
    pub linked_projects: Vec<String>,
    
    /// File Content (pre-read by host)
    pub source_code: String,
    pub file_type: crate::types::FileType,
    
    /// Resolved Information (from host's parsers)
    pub scopes: Vec<Scope>,
    pub extended_scopes: Vec<ExtendedScope>,
    pub module_path: String,
}

/// Request sent to plugins for command building
#[derive(Debug, Clone)]
pub struct PluginRequest {
    /// Execution context from host
    pub context: ExecutionContext,
    
    /// All detected runnables in the file
    pub detected_runnables: Vec<Runnable>,
    
    /// The specific runnable to build a command for
    pub target_runnable: Option<Runnable>,
    
    /// Configuration layers that apply to this context
    pub config_layers: Vec<LayerConfig>,
}

/// Response from plugins after command building
#[derive(Debug, Clone)]
pub struct PluginResponse {
    /// The built command
    pub command: CargoCommand,
    
    /// Additional metadata about the command
    pub metadata: CommandMetadata,
}

/// Metadata about a built command
#[derive(Debug, Clone, Default)]
pub struct CommandMetadata {
    /// Strategy used to build the command
    pub strategy: Option<String>,
    
    /// Framework kind for the command
    pub framework: Option<FrameworkKind>,
    
    /// Whether the command was modified by configuration
    pub config_applied: bool,
    
    /// Additional notes or warnings
    pub notes: Vec<String>,
}