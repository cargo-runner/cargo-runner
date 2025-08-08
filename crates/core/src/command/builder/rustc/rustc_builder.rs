//! Rustc command builder for standalone files

use crate::{
    command::{builder::{CommandBuilderImpl, ConfigAccess}, CargoCommand},
    config::Config,
    error::Result,
    types::{FileType, FunctionIdentity, Runnable, RunnableKind},
};

/// Rustc command builder for standalone files
pub struct RustcCommandBuilder;

impl ConfigAccess for RustcCommandBuilder {}

impl CommandBuilderImpl for RustcCommandBuilder {
    fn build(
        runnable: &Runnable,
        _package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let builder = RustcCommandBuilder;
        let file_name = runnable
            .file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| crate::error::Error::ParseError("Invalid file name".to_string()))?;

        match &runnable.kind {
            RunnableKind::Test { .. } => {
                // Run all tests with rustc --test
                let output_name = format!("{}_test", file_name);
                let mut args = vec![
                    "--test".to_string(),
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "-o".to_string(),
                    output_name.clone(),
                ];

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                // Get test filter if available
                let test_filter = runnable.get_function_name();

                // Create a rustc command with test filter
                let mut command = CargoCommand::new_rustc(args)
                    .with_test_filter(test_filter.unwrap_or_default());

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::ModuleTests { .. } => {
                // Run all tests in module with rustc --test
                let mut args = vec![
                    "--test".to_string(),
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "-o".to_string(),
                    format!("{}_test", file_name),
                ];

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new_rustc(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::Binary { .. } | RunnableKind::Standalone { .. } => {
                // Run main binary
                let mut args = vec![
                    runnable.file_path.to_str().unwrap_or("").to_string(),
                    "-o".to_string(),
                    file_name.to_string(),
                ];

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new_rustc(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            _ => Err(crate::error::Error::ParseError(
                "Unsupported runnable type for rustc".to_string(),
            )),
        }
    }
}

impl RustcCommandBuilder {
    fn apply_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_rustc) = &override_config.rustc {
                if let Some(extra_args) = &override_rustc.extra_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_env(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_rustc) = &override_config.rustc {
                if let Some(extra_env) = &override_rustc.extra_env {
                    for (key, value) in extra_env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
    }

    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config, file_type);
        config.get_override_for(&identity)
    }

    fn create_identity(
        &self,
        runnable: &Runnable,
        _config: &Config,
        file_type: FileType,
    ) -> FunctionIdentity {
        FunctionIdentity {
            package: None, // Standalone files don't have packages
            module_path: None,
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        }
    }

    fn apply_common_config(
        &self,
        command: &mut CargoCommand,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply environment variables based on file type
        if let Some(extra_env) = self.get_extra_env(config, file_type) {
            for (key, value) in extra_env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
}