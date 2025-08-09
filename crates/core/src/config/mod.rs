//! Configuration management for cargo-runner

mod cargo_config;
mod features;
mod merge;
pub mod override_config;
mod rustc_config;
mod settings;
pub mod test_framework;
pub mod utils;

// Re-export main types
pub use cargo_config::{BinaryFramework, CargoConfig, SingleFileScriptConfig};
pub use features::Features;
pub use merge::{ConfigInfo, ConfigMerger};
pub use override_config::Override;
pub use rustc_config::{RustcConfig, RustcFramework, RustcPhaseConfig};
pub use settings::Config;
pub use test_framework::TestFramework;
pub use utils::is_valid_channel;
