//! Configuration management for cargo-runner

mod bazel_config;
mod cargo_config;
mod features;
mod merge;
pub mod override_config;
mod rustc_config;
mod settings;
pub mod test_framework;
pub mod utils;
pub mod validation;

// Re-export main types
pub use bazel_config::{BazelConfig, BazelFramework};
// NUKE-CONFIG: Removed BinaryFramework export
pub use cargo_config::{CargoConfig, SingleFileScriptConfig};
pub use features::Features;
pub use merge::{ConfigInfo, ConfigMerger};
pub use override_config::Override;
pub use rustc_config::{RustcConfig, RustcFramework, RustcPhaseConfig};
pub use settings::Config;
// NUKE-CONFIG: Removed TestFramework export
// TODO: Delete test_framework.rs file after removing all references
pub use utils::is_valid_channel;
