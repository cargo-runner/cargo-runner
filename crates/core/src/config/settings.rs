use crate::{
    error::{Error, Result},
    types::FunctionIdentity,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{Override, TestFramework};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    // Global runner configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_frameworks: Option<TestFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_projects: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,

    // Overrides for specific functions
    #[serde(default)]
    pub overrides: Vec<Override>,

    // Cache settings (internal, not exposed in JSON)
    #[serde(skip)]
    pub cache_enabled: bool,
    #[serde(skip)]
    pub cache_dir: Option<PathBuf>,
}


impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config: {e}")))?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn get_override_for(&self, identity: &FunctionIdentity) -> Option<&Override> {
        self.overrides
            .iter()
            .find(|override_| override_.identity.matches(identity))
    }

    pub fn find_config_file(start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path;

        loop {
            let config_path = current.join(".cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            let config_path = current.join("cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            current = current.parent()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = Config {
            command: Some("cargo".to_string()),
            channel: Some("nightly".to_string()),
            extra_args: Some(vec!["--release".to_string()]),
            overrides: vec![Override {
                identity: FunctionIdentity {
                    package: Some("my_crate".to_string()),
                    module_path: Some("my_crate::tests::unit".to_string()),
                    file_path: None,
                    function_name: Some("test_addition".to_string()),
                },
                command: Some("cargo".to_string()),
                subcommand: Some("nextest".to_string()),
                channel: None,
                extra_args: Some(vec!["--nocapture".to_string()]),
                extra_test_binary_args: Some(vec!["--test-threads=1".to_string()]),
                test_framework: None,
                force_replace_args: Some(false),
                env: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "debug".to_string(),
                )])),
                force_replace_env: Some(false),
            }],
            cache_enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/cargo-runner-cache")),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{json}");

        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.overrides.len(), 1);
        assert!(parsed.cache_enabled);
        assert_eq!(parsed.channel, Some("nightly".to_string()));
    }

    #[test]
    fn test_get_override_for() {
        let config = Config {
            overrides: vec![Override {
                identity: FunctionIdentity {
                    package: Some("my_crate".to_string()),
                    module_path: Some("my_crate::tests".to_string()),
                    file_path: None,
                    function_name: Some("test_foo".to_string()),
                },
                extra_args: Some(vec!["--nocapture".to_string()]),
                ..Default::default()
            }],
            ..Default::default()
        };

        let identity = FunctionIdentity {
            package: Some("my_crate".to_string()),
            module_path: Some("my_crate::tests".to_string()),
            file_path: None,
            function_name: Some("test_foo".to_string()),
        };

        let override_config = config.get_override_for(&identity);
        assert!(override_config.is_some());
        assert_eq!(
            override_config.unwrap().extra_args,
            Some(vec!["--nocapture".to_string()])
        );

        // Test with different identity
        let identity2 = FunctionIdentity {
            package: Some("my_crate".to_string()),
            module_path: Some("my_crate::tests".to_string()),
            file_path: None,
            function_name: Some("test_bar".to_string()),
        };

        let override_config2 = config.get_override_for(&identity2);
        assert!(override_config2.is_none());
    }
}
