//! Test command builder with test framework support

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};
use std::path::Path;

/// Test command builder with test framework support
pub struct TestCommandBuilder;

impl ConfigAccess for TestCommandBuilder {}
impl CargoBuilderHelper for TestCommandBuilder {}

impl CommandBuilderImpl for TestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let builder = TestCommandBuilder;
        let mut args = vec![];

        // Handle test framework configuration
        if let Some(test_framework) = builder.get_test_framework(config, file_type) {
            // Add channel
            if let Some(channel) = &test_framework.channel {
                args.push(format!("+{}", channel));
            } else if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }

            // Add subcommand
            if let Some(subcommand) = &test_framework.subcommand {
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                args.push("test".to_string());
            }

            // Add framework features
            if let Some(features) = &test_framework.features {
                args.extend(features.to_args());
            }

            // Add framework args
            if let Some(framework_args) = &test_framework.extra_args {
                args.extend(framework_args.clone());
            }
        } else {
            // Standard test command
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }
            args.push("test".to_string());
        }

        // Add package
        if let Some(pkg) = package {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        // Add target
        builder.add_target(&mut args, &runnable.file_path, package)?;

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add test filter
        builder.add_test_filter(&mut args, runnable, config, file_type);

        let mut command = CargoCommand::new(args);

        // Apply test framework env
        if let Some(test_framework) = builder.get_test_framework(config, file_type) {
            if let Some(extra_env) = &test_framework.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }

        builder.apply_common_config(
            &mut command,
            config,
            file_type,
            builder.get_extra_env(config, file_type),
        );
        builder.apply_env(&mut command, runnable, config, file_type);

        Ok(command)
    }
}

impl TestCommandBuilder {
    fn add_target(
        &self,
        args: &mut Vec<String>,
        file_path: &Path,
        _package: Option<&str>,
    ) -> Result<()> {
        // For integration tests, add --test flag
        if let Some(parent) = file_path.parent() {
            if parent.ends_with("tests") || parent.to_str().map_or(false, |s| s.contains("/tests/"))
            {
                if let Some(stem) = file_path.file_stem() {
                    args.push("--test".to_string());
                    args.push(stem.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }

    fn add_test_filter(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        if let RunnableKind::Test { test_name, .. } = &runnable.kind {
            args.push("--".to_string());
            args.push(test_name.clone());

            // Apply test binary args
            self.apply_test_binary_args(args, runnable, config, file_type);
        }
    }

    fn apply_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply features first
        self.apply_features(
            args,
            runnable,
            config,
            file_type,
            self.get_features(config, file_type),
        );

        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_test_binary_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override test binary args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global test binary args
        if let Some(extra_args) = self.get_extra_test_binary_args(config, file_type) {
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
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_env) = &override_cargo.extra_env {
                    for (key, value) in extra_env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
    }
}
