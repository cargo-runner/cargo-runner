//! cargo-runner - A tool for detecting and running Rust code at specific locations
//!
//! This crate provides functionality to:
//! - Parse Rust source files and detect runnable items (tests, benchmarks, binaries)
//! - Generate appropriate cargo commands for running specific items
//! - Support various project structures and configurations
pub mod build_system;
pub mod command;
pub mod config;
pub mod error;
pub mod parser;
pub mod patterns;
pub mod runner;
pub mod types;

// Re-export commonly used types and traits
pub use error::{Error, Result};
pub use types::*;

// Re-export main API components
pub use command::{CargoCommand, CommandType};
pub use config::Config;
pub use runner::CargoRunner;
