//! New runner architecture with clear separation of concerns

pub mod traits;
pub mod cargo_runner;
pub mod bazel_runner;
pub mod rustc_runner;
pub mod unified_runner;
pub mod framework;
pub mod options;
pub mod validation;
pub mod builder;

// Re-export main types
pub use traits::{CommandRunner, RunnerCommand};
pub use unified_runner::UnifiedRunner;
pub use framework::{Framework, FrameworkKind};
pub use builder::{CommandBuilder, Unvalidated, Validated};
pub use validation::{ValidationRule, ValidationError};