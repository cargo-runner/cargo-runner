//! Benchmark command builder

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        Command,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

/// Benchmark command builder
pub struct BenchmarkCommandBuilder;

impl ConfigAccess for BenchmarkCommandBuilder {}
impl CargoBuilderHelper for BenchmarkCommandBuilder {}

impl CommandBuilderImpl for BenchmarkCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
        let builder = BenchmarkCommandBuilder;
        let mut args = vec![];
        let mut strategy = crate::command::CommandStrategy::Cargo;

        let override_cmd = builder.apply_cargo_override_command(
            &mut args,
            runnable,
            config,
            file_type,
            "bench",
        );

        if let Some((strat, _)) = override_cmd {
            strategy = strat;
        } else {
            // Add channel
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{channel}"));
            }
            args.push("bench".to_string());
        }

        // Add package
        if strategy == crate::command::CommandStrategy::Cargo
            && let Some(pkg) = package
            && !pkg.is_empty()
        {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add benchmark filter
        if strategy == crate::command::CommandStrategy::Cargo
            && let RunnableKind::Benchmark { bench_name } = &runnable.kind
        {
            args.push("--".to_string());
            args.push(bench_name.clone());
        }

        let mut command = match strategy {
            crate::command::CommandStrategy::Shell => {
                let program = args.first().cloned().unwrap_or_else(|| "cargo".into());
                let rest = if args.len() > 1 {
                    args[1..].to_vec()
                } else {
                    vec![]
                };
                Command::shell(program, rest)
            }
            _ => Command::cargo(args),
        };

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
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

impl BenchmarkCommandBuilder {
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
        if let Some(override_config) = self.get_override(runnable, config, file_type)
            && let Some(override_cargo) = &override_config.cargo
            && let Some(extra_args) = &override_cargo.extra_args
        {
            args.extend(extra_args.clone());
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_env(
        &self,
        command: &mut Command,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type)
            && let Some(override_cargo) = &override_config.cargo
            && let Some(extra_env) = &override_cargo.extra_env
        {
            for (key, value) in extra_env {
                command.env.insert(key.clone(), value.clone());
            }
        }
    }
}
