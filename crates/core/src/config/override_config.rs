use crate::types::FunctionIdentity;
use serde::{Deserialize, Serialize};

use super::{BazelConfig, CargoConfig, RustcConfig, SingleFileScriptConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Override {
    #[serde(rename = "match")]
    pub identity: FunctionIdentity,

    // Command-type specific overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo: Option<CargoConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc: Option<RustcConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_file_script: Option<SingleFileScriptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bazel: Option<BazelConfig>,
}
