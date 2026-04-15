// Feature gate needed for Bazel nightly toolchain; no-op on stable 1.88+
#![allow(stable_features)]
#![feature(let_chains)]
pub mod cli;
pub mod commands;
pub mod config;
pub mod display;
pub mod utils;

// Re-export commonly used items
pub use cli::{Cargo, Command, Commands, Runner};
