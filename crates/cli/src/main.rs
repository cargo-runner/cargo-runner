use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;
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
        
        /// Show verbose JSON output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
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
                Commands::Analyze { filepath, verbose } => analyze_command(&filepath, verbose),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
            }
        } else {
            // Being invoked directly as "cargo-runner"
            let runner = Runner::parse();
            match runner.command {
                Commands::Analyze { filepath, verbose } => analyze_command(&filepath, verbose),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
            }
        }
    } else {
        // Fallback to direct parsing
        let runner = Runner::parse();
        match runner.command {
            Commands::Analyze { filepath, verbose } => analyze_command(&filepath, verbose),
            Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
        }
    }
}

fn analyze_command(filepath: &str, verbose: bool) -> Result<()> {
    debug!("Analyzing file: {}", filepath);
    
    let mut runner = cargo_runner_core::CargoRunner::new()?;
    
    if verbose {
        // Show JSON output for verbose mode
        let runnables = runner.analyze(filepath)?;
        println!("{runnables}");
    } else {
        // Show formatted output
        print_formatted_analysis(&mut runner, filepath)?;
    }
    
    Ok(())
}

fn print_formatted_analysis(runner: &mut cargo_runner_core::CargoRunner, filepath: &str) -> Result<()> {
    println!("🔍 Analyzing: {}", filepath);
    println!("{}", "=".repeat(80));
    
    // Get file-level command
    let path = Path::new(filepath);
    if let Some(cmd) = runner.get_file_command(path)? {
        println!("📄 File-level command:");
        print_command_breakdown(&cmd);
        
        // Determine file type
        let file_type = determine_file_type(path);
        println!("   📦 Type: {}", file_type);
        
        // Get file scope info
        if let Ok(source) = std::fs::read_to_string(path) {
            let line_count = source.lines().count();
            println!("   📏 Scope: lines 1-{}", line_count);
        }
    }
    
    // Get all runnables
    let runnables = runner.detect_all_runnables(path)?;
    
    if runnables.is_empty() {
        println!("\n❌ No specific runnables found in this file.");
    } else {
        println!("\n✅ Found {} runnable(s):\n", runnables.len());
        
        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);
            
            // Show scope with 1-based line numbers
            println!("   📏 Scope: lines {}-{}", 
                runnable.scope.start.line + 1,
                runnable.scope.end.line + 1
            );
            
            // Show attributes if present
            if let Some(ref extended) = runnable.extended_scope {
                if extended.attribute_lines > 0 {
                    println!("   🏷️  Attributes: {} lines", extended.attribute_lines);
                }
                if extended.has_doc_tests {
                    println!("   🧪 Contains doc tests");
                }
            }
            
            // Build and show command
            if let Some(command) = runner.build_command_for_runnable(runnable)? {
                print_command_breakdown(&command);
            }
            
            // Show type
            print!("   📦 Type: ");
            print_runnable_type(&runnable.kind);
            
            // Show module path
            if !runnable.module_path.is_empty() {
                println!("   📁 Module path: {}", runnable.module_path);
            }
            
            if i < runnables.len() - 1 {
                println!();
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    Ok(())
}

fn determine_file_type(path: &Path) -> String {
    let path_str = path.to_str().unwrap_or("");
    
    if path_str.ends_with("/src/lib.rs") || path_str == "src/lib.rs" {
        "Library (lib.rs)".to_string()
    } else if path_str.ends_with("/src/main.rs") || path_str == "src/main.rs" {
        "Binary (main.rs)".to_string()
    } else if path_str.contains("/src/bin/") {
        format!("Binary '{}'", path.file_stem().unwrap_or_default().to_str().unwrap_or(""))
    } else if path_str.contains("/tests/") {
        format!("Integration test '{}'", path.file_stem().unwrap_or_default().to_str().unwrap_or(""))
    } else if path_str.contains("/benches/") {
        format!("Benchmark '{}'", path.file_stem().unwrap_or_default().to_str().unwrap_or(""))
    } else if path_str.contains("/examples/") {
        format!("Example '{}'", path.file_stem().unwrap_or_default().to_str().unwrap_or(""))
    } else if path_str.contains("/src/") || path_str.starts_with("src/") {
        "Library module".to_string()
    } else {
        "Rust file".to_string()
    }
}

fn print_command_breakdown(command: &cargo_runner_core::CargoCommand) {
    // Parse the command arguments
    let args = &command.args;
    
    // Extract components
    let (subcommand, package, extra_args, test_binary_args) = parse_cargo_command(args);
    
    println!("   🔧 Command breakdown:");
    println!("      • command: cargo");
    
    if let Some(subcmd) = subcommand {
        println!("      • subcommand: {}", subcmd);
    }
    
    if let Some(pkg) = package {
        println!("      • package: {}", pkg);
    }
    
    if !extra_args.is_empty() {
        println!("      • extraArgs: {:?}", extra_args);
    }
    
    if !test_binary_args.is_empty() {
        println!("      • extraTestBinaryArgs: {:?}", test_binary_args);
    }
    
    println!("   🚀 Final command: {}", command.to_shell_command());
}

fn parse_cargo_command(args: &[String]) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let mut subcommand = None;
    let mut package = None;
    let mut extra_args = Vec::new();
    let mut test_binary_args = Vec::new();
    
    let mut i = 0;
    let mut after_separator = false;
    
    while i < args.len() {
        let arg = &args[i];
        
        if arg == "--" {
            after_separator = true;
            i += 1;
            continue;
        }
        
        if after_separator {
            test_binary_args.push(arg.clone());
        } else if subcommand.is_none() && !arg.starts_with('-') {
            subcommand = Some(arg.clone());
        } else if arg == "--package" || arg == "-p" {
            if i + 1 < args.len() {
                package = Some(args[i + 1].clone());
                i += 1;
            }
        } else if arg.starts_with("--package=") {
            package = Some(arg.strip_prefix("--package=").unwrap().to_string());
        } else if arg.starts_with('-') {
            // Skip the value if this is a known flag that takes a value
            if matches!(arg.as_str(), "--bin" | "--example" | "--test" | "--bench") {
                extra_args.push(arg.clone());
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                    extra_args.push(args[i].clone());
                }
            } else {
                extra_args.push(arg.clone());
            }
        }
        
        i += 1;
    }
    
    (subcommand, package, extra_args, test_binary_args)
}

fn print_runnable_type(kind: &cargo_runner_core::RunnableKind) {
    match kind {
        cargo_runner_core::RunnableKind::Test { test_name, is_async } => {
            print!("Test function '{}'", test_name);
            if *is_async {
                print!(" (async)");
            }
            println!();
        }
        cargo_runner_core::RunnableKind::DocTest { struct_or_module_name, method_name } => {
            print!("Doc test for '{}'", struct_or_module_name);
            if let Some(method) = method_name {
                print!("::{}", method);
            }
            println!();
        }
        cargo_runner_core::RunnableKind::Benchmark { bench_name } => {
            println!("Benchmark '{}'", bench_name);
        }
        cargo_runner_core::RunnableKind::Binary { bin_name } => {
            print!("Binary");
            if let Some(name) = bin_name {
                print!(" '{}'", name);
            }
            println!();
        }
        cargo_runner_core::RunnableKind::ModuleTests { module_name } => {
            println!("Test module '{}'", module_name);
        }
    }
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