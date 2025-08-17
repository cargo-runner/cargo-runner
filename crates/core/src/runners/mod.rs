//! Runner architecture for v2 configuration system

pub mod builder;
pub mod common;
pub mod composable_runner;
pub mod framework;
pub mod options;
pub mod traits;
pub mod unified_runner;
pub mod validation;

// Re-export main types
pub use builder::{CommandBuilder, Unvalidated, Validated};
pub use composable_runner::{ComposableRunner, ComposableRunnerBuilder};
pub use framework::{Framework, FrameworkKind};
pub use traits::{CommandRunner, RunnerCommand};
pub use unified_runner::UnifiedRunner;
pub use validation::{ValidationError, ValidationRule};
