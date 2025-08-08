use crate::{
    error::{Error, Result},
    types::FunctionIdentity,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{CargoConfig, Override, RustcConfig, SingleFileScriptConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    // Command-type specific configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo: Option<CargoConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc: Option<RustcConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_file_script: Option<SingleFileScriptConfig>,

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
    use crate::types::{FunctionIdentity, FileType};
    use std::collections::HashMap;

    #[test]
    fn test_config_serialization() {
        let config = Config {
            cargo: Some(CargoConfig {
                command: Some("cargo".to_string()),
                channel: Some("nightly".to_string()),
                extra_args: Some(vec!["--release".to_string()]),
                ..Default::default()
            }),
            overrides: vec![Override {
                identity: FunctionIdentity {
                    package: Some("my_crate".to_string()),
                    module_path: Some("my_crate::tests::unit".to_string()),
                    file_path: None,
                    function_name: Some("test_addition".to_string()),
                    file_type: Some(FileType::CargoProject),
                },
                features: None,
                force_replace_features: Some(false),
                command: Some("cargo".to_string()),
                subcommand: Some("nextest".to_string()),
                channel: None,
                extra_args: Some(vec!["--nocapture".to_string()]),
                extra_test_binary_args: Some(vec!["--test-threads=1".to_string()]),
                test_framework: None,
                force_replace_args: Some(false),
                extra_env: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "debug".to_string(),
                )])),
                force_replace_env: Some(false),
                cargo: None,
                rustc: None,
                single_file_script: None,
            }],
            cache_enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/cargo-runner-cache")),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{json}");

        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.overrides.len(), 1);
        // cache_enabled is skipped in serialization, so it will be default (false)
        assert!(!parsed.cache_enabled);
        assert_eq!(parsed.cargo.as_ref().unwrap().channel, Some("nightly".to_string()));
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
                    file_type: Some(FileType::CargoProject),
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
            file_type: Some(FileType::CargoProject),
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
            file_type: Some(FileType::CargoProject),
        };

        let override_config2 = config.get_override_for(&identity2);
        assert!(override_config2.is_none());
    }
}
