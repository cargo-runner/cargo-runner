//! Configuration management for cargo-runner

pub mod config;
pub mod override_config;
pub mod test_framework;
pub mod utils;

// Re-export main types
pub use config::Config;
pub use override_config::Override;
pub use test_framework::TestFramework;
pub use utils::is_valid_channel;