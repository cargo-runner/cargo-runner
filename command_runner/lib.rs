//! Universal Command Runner Library
//! 
//! A language-agnostic framework for detecting and running code in any programming language.

pub mod core_traits;
pub mod plugin_registry;
pub mod main_runner;

// Re-export main types
pub use core_traits::*;
pub use plugin_registry::{PluginLoader, PluginRegistry, PluginManifest};
pub use main_runner::{CommandRunner, UniversalRunner};

/// Version of the command runner framework
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the command runner with default settings
pub fn init() -> Result<CommandRunner, String> {
    CommandRunner::new()
}

/// Initialize with custom plugin directory
pub fn init_with_plugins(plugin_dir: &std::path::Path) -> Result<CommandRunner, String> {
    let mut runner = CommandRunner::new()?;
    // In real implementation, would load plugins from directory
    Ok(runner)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}