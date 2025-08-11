//! Common functionality for cargo builders

use crate::{
    config::{Config, Features},
    types::{FileType, FunctionIdentity, Runnable},
};

/// Helper trait for common builder functionality
pub trait CargoBuilderHelper {
    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config, file_type);
        tracing::debug!("Looking for override for identity: {:?}", identity);
        let result = config.get_override_for(&identity);
        if result.is_some() {
            tracing::debug!("Found matching override!");
        } else {
            tracing::debug!("No matching override found");
        }
        result
    }

    fn create_identity(
        &self,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) -> FunctionIdentity {
        FunctionIdentity {
            package: config.cargo.as_ref().and_then(|c| c.package.clone()),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        }
    }

    fn apply_features(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
        features: Option<&Features>,
    ) {
        // Features are only applicable to Cargo projects
        if file_type != FileType::CargoProject {
            return;
        }

        // Apply override features
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(features) = &override_cargo.features {
                    args.extend(features.to_args());
                    // Features are merged by default now
                }
            }
        }

        // Apply provided features
        if let Some(features) = features {
            args.extend(features.to_args());
        }
    }

    fn apply_common_config(
        &self,
        command: &mut crate::command::CargoCommand,
        _config: &Config,
        _file_type: FileType,
        extra_env: Option<&std::collections::HashMap<String, String>>,
    ) {
        // Apply environment variables based on file type
        if let Some(extra_env) = extra_env {
            for (key, value) in extra_env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
}
