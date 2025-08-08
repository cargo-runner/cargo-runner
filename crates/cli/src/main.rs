use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::Command;
use tracing::debug;

/// A tool for analyzing and running Rust code
#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(subcommand_required = true, arg_required_else_help = true)]
struct Cargo {
    #[command(subcommand)]
    command: CargoCommand,
}

#[derive(Subcommand)]
enum CargoCommand {
    /// Analyze and run Rust code at specific locations
    Runner(Runner),
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(after_help = "ENVIRONMENT:\n    RUST_LOG=debug    Enable debug logging")]
struct Runner {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze runnables in a file
    Analyze {
        /// Path to the Rust file to analyze
        filepath: String,
    },
    /// Run code at specific location
    Run {
        /// Path to the Rust file with optional line number (e.g., src/lib.rs:42)
        filepath: String,
        
        /// Show command without executing
        #[arg(short = 'd', long = "dry-run")]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    // Initialize tracing based on RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Check if we're being run as a cargo subcommand
    let args: Vec<String> = std::env::args().collect();
    
    // If invoked as "cargo-runner" directly
    if !args.is_empty() && args[0].ends_with("cargo-runner") {
        // Check if the next arg is "runner" (from cargo invocation)
        if args.len() > 1 && args[1] == "runner" {
            // Being invoked as "cargo runner", parse as cargo subcommand
            let cargo = Cargo::parse();
            let CargoCommand::Runner(runner) = cargo.command;
            match runner.command {
                Commands::Analyze { filepath } => analyze_command(&filepath),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
            }
        } else {
            // Being invoked directly as "cargo-runner"
            let runner = Runner::parse();
            match runner.command {
                Commands::Analyze { filepath } => analyze_command(&filepath),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
            }
        }
    } else {
        // Fallback to direct parsing
        let runner = Runner::parse();
        match runner.command {
            Commands::Analyze { filepath } => analyze_command(&filepath),
            Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
        }
    }
}

fn analyze_command(filepath: &str) -> Result<()> {
    debug!("Analyzing file: {}", filepath);
    
    let mut runner = cargo_runner_core::CargoRunner::new()?;
    let runnables = runner.analyze(filepath)?;
    
    // Print the analysis results
    println!("{runnables}");
    
    Ok(())
}

fn run_command(filepath_arg: &str, dry_run: bool) -> Result<()> {
    // Parse filepath and line number
    let (filepath, line) = if let Some(colon_pos) = filepath_arg.rfind(':') {
        let path_part = &filepath_arg[..colon_pos];
        let line_part = &filepath_arg[colon_pos + 1..];
        
        // Check if it's a valid line number
        if let Ok(line_num) = line_part.parse::<usize>() {
            // Convert 1-based to 0-based
            (path_part.to_string(), Some(line_num.saturating_sub(1)))
        } else {
            // Not a valid line number, treat the whole thing as a path
            (filepath_arg.to_string(), None)
        }
    } else {
        (filepath_arg.to_string(), None)
    };
    
    debug!("Running file: {} at line: {:?}", filepath, line);
    
    let mut runner = cargo_runner_core::CargoRunner::new()?;
    let command = runner.get_command_at_position(&filepath, line)?;
    
    if dry_run {
        println!("{command}");
    } else {
        println!("Running: {command}");
        
        // Parse and execute the command
        let mut parts = command.split_whitespace();
        let cmd = parts.next().context("Empty command")?;
        let args: Vec<&str> = parts.collect();
        
        let status = Command::new(cmd)
            .args(&args)
            .status()
            .with_context(|| format!("Failed to execute: {command}"))?;
        
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }
    
    Ok(())
}