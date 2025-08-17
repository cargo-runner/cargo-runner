//! Universal Command Runner CLI

use clap::{Parser, Subcommand};
use command_runner::{CommandRunner, Command};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "runner")]
#[command(about = "Universal command runner for any programming language")]
#[command(version = command_runner::VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run code at specific location
    Run {
        /// Target file (supports file.ext:line syntax)
        target: String,
    },
    
    /// Run tests
    Test {
        /// Target file (supports file.ext:line syntax)
        target: String,
    },
    
    /// Analyze file and list runnables
    Analyze {
        /// Target file
        target: String,
    },
    
    /// List available plugins
    #[command(name = "plugin-list")]
    PluginList,
}

fn main() {
    let cli = Cli::parse();
    
    let runner = match CommandRunner::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize: {}", e);
            process::exit(1);
        }
    };
    
    match cli.command {
        Commands::Run { target } => {
            let (path, line) = parse_target(&target);
            match runner.run(&path, line) {
                Ok(cmd) => execute_command(cmd),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        
        Commands::Test { target } => {
            let (path, line) = parse_target(&target);
            match runner.run(&path, line) {
                Ok(cmd) => execute_command(cmd),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        
        Commands::Analyze { target } => {
            let path = PathBuf::from(target);
            match runner.analyze(&path) {
                Ok(runnables) => {
                    println!("Found {} runnables:", runnables.len());
                    for (i, runnable) in runnables.iter().enumerate() {
                        println!("  {}. {} (lines {}-{})", 
                            i + 1, 
                            runnable.label,
                            runnable.line_start + 1,
                            runnable.line_end + 1
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        
        Commands::PluginList => {
            let plugins = runner.list_plugins();
            println!("Available plugins:");
            for plugin in plugins {
                println!("  - {}", plugin);
            }
        }
    }
}

fn parse_target(target: &str) -> (PathBuf, Option<u32>) {
    if let Some(colon_idx) = target.rfind(':') {
        let file_part = &target[..colon_idx];
        let line_part = &target[colon_idx + 1..];
        
        if let Ok(line) = line_part.parse::<u32>() {
            return (PathBuf::from(file_part), Some(line));
        }
    }
    
    (PathBuf::from(target), None)
}

fn execute_command(cmd: Command) {
    println!("Executing: {} {}", cmd.program, cmd.args.join(" "));
    
    if let Some(dir) = &cmd.working_dir {
        println!("Working directory: {}", dir.display());
    }
    
    let mut command = process::Command::new(&cmd.program);
    command.args(&cmd.args);
    
    if let Some(dir) = &cmd.working_dir {
        command.current_dir(dir);
    }
    
    for (key, value) in &cmd.env {
        command.env(key, value);
    }
    
    match command.status() {
        Ok(status) => {
            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
            process::exit(1);
        }
    }
}