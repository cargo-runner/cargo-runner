//! Test command builder with test framework support

use super::TargetCommandBuilder;
use crate::{
    command::{CargoCommand, Target},
    config::Config,
    error::Result,
    types::{FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;
use tracing::debug;

pub struct TestBuilder {
    config: Config,
}

impl TestBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    /// Build args based on test framework configuration
    fn build_base_args(&self) -> Vec<String> {
        let mut args = vec![];
        
        // Check if we have a test framework configured
        if let Some(test_framework) = &self.config.test_framework {
            // Use the subcommand from test framework (e.g., "nextest run")
            if let Some(subcommand) = &test_framework.subcommand {
                // Split subcommand into parts (e.g., "nextest run" -> ["nextest", "run"])
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                // Default to "test" if no subcommand specified
                args.push("test".to_string());
            }
            
            // Add framework-specific args
            if let Some(framework_args) = &test_framework.args {
                args.extend(framework_args.clone());
            }
        } else {
            // No test framework, use standard cargo test
            args.push("test".to_string());
        }
        
        args
    }
    
    /// Get the command prefix (cargo + channel)
    fn get_command_prefix(&self) -> String {
        // Check override first, then test framework, then global config
        let channel = self.config.test_framework
            .as_ref()
            .and_then(|tf| tf.channel.as_ref())
            .or(self.config.channel.as_ref());
            
        if let Some(channel) = channel {
            format!("+{}", channel)
        } else {
            String::new()
        }
    }
}

impl TargetCommandBuilder for TestBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        _project_root: &Path,
    ) -> Result<CargoCommand> {
        let mut args = self.build_base_args();
        
        // Add channel if specified
        let channel_prefix = self.get_command_prefix();
        if !channel_prefix.is_empty() {
            args.insert(0, channel_prefix);
        }
        
        // Add package if specified
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
        
        // Determine and add target
        debug!("Determining target for test file: {:?}", runnable.file_path);
        if let Some(target) = Target::from_file_path(&runnable.file_path) {
            match target {
                Target::Lib => args.push("--lib".to_string()),
                Target::Bin(name) => {
                    args.push("--bin".to_string());
                    args.push(name);
                }
                Target::Test(name) => {
                    args.push("--test".to_string());
                    args.push(name);
                }
                _ => {} // Other targets not applicable for tests
            }
        }
        
        // Apply configuration overrides
        if let Some(override_config) = self.get_override(runnable) {
            // Add extra args from override
            if let Some(extra_args) = &override_config.extra_args {
                args.extend(extra_args.clone());
            }
        }
        
        // Apply global extra args
        if let Some(extra_args) = &self.config.extra_args {
            args.extend(extra_args.clone());
        }
        
        // Add test filter
        if let RunnableKind::Test { test_name, .. } = &runnable.kind {
            args.push("--".to_string());
            
            // Build the full test path
            let test_path = if runnable.module_path.is_empty() {
                test_name.clone()
            } else {
                format!("{}::{}", runnable.module_path, test_name)
            };
            args.push(test_path);
            args.push("--exact".to_string());
            
            // Apply test binary args from override
            if let Some(override_config) = self.get_override(runnable) {
                if let Some(extra_test_binary_args) = &override_config.extra_test_binary_args {
                    args.extend(extra_test_binary_args.clone());
                }
            }
            
            // Apply global test binary args
            if let Some(extra_test_binary_args) = &self.config.extra_test_binary_args {
                args.extend(extra_test_binary_args.clone());
            }
        }
        
        let mut command = CargoCommand::new(args);
        
        // Apply test framework env vars
        if let Some(test_framework) = &self.config.test_framework {
            if let Some(env) = &test_framework.extra_env {
                for (key, value) in env {
                    command.env.insert(key.clone(), value.clone());
                }
            }
        }
        
        // Apply common configuration
        self.apply_common_config(&mut command, runnable);
        
        // Apply override env vars (these take precedence)
        if let Some(override_config) = self.get_override(runnable) {
            if let Some(extra_env) = &override_config.extra_env {
                for (key, value) in extra_env {
                    command.env.insert(key.clone(), value.clone());
                }
            }
        }
        
        Ok(command)
    }
    
    fn config(&self) -> &Config {
        &self.config
    }
    
    fn create_identity(&self, runnable: &Runnable) -> FunctionIdentity {
        FunctionIdentity {
            package: self.config.package.clone(),
            module_path: if runnable.module_path.is_empty() { 
                None 
            } else { 
                Some(runnable.module_path.clone()) 
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
                _ => None,
            },
        }
    }
}