pub mod function_identity;
pub mod position;
pub mod runnable;
pub mod scope;
pub mod scope_kind;

use crate::impl_case_insensitive_deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FileType {
    CargoProject,
    Standalone,
    SingleFileScript,
}

// Implement case-insensitive deserialization
impl_case_insensitive_deserialize!(
    FileType,
    CargoProject => "cargo_project",
    Standalone => "standalone",
    SingleFileScript => "single_file_script"
);

// Re-export commonly used types
pub use function_identity::FunctionIdentity;
pub use position::Position;
pub use runnable::{Runnable, RunnableKind, RunnableWithScore};
pub use scope::{ExtendedScope, Scope};
pub use scope_kind::{FileScope, ScopeKind};
