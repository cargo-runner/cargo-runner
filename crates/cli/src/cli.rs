use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    analyze_command, init_command, run_command, unset_command,
};

#[derive(Parser)]
#[command(bin_name = "cargo")]
#[command(version, propagate_version = true)]
pub struct Cargo {
    #[command(subcommand)]
    pub command: CargoCommand,
}

#[derive(Subcommand, Debug)]
pub enum CargoCommand {
    #[command(name = "runner")]
    #[command(about = "Run Rust code at specific locations")]
    Runner(Runner),
}

#[derive(Parser, Debug)]
#[command(name = "cargo-runner")]
#[command(version, about, long_about = None)]
pub struct Runner {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a Rust file and list all runnable items
    #[command(visible_alias = "a")]
    Analyze {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10)
        filepath: String,

        /// Show verbose output with command details
        #[arg(short, long)]
        verbose: bool,

        /// Show current configuration
        #[arg(short, long)]
        config: bool,
    },
    /// Run Rust code at a specific location
    #[command(visible_alias = "r")]
    Run {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10)
        filepath: String,

        /// Print the command without executing it
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Initialize cargo-runner configuration
    Init {
        /// Specify the current working directory
        #[arg(short, long)]
        cwd: Option<String>,

        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,

        /// Add rustc configuration for standalone Rust files
        #[arg(long)]
        rustc: bool,

        /// Add single-file script configuration
        #[arg(long)]
        single_file_script: bool,
    },
    /// Remove cargo-runner configuration
    Unset {
        /// Clean up all generated configuration files
        #[arg(short, long)]
        clean: bool,
    },
}

impl Commands {
    /// Execute the command
    pub fn execute(self) -> Result<()> {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cargo-runner-execute.log")
        {
            writeln!(f, "DEBUG Commands::execute called with: {:?}", self).ok();
        }

        match self {
            Commands::Analyze {
                filepath,
                verbose,
                config,
            } => analyze_command(&filepath, verbose, config),
            Commands::Run { filepath, dry_run } => {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .open("/tmp/cargo-runner-execute.log")
                {
                    writeln!(
                        f,
                        "DEBUG calling run_command with filepath={:?}, dry_run={}",
                        filepath, dry_run
                    )
                    .ok();
                }
                run_command(&filepath, dry_run)
            }
            Commands::Init {
                cwd,
                force,
                rustc,
                single_file_script,
            } => init_command(cwd.as_deref(), force, rustc, single_file_script),
            Commands::Unset { clean } => unset_command(clean),
        }
    }
}
