use anyhow::{Context, Result};
use std::env;
use std::process::Command;
use tracing::debug;

fn main() -> Result<()> {
    // Initialize tracing based on RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut args: Vec<String> = env::args().collect();
    
    // When invoked as "cargo runner", cargo passes "runner" as the first argument
    // So we need to skip it if present
    if args.len() > 1 && args[1] == "runner" {
        args.remove(1);
    }

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "analyze" => analyze_command(&args[2..]),
        "run" => run_command(&args[2..]),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("cargo-runner - A tool for analyzing and running Rust code");
    println!();
    println!("USAGE:");
    println!("    cargo runner <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    analyze <filepath>           Analyze runnables in a file");
    println!("    run <filepath[:line]>        Run code at specific location");
    println!();
    println!("OPTIONS:");
    println!("    -d, --dry-run               Show command without executing");
    println!("    -h, --help                  Print help information");
    println!();
    println!("ENVIRONMENT:");
    println!("    RUST_LOG=debug              Enable debug logging");
}

fn analyze_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: Missing filepath argument");
        eprintln!("Usage: cargo runner analyze <filepath>");
        std::process::exit(1);
    }

    let filepath = &args[0];
    debug!("Analyzing file: {}", filepath);

    let mut runner = cargo_runner_core::CargoRunner::new()?;
    let runnables = runner.analyze(filepath)?;

    // Print the analysis results (similar to cargo-r)
    println!("{runnables}");

    Ok(())
}

fn run_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Error: Missing filepath argument");
        eprintln!("Usage: cargo runner run <filepath[:line]> [OPTIONS]");
        std::process::exit(1);
    }

    let mut dry_run = false;
    let mut filepath_arg = None;

    for arg in args {
        match arg.as_str() {
            "-d" | "--dry-run" => dry_run = true,
            _ => {
                if filepath_arg.is_none() {
                    filepath_arg = Some(arg.clone());
                }
            }
        }
    }

    let filepath_arg = filepath_arg.context("Missing filepath argument")?;

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
            (filepath_arg, None)
        }
    } else {
        (filepath_arg, None)
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
