//! Cargo command generation and execution

pub mod builder;
pub mod builder_v2;
pub mod cargo_command;
pub mod fallback;
pub mod target;

// Re-export commonly used types
pub use cargo_command::{CargoCommand, CommandType};
pub use target::Target;

// Clean public API for library users
pub use builder_v2::CommandBuilder;
