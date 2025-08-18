//! Benchmark command builder

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
    ) -> Result<CargoCommand> {
        let builder = BenchmarkCommandBuilder;
        let mut args = vec![];

        // NUKE-CONFIG: Removed channel selection
        // TODO: Add back criterion support with simple tool selection
        args.push("bench".to_string());
        let _ = (config, file_type); // Suppress warnings

        // Add package
        if let Some(pkg) = package {
            if !pkg.is_empty() {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }
        }

        // NUKE-CONFIG: Removed apply_args
        // TODO: Add simple extra_args support later

        // Add benchmark filter
        if let RunnableKind::Benchmark { bench_name } = &runnable.kind {
            args.push("--".to_string());
            args.push(bench_name.clone());
        }

        let mut command = CargoCommand::new(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        // NUKE-CONFIG: Removed all env configuration

        Ok(command)
    }
}

impl BenchmarkCommandBuilder {
    // NUKE-CONFIG: Removed apply_args and apply_env methods
    // TODO: Add simple configuration support when new config is ready
}
