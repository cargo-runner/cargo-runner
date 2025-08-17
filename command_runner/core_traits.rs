//! Core traits for the Universal Command Runner Framework
//! 
//! These traits define the interface that all language plugins must implement.
//! They are designed to be language-agnostic while providing enough structure
//! for consistent behavior across different programming languages.

use std::path::{Path, PathBuf};
use std::collections::HashMap;

// ============================================================================
// Core Data Types (Language-Agnostic)
// ============================================================================

/// Position in source code (line and column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// A range in source code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRange {
    pub start: Position,
    pub end: Position,
}

/// A runnable item in any language
#[derive(Debug, Clone)]
pub struct Runnable {
    /// Human-readable label (e.g., "test_addition", "main", "bench_sort")
    pub label: String,
    
    /// What kind of runnable this is
    pub kind: RunnableKind,
    
    /// Where in the source code this runnable is
    pub range: SourceRange,
    
    /// Language-specific metadata
    pub metadata: HashMap<String, String>,
}

/// Universal runnable kinds that apply to most languages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnableKind {
    /// Unit test (test, it, describe, etc.)
    Test { name: String },
    
    /// Benchmark/performance test
    Benchmark { name: String },
    
    /// Main entry point or script
    Main,
    
    /// Example/demo code
    Example { name: String },
    
    /// Documentation example
    DocExample { context: String },
    
    /// Language-specific kind
    Custom { type_name: String, data: String },
}

/// Information about a project
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// Root directory of the project
    pub root: PathBuf,
    
    /// Project name
    pub name: String,
    
    /// Detected build system
    pub build_system: BuildSystem,
    
    /// Project dependencies
    pub dependencies: Vec<String>,
    
    /// Language-specific project data
    pub metadata: HashMap<String, String>,
}

/// Build system information
#[derive(Debug, Clone)]
pub struct BuildSystem {
    /// Name of the build system (cargo, npm, pip, etc.)
    pub name: String,
    
    /// Version if available
    pub version: Option<String>,
    
    /// Config file that identified this build system
    pub config_file: PathBuf,
}

/// A command to be executed
#[derive(Debug, Clone)]
pub struct Command {
    /// The program to run (cargo, npm, python, etc.)
    pub program: String,
    
    /// Arguments to pass
    pub args: Vec<String>,
    
    /// Environment variables
    pub env: HashMap<String, String>,
    
    /// Working directory
    pub working_dir: Option<PathBuf>,
}

/// Context provided to plugins
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// The file being analyzed
    pub file_path: PathBuf,
    
    /// Source code content
    pub source_code: String,
    
    /// Target line number (if running at specific line)
    pub target_line: Option<u32>,
    
    /// Project information (if detected)
    pub project: Option<ProjectInfo>,
    
    /// User configuration
    pub config: PluginConfig,
}

/// Plugin-specific configuration
#[derive(Debug, Clone, Default)]
pub struct PluginConfig {
    /// Key-value configuration
    pub settings: HashMap<String, String>,
    
    /// Feature flags
    pub features: HashMap<String, bool>,
}

// ============================================================================
// Core Traits
// ============================================================================

/// Main trait that every language plugin must implement
pub trait LanguageRunner: Send + Sync {
    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Check if this plugin can handle the given file
    fn can_handle(&self, file_path: &Path) -> bool;
    
    /// Detect project information from a path
    fn detect_project(&self, path: &Path) -> Option<ProjectInfo>;
    
    /// Parse source and detect all runnables
    fn detect_runnables(&self, context: &ExecutionContext) -> Vec<Runnable>;
    
    /// Build a command for a specific runnable
    fn build_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command;
    
    /// Validate that required tools are installed
    fn validate_environment(&self) -> Result<(), String>;
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin name (e.g., "rust-runner", "node-runner")
    pub name: String,
    
    /// Plugin version
    pub version: String,
    
    /// Language this plugin handles
    pub language: String,
    
    /// File extensions this plugin handles
    pub file_extensions: Vec<String>,
    
    /// Author information
    pub author: Option<String>,
    
    /// Plugin description
    pub description: Option<String>,
    
    /// Supported features
    pub capabilities: PluginCapabilities,
}

/// What a plugin can do
#[derive(Debug, Clone, Default)]
pub struct PluginCapabilities {
    /// Can parse AST
    pub parse_ast: bool,
    
    /// Can detect tests
    pub detect_tests: bool,
    
    /// Can detect benchmarks
    pub detect_benchmarks: bool,
    
    /// Can detect main/binaries
    pub detect_binaries: bool,
    
    /// Can detect examples
    pub detect_examples: bool,
    
    /// Supports incremental parsing
    pub incremental_parsing: bool,
    
    /// Supports language server protocol
    pub lsp_support: bool,
}

// ============================================================================
// Optional Advanced Traits
// ============================================================================

/// For plugins that support AST parsing
pub trait AstParser {
    /// Parse source into an AST representation
    fn parse_ast(&self, source: &str) -> Result<Box<dyn AstNode>, String>;
    
    /// Get syntax errors
    fn get_syntax_errors(&self, source: &str) -> Vec<SyntaxError>;
}

/// Generic AST node (language-agnostic)
pub trait AstNode: std::fmt::Debug {
    /// Node type (function, class, module, etc.)
    fn node_type(&self) -> &str;
    
    /// Node name if applicable
    fn name(&self) -> Option<&str>;
    
    /// Source range
    fn range(&self) -> SourceRange;
    
    /// Child nodes
    fn children(&self) -> Vec<Box<dyn AstNode>>;
    
    /// Parent node
    fn parent(&self) -> Option<Box<dyn AstNode>>;
    
    /// Language-specific data as JSON
    fn metadata(&self) -> String;
}

/// Syntax error information
#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub message: String,
    pub range: SourceRange,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// For plugins that support code navigation
pub trait CodeNavigator {
    /// Find definition of symbol at position
    fn find_definition(&self, source: &str, position: Position) -> Option<SourceRange>;
    
    /// Find all references to symbol at position
    fn find_references(&self, source: &str, position: Position) -> Vec<SourceRange>;
    
    /// Get hover information at position
    fn get_hover_info(&self, source: &str, position: Position) -> Option<String>;
}

/// For plugins that support debugging
pub trait DebugSupport {
    /// Get debug command for a runnable
    fn build_debug_command(&self, runnable: &Runnable, context: &ExecutionContext) -> Command;
    
    /// Get valid breakpoint locations
    fn get_breakpoint_locations(&self, source: &str) -> Vec<Position>;
}

// ============================================================================
// Plugin Lifecycle
// ============================================================================

/// Plugin lifecycle hooks
pub trait PluginLifecycle {
    /// Called when plugin is loaded
    fn on_load(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Called before plugin is unloaded
    fn on_unload(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Called to update configuration
    fn configure(&mut self, config: PluginConfig) -> Result<(), String> {
        Ok(())
    }
    
    /// Health check
    fn health_check(&self) -> Result<(), String> {
        Ok(())
    }
}

// ============================================================================
// Error Handling
// ============================================================================

pub type Result<T> = std::result::Result<T, RunnerError>;

#[derive(Debug)]
pub enum RunnerError {
    /// Plugin doesn't support this file type
    UnsupportedFile(String),
    
    /// Failed to parse source code
    ParseError(String),
    
    /// No runnable found
    NoRunnableFound,
    
    /// Build system not found
    NoBuildSystem,
    
    /// Required tool not installed
    ToolNotFound(String),
    
    /// Configuration error
    ConfigError(String),
    
    /// Generic error
    Other(String),
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFile(msg) => write!(f, "Unsupported file: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::NoRunnableFound => write!(f, "No runnable found at this location"),
            Self::NoBuildSystem => write!(f, "No build system detected"),
            Self::ToolNotFound(tool) => write!(f, "Required tool not found: {}", tool),
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RunnerError {}