//! Binary command builder with binary framework support

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand, CommandType,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable},
};

/// Binary command builder with binary framework support
pub struct BinaryCommandBuilder;

impl ConfigAccess for BinaryCommandBuilder {}
impl CargoBuilderHelper for BinaryCommandBuilder {}

impl CommandBuilderImpl for BinaryCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let builder = BinaryCommandBuilder;
        let mut args = vec![];
        let mut command_type = CommandType::Cargo;

        // Handle binary framework configuration
        if let Some(binary_framework) = builder.get_binary_framework(config, file_type) {
            // Handle custom command
            if let Some(cmd) = &binary_framework.command {
                if cmd != "cargo" {
                    command_type = CommandType::Shell;
                    args.push(cmd.clone());

                    // Add subcommand if specified
                    if let Some(subcommand) = &binary_framework.subcommand {
                        args.extend(subcommand.split_whitespace().map(String::from));
                    }
                } else {
                    // Standard cargo with channel
                    if let Some(channel) = &binary_framework.channel {
                        args.push(format!("+{}", channel));
                    } else if let Some(channel) = builder.get_channel(config, file_type) {
                        args.push(format!("+{}", channel));
                    }

                    // Add subcommand
                    if let Some(subcommand) = &binary_framework.subcommand {
                        args.extend(subcommand.split_whitespace().map(String::from));
                    } else {
                        args.push("run".to_string());
                    }
                }
            } else {
                // No custom command, use standard cargo
                if let Some(channel) = &binary_framework.channel {
                    args.push(format!("+{}", channel));
                } else if let Some(channel) = builder.get_channel(config, file_type) {
                    args.push(format!("+{}", channel));
                }

                if let Some(subcommand) = &binary_framework.subcommand {
                    args.extend(subcommand.split_whitespace().map(String::from));
                } else {
                    args.push("run".to_string());
                }
            }

            // Add framework features (only for cargo commands)
            if command_type == CommandType::Cargo {
                if let Some(features) = &binary_framework.features {
                    args.extend(features.to_args());
                }
            }

            // Add framework args
            if let Some(framework_args) = &binary_framework.extra_args {
                args.extend(framework_args.clone());
            }
        } else {
            // Standard binary command
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }
            args.push("run".to_string());
        }

        // Add package (only for cargo commands)
        if command_type == CommandType::Cargo {
            if let Some(pkg) = package {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }

            // Add binary name
            builder.add_binary_name(&mut args, runnable)?;
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        let mut command = match command_type {
            CommandType::Shell => {
                // Extract the command from args[0]
                let cmd = args[0].clone();
                let cmd_args = args[1..].to_vec();
                CargoCommand::new_shell(cmd, cmd_args)
            }
            _ => CargoCommand::new(args),
        };

        // Apply binary framework env
        if let Some(binary_framework) = builder.get_binary_framework(config, file_type) {
            if let Some(extra_env) = &binary_framework.extra_env {
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

impl BinaryCommandBuilder {
    fn add_binary_name(&self, args: &mut Vec<String>, runnable: &Runnable) -> Result<()> {
        if let Some(stem) = runnable.file_path.file_stem() {
            let stem_str = stem.to_string_lossy();
            if stem_str != "main" {
                args.push("--bin".to_string());
                args.push(stem_str.to_string());
            }
        }
        Ok(())
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
