//! Configuration merging logic for cargo-runner
//! 
//! Implements the merging hierarchy: root -> package (or workspace -> package)
//! The default mode is merge, but overrides can use force_replace to replace instead of merge

use super::{Config, Override};
use crate::error::Result;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub root_config_path: Option<PathBuf>,
    pub workspace_config_path: Option<PathBuf>,
    pub package_config_path: Option<PathBuf>,
}

pub struct ConfigMerger {
    root_config: Option<Config>,
    workspace_config: Option<Config>,
    package_config: Option<Config>,
    config_info: ConfigInfo,
}

impl ConfigMerger {
    pub fn new() -> Self {
        Self {
            root_config: None,
            workspace_config: None,
            package_config: None,
            config_info: ConfigInfo {
                root_config_path: None,
                workspace_config_path: None,
                package_config_path: None,
            },
        }
    }

    /// Load all relevant configs for a given file path
    pub fn load_configs_for_path(&mut self, file_path: &Path) -> Result<()> {
        debug!("Loading configs for path: {:?}", file_path);

        // Find package-level config first
        if let Some(package_config_path) = Self::find_package_config(file_path) {
            debug!("Found package config at: {:?}", package_config_path);
            self.package_config = Some(Config::load_from_file(&package_config_path)?);
            self.config_info.package_config_path = Some(package_config_path);
        }

        // Find workspace config (if different from package)
        if let Some(workspace_config_path) = Self::find_workspace_config(file_path) {
            // Only load if it's different from package config
            if self.package_config.is_none() || 
               self.package_config.as_ref().and_then(|_| Self::find_package_config(file_path)) != Some(workspace_config_path.clone()) {
                debug!("Found workspace config at: {:?}", workspace_config_path);
                self.workspace_config = Some(Config::load_from_file(&workspace_config_path)?);
                self.config_info.workspace_config_path = Some(workspace_config_path);
            }
        }

        // Find PROJECT_ROOT config
        if let Ok(project_root) = std::env::var("PROJECT_ROOT") {
            let root_path = PathBuf::from(&project_root);
            let root_config_path = root_path.join(".cargo-runner.json");
            if root_config_path.exists() {
                debug!("Found root config at: {:?}", root_config_path);
                self.root_config = Some(Config::load_from_file(&root_config_path)?);
                self.config_info.root_config_path = Some(root_config_path);
            }
        }

        Ok(())
    }

    /// Get the merged configuration
    pub fn get_merged_config(&self) -> Config {
        let mut config = Config::default();

        // Start with root config
        if let Some(ref root) = self.root_config {
            config = self.merge_configs(config, root.clone(), false);
        }

        // Apply workspace config
        if let Some(ref workspace) = self.workspace_config {
            config = self.merge_configs(config, workspace.clone(), false);
        }

        // Apply package config
        if let Some(ref package) = self.package_config {
            config = self.merge_configs(config, package.clone(), false);
        }

        config
    }

    /// Merge two configs, respecting force_replace settings
    fn merge_configs(&self, mut base: Config, override_config: Config, force_replace: bool) -> Config {
        // Merge global settings
        if override_config.command.is_some() {
            base.command = override_config.command;
        }
        if override_config.subcommand.is_some() {
            base.subcommand = override_config.subcommand;
        }
        if override_config.channel.is_some() {
            base.channel = override_config.channel;
        }
        if override_config.package.is_some() {
            base.package = override_config.package;
        }

        // Merge extra_args
        if let Some(ref extra_args) = override_config.extra_args {
            if force_replace || base.extra_args.is_none() {
                base.extra_args = Some(extra_args.clone());
            } else if let Some(ref mut base_args) = base.extra_args {
                base_args.extend(extra_args.clone());
            }
        }

        // Merge extra_test_binary_args
        if let Some(ref extra_test_args) = override_config.extra_test_binary_args {
            if force_replace || base.extra_test_binary_args.is_none() {
                base.extra_test_binary_args = Some(extra_test_args.clone());
            } else if let Some(ref mut base_args) = base.extra_test_binary_args {
                base_args.extend(extra_test_args.clone());
            }
        }

        // Merge env
        if let Some(ref env) = override_config.env {
            if force_replace || base.env.is_none() {
                base.env = Some(env.clone());
            } else if let Some(ref mut base_env) = base.env {
                base_env.extend(env.clone());
            }
        }

        // Merge test_frameworks
        if override_config.test_frameworks.is_some() {
            base.test_frameworks = override_config.test_frameworks;
        }

        // Merge overrides - these are function-specific, so we merge the arrays
        self.merge_overrides(&mut base.overrides, override_config.overrides);

        // linked_projects is only allowed at root level, so only copy from root config
        if override_config.linked_projects.is_some() {
            base.linked_projects = override_config.linked_projects;
        }
        
        // Cache settings are internal and not merged

        base
    }
    
    /// Get information about which config files were loaded
    pub fn get_config_info(&self) -> &ConfigInfo {
        &self.config_info
    }

    /// Merge override arrays, handling force_replace per override
    fn merge_overrides(&self, base_overrides: &mut Vec<Override>, new_overrides: Vec<Override>) {
        for new_override in new_overrides {
            // Check if we already have an override for this identity
            if let Some(existing) = base_overrides.iter_mut().find(|o| o.identity == new_override.identity) {
                // Check if this specific override has force_replace
                let _force = new_override.force_replace_args.unwrap_or(false) || 
                            new_override.force_replace_env.unwrap_or(false);
                
                // Merge or replace based on force_replace settings
                if new_override.command.is_some() {
                    existing.command = new_override.command;
                }
                if new_override.subcommand.is_some() {
                    existing.subcommand = new_override.subcommand;
                }
                if new_override.channel.is_some() {
                    existing.channel = new_override.channel;
                }
                if new_override.test_framework.is_some() {
                    existing.test_framework = new_override.test_framework;
                }

                // Handle args merging
                if let Some(ref args) = new_override.extra_args {
                    if new_override.force_replace_args.unwrap_or(false) || existing.extra_args.is_none() {
                        existing.extra_args = Some(args.clone());
                    } else if let Some(ref mut existing_args) = existing.extra_args {
                        existing_args.extend(args.clone());
                    }
                }

                // Handle test binary args merging
                if let Some(ref args) = new_override.extra_test_binary_args {
                    if new_override.force_replace_args.unwrap_or(false) || existing.extra_test_binary_args.is_none() {
                        existing.extra_test_binary_args = Some(args.clone());
                    } else if let Some(ref mut existing_args) = existing.extra_test_binary_args {
                        existing_args.extend(args.clone());
                    }
                }

                // Handle env merging
                if let Some(ref env) = new_override.env {
                    if new_override.force_replace_env.unwrap_or(false) || existing.env.is_none() {
                        existing.env = Some(env.clone());
                    } else if let Some(ref mut existing_env) = existing.env {
                        existing_env.extend(env.clone());
                    }
                }

                // Update force_replace flags
                if new_override.force_replace_args.is_some() {
                    existing.force_replace_args = new_override.force_replace_args;
                }
                if new_override.force_replace_env.is_some() {
                    existing.force_replace_env = new_override.force_replace_env;
                }
            } else {
                // No existing override for this identity, add the new one
                base_overrides.push(new_override);
            }
        }
    }

    /// Find the nearest .cargo-runner.json for a package
    fn find_package_config(path: &Path) -> Option<PathBuf> {
        let mut current = path;
        if current.is_file() {
            current = current.parent()?;
        }

        loop {
            // Check for config file
            let config_path = current.join(".cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            // Check if we've hit a Cargo.toml (package boundary)
            if current.join("Cargo.toml").exists() {
                // This is a package root, but no config found
                return None;
            }

            current = current.parent()?;
        }
    }

    /// Find workspace-level config
    fn find_workspace_config(path: &Path) -> Option<PathBuf> {
        let mut current = path;
        if current.is_file() {
            current = current.parent()?;
        }

        let mut _found_package_root = false;

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is a workspace
                if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                    if contents.contains("[workspace]") {
                        let config_path = current.join(".cargo-runner.json");
                        if config_path.exists() {
                            return Some(config_path);
                        }
                    }
                }
                _found_package_root = true;
            }

            // If we've passed a package root and still looking, we might find workspace
            current = current.parent()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_merging() {
        let mut base = Config {
            command: Some("cargo".to_string()),
            extra_args: Some(vec!["--release".to_string()]),
            env: Some(HashMap::from([("RUST_LOG".to_string(), "info".to_string())])),
            ..Default::default()
        };

        let override_config = Config {
            channel: Some("nightly".to_string()),
            extra_args: Some(vec!["--features".to_string(), "foo".to_string()]),
            env: Some(HashMap::from([("RUST_BACKTRACE".to_string(), "1".to_string())])),
            ..Default::default()
        };

        let merger = ConfigMerger::new();
        let merged = merger.merge_configs(base, override_config, false);

        assert_eq!(merged.command, Some("cargo".to_string()));
        assert_eq!(merged.channel, Some("nightly".to_string()));
        assert_eq!(
            merged.extra_args,
            Some(vec!["--release".to_string(), "--features".to_string(), "foo".to_string()])
        );
        assert_eq!(
            merged.env.unwrap().get("RUST_LOG"),
            Some(&"info".to_string())
        );
        assert_eq!(
            merged.env.unwrap().get("RUST_BACKTRACE"),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn test_force_replace() {
        let base = Config {
            extra_args: Some(vec!["--release".to_string()]),
            env: Some(HashMap::from([("RUST_LOG".to_string(), "info".to_string())])),
            ..Default::default()
        };

        let override_config = Config {
            extra_args: Some(vec!["--debug".to_string()]),
            env: Some(HashMap::from([("RUST_LOG".to_string(), "debug".to_string())])),
            ..Default::default()
        };

        let merger = ConfigMerger::new();
        let merged = merger.merge_configs(base, override_config, true);

        // With force_replace, args should be replaced, not merged
        assert_eq!(merged.extra_args, Some(vec!["--debug".to_string()]));
        // Env should also be replaced
        assert_eq!(
            merged.env.unwrap().get("RUST_LOG"),
            Some(&"debug".to_string())
        );
    }
}