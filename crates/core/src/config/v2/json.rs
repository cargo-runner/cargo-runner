//! JSON serialization and deserialization for v2 configs
//!
//! This module provides serde support for converting v2 configurations
//! to and from JSON format.

use super::{Config, ConfigBuilder, builder::LayerConfigExt};
use crate::build_system::BuildSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON representation of a v2 configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonConfig {
    /// Version field (should be "2.0")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    
    /// Linked projects (only at PROJECT_ROOT level)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_projects: Option<Vec<String>>,
    
    /// Build system (top-level, not nested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_system: Option<BuildSystem>,
    
    /// Framework strategies (top-level, not nested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frameworks: Option<JsonFrameworks>,
    
    /// Arguments configuration (top-level for workspace defaults)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<JsonArgs>,
    
    /// Environment variables (top-level for workspace defaults)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    
    /// Workspace-level configuration (DEPRECATED - for backward compatibility only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<JsonLayerConfig>,

    /// Crate-level overrides
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub crates: HashMap<String, JsonLayerConfig>,

    /// File-level overrides
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub files: HashMap<String, JsonLayerConfig>,

    /// Module-level overrides
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub modules: HashMap<String, JsonLayerConfig>,

    /// Function-level overrides
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub functions: HashMap<String, JsonLayerConfig>,
}

/// JSON representation of a configuration layer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonLayerConfig {
    /// Build system override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_system: Option<BuildSystem>,

    /// Framework strategies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frameworks: Option<JsonFrameworks>,

    /// Arguments configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<JsonArgs>,

    /// Environment variables
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

/// JSON representation of framework strategies
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonFrameworks {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub doctest: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

/// JSON representation of arguments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonArgs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub test: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_binary: Option<Vec<String>>,
}

impl JsonConfig {
    /// Convert to a v2 Config
    pub fn to_config(self) -> Config {
        let mut builder = ConfigBuilder::new();

        // Check if we have any top-level configuration
        let has_top_level_config = self.build_system.is_some()
            || self.frameworks.is_some()
            || self.args.is_some()
            || !self.env.is_empty();

        // Only create workspace layer if we have configuration
        if has_top_level_config || self.workspace.is_some() {
            builder = builder.workspace(|w| {
                // Apply top-level build system
                if let Some(build_system) = self.build_system.clone() {
                    w.build_system(build_system);
                }
                
                // Apply top-level frameworks
                if let Some(frameworks) = self.frameworks.clone() {
                    if let Some(test) = frameworks.test {
                        w.framework_test(test);
                    }
                    if let Some(binary) = frameworks.binary {
                        w.framework_binary(binary);
                    }
                    if let Some(benchmark) = frameworks.benchmark {
                        w.framework_benchmark(benchmark);
                    }
                    if let Some(doctest) = frameworks.doctest {
                        w.framework_doctest(doctest);
                    }
                    if let Some(build) = frameworks.build {
                        w.framework_build(build);
                    }
                }
                
                // Apply top-level args
                if let Some(args) = self.args.clone() {
                    if let Some(all) = args.all {
                        w.args_all(all);
                    }
                    if let Some(test) = args.test {
                        w.args_test(test);
                    }
                    if let Some(binary) = args.binary {
                        w.args_binary(binary);
                    }
                    if let Some(benchmark) = args.benchmark {
                        w.args_benchmark(benchmark);
                    }
                    if let Some(build) = args.build {
                        w.args_build(build);
                    }
                    if let Some(test_binary) = args.test_binary {
                        w.args_test_binary(test_binary);
                    }
                }
                
                // Apply top-level env
                for (key, value) in self.env.clone() {
                    w.env(key, value);
                }
                
                // Apply workspace config if present (for backward compatibility)
                if let Some(workspace) = self.workspace.clone() {
                    Self::apply_layer_config(w, workspace);
                }
            });
        }

        // Apply crate overrides
        for (crate_name, config) in self.crates {
            builder = builder.crate_override(&crate_name, |c| {
                Self::apply_layer_config(c, config);
            });
        }

        // Apply file overrides
        for (file_path, config) in self.files {
            builder = builder.file_override(&file_path, |f| {
                Self::apply_layer_config(f, config);
            });
        }

        // Apply module overrides
        for (module_path, config) in self.modules {
            builder = builder.module_override(&module_path, |m| {
                Self::apply_layer_config(m, config);
            });
        }

        // Apply function overrides
        for (func_name, config) in self.functions {
            builder = builder.function_override(&func_name, |f| {
                Self::apply_layer_config(f, config);
            });
        }

        let mut config = builder.build();
        
        // Set linked_projects if present
        config.linked_projects = self.linked_projects;
        
        config
    }

    /// Apply a JSON layer config to a layer builder
    fn apply_layer_config<T: LayerConfigExt>(layer: &mut T, config: JsonLayerConfig) {
        // Build system
        if let Some(build_system) = config.build_system {
            layer.build_system(build_system);
        }

        // Frameworks
        if let Some(frameworks) = config.frameworks {
            if let Some(test) = frameworks.test {
                layer.framework_test(test);
            }
            if let Some(binary) = frameworks.binary {
                layer.framework_binary(binary);
            }
            if let Some(benchmark) = frameworks.benchmark {
                layer.framework_benchmark(benchmark);
            }
            if let Some(doctest) = frameworks.doctest {
                layer.framework_doctest(doctest);
            }
            if let Some(build) = frameworks.build {
                layer.framework_build(build);
            }
        }

        // Arguments
        if let Some(args) = config.args {
            if let Some(all) = args.all {
                layer.args_all(all);
            }
            if let Some(test) = args.test {
                layer.args_test(test);
            }
            if let Some(binary) = args.binary {
                layer.args_binary(binary);
            }
            if let Some(benchmark) = args.benchmark {
                layer.args_benchmark(benchmark);
            }
            if let Some(build) = args.build {
                layer.args_build(build);
            }
            if let Some(test_binary) = args.test_binary {
                layer.args_test_binary(test_binary);
            }
        }

        // Environment variables
        for (key, value) in config.env {
            layer.env(key, value);
        }
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Convert a Config back to JsonConfig (for saving)
impl From<&Config> for JsonConfig {
    fn from(_config: &Config) -> Self {
        // Note: This is a lossy conversion since we don't store the original
        // structure in the Config. For now, we'll return an empty config.
        // In a real implementation, we'd need to store metadata about the
        // original structure or provide a way to reconstruct it.
        JsonConfig {
            version: None,
            linked_projects: None,
            build_system: None,
            frameworks: None,
            args: None,
            env: HashMap::new(),
            workspace: None,
            crates: HashMap::new(),
            files: HashMap::new(),
            modules: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_round_trip() {
        let json = r#"{
            "version": "2.0",
            "build_system": "Cargo",
            "frameworks": {
                "test": "cargo-test",
                "binary": "cargo-run"
            },
            "args": {
                "all": ["--verbose"],
                "test": ["--nocapture"]
            },
            "env": {
                "RUST_LOG": "info"
            },
            "crates": {
                "my-crate": {
                    "frameworks": {
                        "test": "cargo-nextest"
                    },
                    "env": {
                        "RUST_LOG": "debug"
                    }
                }
            },
            "modules": {
                "tests": {
                    "args": {
                        "test_binary": ["--test-threads=1"]
                    }
                }
            }
        }"#;

        let json_config = JsonConfig::from_json(json).unwrap();
        let config = json_config.to_config();

        // Test that config was built correctly
        let resolver = config.resolver();
        let context = super::super::scope::ScopeContext::new()
            .with_crate("my-crate".to_string())
            .with_module("tests".to_string());

        let command = resolver
            .resolve_command(
                &context,
                crate::types::RunnableKind::Test {
                    test_name: "test".to_string(),
                    is_async: false,
                },
            )
            .unwrap();

        // Should use nextest from crate override
        assert!(command.args.contains(&"nextest".to_string()));
        // Should have verbose from workspace
        assert!(command.args.contains(&"--verbose".to_string()));
        // Should have debug log level from crate override
        assert!(
            command
                .env
                .iter()
                .any(|(k, v)| k == "RUST_LOG" && v == "debug")
        );
    }

    #[test]
    fn test_empty_json_config() {
        let json = "{}";
        let json_config = JsonConfig::from_json(json).unwrap();
        let config = json_config.to_config();

        // Should create a valid but empty config
        assert_eq!(config.layers.len(), 0);
    }

    #[test]
    fn test_json_serialization() {
        let mut json_config = JsonConfig {
            version: Some("2.0".to_string()),
            linked_projects: None,
            build_system: Some(BuildSystem::Cargo),
            frameworks: Some(JsonFrameworks {
                test: Some("cargo-test".to_string()),
                ..Default::default()
            }),
            args: None,
            env: HashMap::new(),
            workspace: None,
            crates: HashMap::new(),
            files: HashMap::new(),
            modules: HashMap::new(),
            functions: HashMap::new(),
        };

        json_config.crates.insert(
            "my-crate".to_string(),
            JsonLayerConfig {
                frameworks: Some(JsonFrameworks {
                    test: Some("cargo-nextest".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );

        let json = json_config.to_json().unwrap();
        assert!(json.contains("\"build_system\": \"Cargo\""));
        assert!(json.contains("\"test\": \"cargo-test\""));
        assert!(json.contains("\"test\": \"cargo-nextest\""));
    }
}
