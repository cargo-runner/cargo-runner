//! Configuration management for cargo-runner

mod merge;
pub mod override_config;
mod settings;
pub mod test_framework;
pub mod utils;

// Re-export main types
pub use merge::{ConfigMerger, ConfigInfo};
pub use override_config::Override;
pub use settings::Config;
pub use test_framework::TestFramework;
pub use utils::is_valid_channel;
