use crate::types::FunctionIdentity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::TestFramework;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Override {
    #[serde(rename = "match")]
    #[serde(alias = "function")] // For backward compatibility with FunctionBased
    pub identity: FunctionIdentity,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_framework: Option<TestFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_replace_args: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_replace_env: Option<bool>,
}
