//! Universal Command Runner Library
//! 
//! A language-agnostic framework for detecting and running code in any programming language.

pub mod core_traits;
pub mod plugin_registry;
pub mod main_runner;
pub mod plugins;

// Re-export main types
pub use core_traits::*;
pub use plugin_registry::{PluginLoader, PluginRegistry, PluginManifest};
pub use main_runner::{CommandRunner, UniversalRunner};

/// Version of the command runner framework
pub const VERSION: &str = "1.0.0";

/// Initialize the command runner
pub fn init() -> Result<CommandRunner, String> {
    CommandRunner::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version() {
        assert_eq!(VERSION, "1.0.0");
    }
}