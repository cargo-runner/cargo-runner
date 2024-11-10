use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};


use crate::Error;

use super::CommandType;

use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub name: String,
    #[serde(default)]
    #[serde(serialize_with = "serialize_command_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_type: Option<CommandType>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub sub_command: Option<String>,
    #[serde(default)]
    pub allowed_subcommands: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

fn serialize_command_type<S>(
    command_type: &Option<CommandType>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match command_type {
        Some(cmd_type) => match cmd_type {
            CommandType::Cargo => serializer.serialize_str("cargo"),
            CommandType::SubCommand => serializer.serialize_str("subcommand"),
            CommandType::Shell => serializer.serialize_str("shell"),
        },
        None => serializer.serialize_none(),
    }
}

impl Config {
    pub fn merge(&mut self, other: &Config) -> Result<(), Error> {
        if self.name != other.name {
            return Err(Error::MergeConflict(self.name.clone(),other.name.clone()));
        }

        if let Some(cmd_type) = &other.command_type {
            self.command_type = Some(cmd_type.clone());
        }
        if let Some(cmd) = &other.command {
            self.command = Some(cmd.clone());
        }
        if let Some(sub_cmd) = &other.sub_command {
            self.sub_command = Some(sub_cmd.clone());
        }
        if let Some(allowed) = &other.allowed_subcommands {
            self.allowed_subcommands = Some(allowed.clone());
        }
        if let Some(other_env) = &other.env {
            let base_env = self.env.get_or_insert_with(HashMap::new);
            base_env.extend(other_env.clone());
        }
        Ok(())
    }
}
