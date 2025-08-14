//! cargo-runner - A tool for detecting and running Rust code at specific locations
//!
//! This crate provides functionality to:
//! - Parse Rust source files and detect runnable items (tests, benchmarks, binaries)
//! - Generate appropriate cargo commands for running specific items
//! - Support various project structures and configurations
pub mod bazel;
pub mod build_system;
pub mod command;
pub mod config;
pub mod error;
pub mod parser;
pub mod patterns;
pub mod types;

// New runner architecture
pub mod runner_v2;

// Re-export commonly used types and traits
pub use error::{Error, Result};
pub use types::*;

// Re-export main API components
pub use command::{CargoCommand, CommandType};
pub use config::Config;

// Export the new unified runner
pub use runner_v2::UnifiedRunner;

// Legacy alias for backwards compatibility
pub use runner_v2::UnifiedRunner as CargoRunner;
