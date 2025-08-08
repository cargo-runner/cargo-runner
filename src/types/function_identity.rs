use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents the identity of a function in a Rust project
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionIdentity {
    pub package: Option<String>,
    pub module_path: Option<String>,
    pub file_path: Option<PathBuf>,
    pub function_name: Option<String>,
}