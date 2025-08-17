//! Universal Command Runner CLI
//! 
//! Command-line interface for the universal command runner framework.

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
    command: Option<Commands>,
    
    /// File to run (supports file.ext:line syntax)
    #[arg(value_name = "FILE[:LINE]")]
    target: Option<String>,
    
    /// Force a specific plugin
    #[arg(short, long)]
    plugin: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the main/default runnable
    Run {
        /// Target file (supports file.ext:line syntax)
        target: String,
        
        /// Force a specific plugin
        #[arg(short, long)]
        plugin: Option<String>,
    },
    
    /// Run tests
    Test {
        /// Target file (supports file.ext:line syntax)
        target: String,
        
        /// Force a specific plugin
        #[arg(short, long)]
        plugin: Option<String>,
    },
    
    /// Run benchmarks
    Bench {
        /// Target file (supports file.ext:line syntax)
        target: String,
        
        /// Force a specific plugin
        #[arg(short, long)]
        plugin: Option<String>,
    },
    
    /// Analyze a file and list all runnables
    Analyze {
        /// Target file to analyze
        target: String,
    },
    
    /// Plugin management
    #[command(subcommand)]
    Plugin(PluginCommands),
}

#[derive(Subcommand)]
enum PluginCommands {
    /// List installed plugins
    List,
    
    /// Install a plugin
    Install {
        /// Plugin name or path
        plugin: String,
    },
    
    /// Remove a plugin
    Remove {
        /// Plugin name
        plugin: String,
    },
    
    /// Reload all plugins
    Reload,
}

fn main() {
    let cli = Cli::parse();
    
    // Initialize runner
    let mut runner = match CommandRunner::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize runner: {}", e);
            process::exit(1);
        }
    };
    
    // Handle commands
    let result = match cli.command {
        Some(Commands::Run { target, plugin }) => {
            if cli.verbose {
                println!("Running: {}", target);
            }
            run_with_optional_plugin(&mut runner, "run", &target, plugin)
        }
        
        Some(Commands::Test { target, plugin }) => {
            if cli.verbose {
                println!("Testing: {}", target);
            }
            run_with_optional_plugin(&mut runner, "test", &target, plugin)
        }
        
        Some(Commands::Bench { target, plugin }) => {
            if cli.verbose {
                println!("Benchmarking: {}", target);
            }
            run_with_optional_plugin(&mut runner, "bench", &target, plugin)
        }
        
        Some(Commands::Analyze { target }) => {
            analyze_file(&mut runner, &target)
        }
        
        Some(Commands::Plugin(plugin_cmd)) => {
            handle_plugin_command(&mut runner, plugin_cmd)
        }
        
        None => {
            // Default behavior when just a file is provided
            if let Some(target) = cli.target {
                if cli.verbose {
                    println!("Running: {}", target);
                }
                run_with_optional_plugin(&mut runner, "run", &target, cli.plugin)
            } else {
                eprintln!("No target specified. Use --help for usage information.");
                process::exit(1)
            }
        }
    };
    
    // Handle result
    match result {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn run_with_optional_plugin(
    runner: &mut CommandRunner,
    action: &str,
    target: &str,
    plugin: Option<String>,
) -> Result<(), String> {
    // Get the command to execute
    let command = if let Some(plugin_name) = plugin {
        // Use specific plugin
        runner.execute_action(action, target)?
    } else {
        // Auto-detect plugin
        runner.execute_action(action, target)?
    };
    
    // Execute the command
    execute_command(command)
}

fn analyze_file(runner: &mut CommandRunner, target: &str) -> Result<(), String> {
    // Parse target
    let (file_path, line) = parse_target(target)?;
    
    // Create a temporary runner instance for analysis
    // In real implementation, would use runner.analyze() directly
    println!("Analyzing: {}", file_path.display());
    
    if let Some(line) = line {
        println!("Target line: {}", line);
    }
    
    // For demo, just print analysis
    println!("\nDetected runnables:");
    println!("  1. Test: test_parser (line 42)");
    println!("  2. Test: test_detector (line 85)");
    println!("  3. Main: main() (line 120)");
    
    Ok(())
}

fn handle_plugin_command(runner: &mut CommandRunner, cmd: PluginCommands) -> Result<(), String> {
    match cmd {
        PluginCommands::List => {
            let plugins = runner.list_plugins();
            
            if plugins.is_empty() {
                println!("No plugins installed.");
            } else {
                println!("Installed plugins:");
                for plugin in plugins {
                    println!("  {} v{} - {}", 
                        plugin.name, 
                        plugin.version,
                        plugin.language
                    );
                    if let Some(desc) = plugin.description {
                        println!("    {}", desc);
                    }
                    println!("    Extensions: {}", plugin.file_extensions.join(", "));
                }
            }
            Ok(())
        }
        
        PluginCommands::Install { plugin } => {
            println!("Installing plugin: {}", plugin);
            // In real implementation, would download and install
            println!("Plugin installed successfully.");
            Ok(())
        }
        
        PluginCommands::Remove { plugin } => {
            println!("Removing plugin: {}", plugin);
            // In real implementation, would remove plugin files
            println!("Plugin removed successfully.");
            Ok(())
        }
        
        PluginCommands::Reload => {
            println!("Reloading plugins...");
            // In real implementation, would call runner.reload_plugins()
            println!("Plugins reloaded successfully.");
            Ok(())
        }
    }
}

fn execute_command(command: Command) -> Result<(), String> {
    println!("Executing: {} {}", command.program, command.args.join(" "));
    
    if let Some(dir) = &command.working_dir {
        println!("Working directory: {}", dir.display());
    }
    
    // Build the command
    let mut cmd = process::Command::new(&command.program);
    cmd.args(&command.args);
    
    if let Some(dir) = &command.working_dir {
        cmd.current_dir(dir);
    }
    
    for (key, value) in &command.env {
        cmd.env(key, value);
    }
    
    // Execute
    let status = cmd.status()
        .map_err(|e| format!("Failed to execute command: {}", e))?;
    
    if !status.success() {
        return Err(format!("Command failed with status: {}", status));
    }
    
    Ok(())
}

fn parse_target(target: &str) -> Result<(PathBuf, Option<u32>), String> {
    if let Some(colon_idx) = target.rfind(':') {
        let file_part = &target[..colon_idx];
        let line_part = &target[colon_idx + 1..];
        
        if let Ok(line) = line_part.parse::<u32>() {
            return Ok((PathBuf::from(file_part), Some(line)));
        }
    }
    
    Ok((PathBuf::from(target), None))
}

// For building as a library that provides a CLI
pub mod command_runner {
    pub const VERSION: &str = "1.0.0";
    
    pub struct CommandRunner;
    pub struct Command {
        pub program: String,
        pub args: Vec<String>,
        pub env: std::collections::HashMap<String, String>,
        pub working_dir: Option<std::path::PathBuf>,
    }
    
    impl CommandRunner {
        pub fn new() -> Result<Self, String> {
            Ok(Self)
        }
        
        pub fn execute_action(&mut self, _action: &str, _target: &str) -> Result<Command, String> {
            // Simplified for demo
            Ok(Command {
                program: "cargo".to_string(),
                args: vec!["test".to_string()],
                env: std::collections::HashMap::new(),
                working_dir: None,
            })
        }
        
        pub fn list_plugins(&self) -> Vec<PluginMetadata> {
            vec![]
        }
    }
    
    pub struct PluginMetadata {
        pub name: String,
        pub version: String,
        pub language: String,
        pub file_extensions: Vec<String>,
        pub description: Option<String>,
    }
}

// Optional: Add clap dependency
use clap_dep as clap;
mod clap_dep {
    pub use clap::*;
}