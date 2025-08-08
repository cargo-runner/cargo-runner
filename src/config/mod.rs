use crate::{FunctionIdentity, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub extra_env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_framework: Option<TestFramework>,
    
    // Overrides for specific functions
    #[serde(default)]
    pub overrides: Vec<Override>,
    
    // Cache settings
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
}

fn default_cache_enabled() -> bool {
    true
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestFramework {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
}

impl Config {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)
            .map_err(|e| crate::Error::ParseError(format!("Failed to parse config: {}", e)))?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| crate::Error::ParseError(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn get_override_for(&self, identity: &FunctionIdentity) -> Option<&Override> {
        self.overrides
            .iter()
            .find(|override_| override_.identity == *identity)
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

pub fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, "stable" | "beta" | "nightly")
        || channel.starts_with("stable-")
        || channel.starts_with("beta-")
        || channel.starts_with("nightly-")
        || channel.starts_with("1.")
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
                extra_env: Some(HashMap::from([
                    ("RUST_LOG".to_string(), "debug".to_string()),
                ])),
                force_replace_env: Some(false),
            }],
            cache_enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/cargo-runner-cache")),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{}", json);

        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.overrides.len(), 1);
        assert_eq!(parsed.cache_enabled, true);
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

    #[test]
    fn test_valid_channel() {
        assert!(is_valid_channel("stable"));
        assert!(is_valid_channel("beta"));
        assert!(is_valid_channel("nightly"));
        assert!(is_valid_channel("stable-2023-01-01"));
        assert!(is_valid_channel("beta-2023-01-01"));
        assert!(is_valid_channel("nightly-2023-01-01"));
        assert!(is_valid_channel("1.75.0"));
        assert!(is_valid_channel("1.76.0-beta.1"));
        
        assert!(!is_valid_channel("invalid"));
        assert!(!is_valid_channel("2.0.0")); // Rust 2.x doesn't exist yet
    }
}