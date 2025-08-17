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
        tracing::debug!(
            "BinaryCommandBuilder::build called for {:?}, package={:?}",
            runnable.file_path,
            package
        );
        let builder = BinaryCommandBuilder;
        let mut args = vec![];
        let mut command_type = CommandType::Cargo;

        // NUKE-CONFIG: Removed all binary framework and override command logic
        // TODO: Add back support for dioxus/leptos/tauri with simple tool selection
        // For now, just use cargo run
        args.push("run".to_string());
        let _ = (config, file_type); // Suppress warnings

        // Add package (only for cargo commands)
        if command_type == CommandType::Cargo {
            if let Some(pkg) = package {
                if !pkg.is_empty() {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
            }

            // NUKE-CONFIG: Simplified binary name logic - always use default run
            // Add binary name
            let is_default_run = true;
            builder.add_target(&mut args, runnable, package, is_default_run)?;
        }

        // NUKE-CONFIG: Removed apply_args
        // TODO: Add simple extra_args support later

        let mut command = match command_type {
            CommandType::Shell => {
                // Extract the command from args[0]
                let cmd = args[0].clone();
                let cmd_args = args[1..].to_vec();
                CargoCommand::new_shell(cmd, cmd_args)
            }
            _ => CargoCommand::new(args),
        };

        // Set working directory to cargo root for all commands
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        // NUKE-CONFIG: Removed binary framework env

        // NUKE-CONFIG: Removed all env configuration
        // TODO: Add back simple env vars if needed

        Ok(command)
    }
}

impl BinaryCommandBuilder {
    fn add_target(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        package: Option<&str>,
        is_default_run: bool,
    ) -> Result<()> {
        tracing::debug!(
            "add_target: is_default_run={}, package={:?}, file_path={:?}",
            is_default_run,
            package,
            runnable.file_path
        );

        let path_str = runnable.file_path.to_str().unwrap_or("");

        // Check if this is an example file
        if path_str.contains("/examples/") {
            self.add_example(args, runnable)?;
            return Ok(());
        }

        // Otherwise, handle as binary
        self.add_binary(args, runnable, package, is_default_run)?;
        Ok(())
    }

    fn add_example(&self, args: &mut Vec<String>, runnable: &Runnable) -> Result<()> {
        if let Some(stem) = runnable.file_path.file_stem() {
            let example_name = stem.to_string_lossy();
            tracing::debug!(
                "Adding --example {} (file in examples/ directory)",
                example_name
            );
            args.push("--example".to_string());
            args.push(example_name.to_string());
        }
        Ok(())
    }

    fn add_binary(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        package: Option<&str>,
        is_default_run: bool,
    ) -> Result<()> {
        // First check if we have an explicit binary name in the runnable
        if let crate::types::RunnableKind::Binary { bin_name } = &runnable.kind {
            if let Some(name) = bin_name {
                tracing::debug!("Using explicit bin_name: {}", name);
                args.push("--bin".to_string());
                args.push(name.clone());
                return Ok(());
            }
        }

        // For src/main.rs, we might need to add --bin with the package name
        // if there are multiple binaries in the project
        if let Some(stem) = runnable.file_path.file_stem() {
            let stem_str = stem.to_string_lossy();
            tracing::debug!("File stem: {}", stem_str);
            if stem_str != "main" {
                tracing::debug!("Adding --bin {} (non-main file)", stem_str);
                args.push("--bin".to_string());
                args.push(stem_str.to_string());
            } else {
                // For main.rs with default cargo run command, use the package name as the binary name
                // This handles the case where there are multiple binaries in the project
                if is_default_run && package.is_some() {
                    tracing::debug!(
                        "Adding --bin {} (main.rs with package name)",
                        package.unwrap()
                    );
                    args.push("--bin".to_string());
                    args.push(package.unwrap().to_string());
                } else {
                    tracing::debug!(
                        "Not adding --bin: is_default_run={}, package={:?}",
                        is_default_run,
                        package
                    );
                }
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
        // NUKE-CONFIG: Removed all config application
        let _ = (runnable, config, file_type); // Suppress warnings
        // TODO: Add simple extra_args support later
    }

    fn apply_env(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // NUKE-CONFIG: Removed all env configuration
        let _ = (command, runnable, config, file_type); // Suppress warnings
    }
}
