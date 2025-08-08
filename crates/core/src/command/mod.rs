//! Cargo command generation and execution

pub mod builder;
pub mod cargo_command;
pub mod fallback;
pub mod target;

// Re-export commonly used types
pub use cargo_command::{CargoCommand, CommandType};
pub use target::Target;