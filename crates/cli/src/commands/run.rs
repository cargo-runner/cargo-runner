use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::utils::parse_filepath_with_line;

pub fn run_command(filepath_arg: &str, dry_run: bool) -> Result<()> {
    // Parse filepath and line number
    let (filepath, line) = parse_filepath_with_line(filepath_arg);

    debug!("Running file: {} at line: {:?}", filepath, line);

    let mut runner = cargo_runner_core::CargoRunner::new()?;
    let command = runner.get_command_at_position_with_dir(&filepath, line)?;

    if dry_run {
        println!("{}", command.to_shell_command());
        if let Some(ref dir) = command.working_dir {
            println!("Working directory: {}", dir);
        }
        if !command.env.is_empty() {
            println!("Environment variables:");
            for (key, value) in &command.env {
                println!("  {}={}", key, value);
            }
        }
    } else {
        let shell_cmd = command.to_shell_command();
        info!("Running: {}", shell_cmd);
        if let Some(ref dir) = command.working_dir {
            info!("Working directory: {}", dir);
        }

        // Execute using the CargoCommand's execute method which handles working_dir
        let status = command
            .execute()
            .with_context(|| format!("Failed to execute: {}", shell_cmd))?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}
