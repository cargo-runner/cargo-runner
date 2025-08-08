//! Doc test command builder

use super::TargetCommandBuilder;
use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    types::{FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;

pub struct DocTestBuilder {
    config: Config,
}

impl DocTestBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl TargetCommandBuilder for DocTestBuilder {
    fn build(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        _project_root: &Path,
    ) -> Result<CargoCommand> {
        let mut args = vec!["test".to_string(), "--doc".to_string()];
        
        // Add package if specified
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
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
        
        // Add doc test filter
        if let RunnableKind::DocTest { struct_or_module_name, method_name } = &runnable.kind {
            args.push("--".to_string());
            
            // Build the test path
            let test_path = if let Some(method) = method_name {
                format!("{}::{}", struct_or_module_name, method)
            } else {
                struct_or_module_name.clone()
            };
            args.push(test_path);
            
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
        
        // Apply common configuration
        self.apply_common_config(&mut command, runnable);
        
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable) {
            if let Some(env) = &override_config.env {
                for (key, value) in env {
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
        let package_name = runnable.file_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(String::from);
            
        if let RunnableKind::DocTest { struct_or_module_name, method_name } = &runnable.kind {
            let function_name = if let Some(method) = method_name {
                Some(format!("{}::{}", struct_or_module_name, method))
            } else {
                Some(struct_or_module_name.clone())
            };
            
            FunctionIdentity {
                package: package_name,
                module_path: if runnable.module_path.is_empty() { 
                    None 
                } else { 
                    Some(runnable.module_path.clone()) 
                },
                file_path: Some(runnable.file_path.clone()),
                function_name,
            }
        } else {
            // Shouldn't happen but handle gracefully
            FunctionIdentity::default()
        }
    }
}