use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

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
        /// Path to the Rust file to analyze with optional line number (e.g., src/lib.rs:42)
        filepath: String,

        /// Show verbose JSON output
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,

        /// Show configuration details (loaded configs and overrides)
        #[arg(short = 'c', long = "config")]
        config: bool,
    },
    /// Run code at specific location
    Run {
        /// Path to the Rust file with optional line number (e.g., src/lib.rs:42)
        filepath: String,

        /// Show command without executing
        #[arg(short = 'd', long = "dry-run")]
        dry_run: bool,
    },
    /// Initialize cargo-runner configuration for a workspace
    Init {
        /// Custom working directory (defaults to current directory)
        #[arg(long = "cwd")]
        cwd: Option<String>,

        /// Force overwrite existing configuration files
        #[arg(short = 'f', long = "force")]
        force: bool,

        /// Generate rustc configuration for standalone files
        #[arg(long = "rustc")]
        rustc: bool,

        /// Generate single-file-script configuration (for cargo scripts)
        #[arg(long = "single-file-script")]
        single_file_script: bool,
    },
    /// Unset PROJECT_ROOT and clean up configuration
    Unset {
        /// Also remove .cargo-runner.json files
        #[arg(long = "clean")]
        clean: bool,
    },
    /// Create override configuration for a specific file location
    Override {
        /// File path with optional line number (e.g., src/main.rs:10)
        filepath: String,

        /// Save to root config instead of project config
        #[arg(long = "root")]
        root: bool,

        /// Arguments to override (after --)
        #[arg(last = true)]
        override_args: Vec<String>,
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
                Commands::Analyze {
                    filepath,
                    verbose,
                    config,
                } => analyze_command(&filepath, verbose, config),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
                Commands::Init { cwd, force, rustc, single_file_script } => init_command(cwd.as_deref(), force, rustc, single_file_script),
                Commands::Unset { clean } => unset_command(clean),
                Commands::Override { filepath, root, override_args } => override_command(&filepath, root, override_args),
            }
        } else {
            // Being invoked directly as "cargo-runner"
            let runner = Runner::parse();
            match runner.command {
                Commands::Analyze {
                    filepath,
                    verbose,
                    config,
                } => analyze_command(&filepath, verbose, config),
                Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
                Commands::Init { cwd, force, rustc, single_file_script } => init_command(cwd.as_deref(), force, rustc, single_file_script),
                Commands::Unset { clean } => unset_command(clean),
                Commands::Override { filepath, root, override_args } => override_command(&filepath, root, override_args),
            }
        }
    } else {
        // Fallback to direct parsing
        let runner = Runner::parse();
        match runner.command {
            Commands::Analyze {
                filepath,
                verbose,
                config,
            } => analyze_command(&filepath, verbose, config),
            Commands::Run { filepath, dry_run } => run_command(&filepath, dry_run),
            Commands::Init { cwd, force, rustc, single_file_script } => init_command(cwd.as_deref(), force, rustc, single_file_script),
            Commands::Unset { clean } => unset_command(clean),
            Commands::Override { filepath, root, override_args } => override_command(&filepath, root, override_args),
        }
    }
}

fn analyze_command(filepath_arg: &str, verbose: bool, show_config: bool) -> Result<()> {
    debug!("Analyzing file: {}", filepath_arg);

    // Parse filepath and line number
    let (filepath, line) = parse_filepath_with_line(filepath_arg);

    let mut runner = cargo_runner_core::CargoRunner::new()?;

    if verbose {
        // Show JSON output for verbose mode
        if let Some(line_num) = line {
            let runnables = runner.analyze_at_line(&filepath, line_num)?;
            println!("{runnables}");
        } else {
            let runnables = runner.analyze(&filepath)?;
            println!("{runnables}");
        }
    } else {
        // Show formatted output
        print_formatted_analysis(&mut runner, &filepath, line, show_config)?;
    }

    Ok(())
}

fn print_formatted_analysis(
    runner: &mut cargo_runner_core::CargoRunner,
    filepath: &str,
    line: Option<usize>,
    show_config: bool,
) -> Result<()> {
    println!(
        "🔍 Analyzing: {}{}",
        filepath,
        if let Some(l) = line {
            format!(":{}", l + 1)
        } else {
            String::new()
        }
    );
    println!("{}", "=".repeat(80));

    let mut final_command: Option<String> = None;

    // Show config details if requested
    if show_config {
        print_config_details(runner, filepath)?;
    }

    let path = Path::new(filepath);

    // Always show file-level command as it represents the entire file scope
    // Get file-level command
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

    // Get runnables based on line number
    let mut runnables = if let Some(line_num) = line {
        runner.detect_runnables_at_line(path, line_num as u32)?
    } else {
        runner.detect_all_runnables(path)?
    };

    // When analyzing a specific line, filter to the most specific runnable
    if line.is_some() && runnables.len() > 1 {
        // For doc tests, prefer more specific ones (e.g., User::new over User)
        runnables.sort_by(|a, b| {
            // For doc tests, use extended scope size if available
            let a_size = if matches!(a.kind, cargo_runner_core::RunnableKind::DocTest { .. }) {
                if let Some(ref extended) = a.extended_scope {
                    extended.scope.end.line - extended.scope.start.line
                } else {
                    a.scope.end.line - a.scope.start.line
                }
            } else {
                a.scope.end.line - a.scope.start.line
            };

            let b_size = if matches!(b.kind, cargo_runner_core::RunnableKind::DocTest { .. }) {
                if let Some(ref extended) = b.extended_scope {
                    extended.scope.end.line - extended.scope.start.line
                } else {
                    b.scope.end.line - b.scope.start.line
                }
            } else {
                b.scope.end.line - b.scope.start.line
            };

            a_size.cmp(&b_size)
        });

        // Keep only the most specific runnable
        if let Some(most_specific) = runnables.first().cloned() {
            runnables = vec![most_specific];
        }
    }

    if runnables.is_empty() {
        if let Some(line_num) = line {
            println!(
                "\n❌ No specific runnables found at line {} (but file-level command above can be used).",
                line_num + 1
            );
        } else {
            println!(
                "\n❌ No specific runnables found in this file (but file-level command above can be used)."
            );
        }
    } else {
        println!("\n✅ Found {} runnable(s):\n", runnables.len());

        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);

            // Show scope with 1-based line numbers
            // For doc tests, show the extended scope if available
            if matches!(
                runnable.kind,
                cargo_runner_core::RunnableKind::DocTest { .. }
            ) {
                if let Some(ref extended) = runnable.extended_scope {
                    println!(
                        "   📏 Scope: lines {}-{}",
                        extended.scope.start.line + 1,
                        extended.scope.end.line + 1
                    );
                } else {
                    println!(
                        "   📏 Scope: lines {}-{}",
                        runnable.scope.start.line + 1,
                        runnable.scope.end.line + 1
                    );
                }
            } else {
                println!(
                    "   📏 Scope: lines {}-{}",
                    runnable.scope.start.line + 1,
                    runnable.scope.end.line + 1
                );
            }

            // Debug: show if this runnable contains the requested line
            if let Some(line_num) = line {
                let contains = runnable.scope.contains_line(line_num as u32);
                debug!(
                    "Runnable '{}' contains line {}? {}",
                    runnable.label,
                    line_num + 1,
                    contains
                );
            }

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
                // Store the final command
                final_command = Some(command.to_shell_command());
            }

            // Show matching override if config details requested
            if show_config {
                if let Some(override_config) = runner.get_override_for_runnable(runnable) {
                    println!("   🔀 Matched override:");
                    println!("      • match: {:?}", override_config.identity);
                    
                    // Show cargo config if present
                    if let Some(cargo) = &override_config.cargo {
                        println!("      • cargo config:");
                        if cargo.command.is_some() {
                            println!("        - command: {:?}", cargo.command);
                        }
                        if cargo.subcommand.is_some() {
                            println!("        - subcommand: {:?}", cargo.subcommand);
                        }
                        if let Some(features) = &cargo.features {
                            match features {
                                cargo_runner_core::config::Features::All(s) if s == "all" => {
                                    println!("        - features: all");
                                }
                                cargo_runner_core::config::Features::Selected(selected) => {
                                    println!("        - features: {:?}", selected);
                                }
                                _ => {}
                            }
                        }
                        if cargo.extra_args.is_some() {
                            println!("        - extra_args: {:?}", cargo.extra_args);
                        }
                        if cargo.extra_test_binary_args.is_some() {
                            println!("        - extra_test_binary_args: {:?}", cargo.extra_test_binary_args);
                        }
                        if cargo.extra_env.is_some() {
                            println!("        - extra_env: {:?}", cargo.extra_env);
                        }
                    }
                    
                    // Show rustc config if present
                    if let Some(rustc) = &override_config.rustc {
                        println!("      • rustc config:");
                        if rustc.test_framework.is_some() {
                            println!("        - test_framework: present");
                        }
                        if rustc.binary_framework.is_some() {
                            println!("        - binary_framework: present");
                        }
                        if rustc.benchmark_framework.is_some() {
                            println!("        - benchmark_framework: present");
                        }
                    }
                    
                    // Show single_file_script config if present
                    if let Some(sfs) = &override_config.single_file_script {
                        println!("      • single_file_script config:");
                        if sfs.extra_args.is_some() {
                            println!("        - extra_args: {:?}", sfs.extra_args);
                        }
                        if sfs.extra_test_binary_args.is_some() {
                            println!("        - extra_test_binary_args: {:?}", sfs.extra_test_binary_args);
                        }
                        if sfs.extra_env.is_some() {
                            println!("        - extra_env: {:?}", sfs.extra_env);
                        }
                    }
                }
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

    // Display final command at the end
    if let Some(cmd) = final_command {
        println!("\n🎯 Command to run:");
        println!("   {}", cmd);
    }

    println!("\n{}", "=".repeat(80));
    Ok(())
}

fn determine_file_type(path: &Path) -> String {
    let path_str = path.to_str().unwrap_or("");

    // Check if it's a standalone file (no Cargo.toml in parents)
    let has_cargo_toml = path.ancestors().any(|p| p.join("Cargo.toml").exists());

    if !has_cargo_toml {
        // Check if it's a cargo script file
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Some(first_line) = content.lines().next() {
                if first_line.starts_with("#!")
                    && first_line.contains("cargo")
                    && first_line.contains("-Zscript")
                {
                    return "Cargo script file".to_string();
                }
            }
        }
        return "Standalone Rust file".to_string();
    }

    if path_str.ends_with("/src/lib.rs") || path_str == "src/lib.rs" {
        "Library (lib.rs)".to_string()
    } else if path_str.ends_with("/src/main.rs") || path_str == "src/main.rs" {
        "Binary (main.rs)".to_string()
    } else if path_str.contains("/src/bin/") {
        format!(
            "Binary '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/tests/") {
        format!(
            "Integration test '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/benches/") {
        format!(
            "Benchmark '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/examples/") {
        format!(
            "Example '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/src/") || path_str.starts_with("src/") {
        "Library module".to_string()
    } else {
        "Rust file".to_string()
    }
}

fn print_command_breakdown(command: &cargo_runner_core::CargoCommand) {
    use cargo_runner_core::CommandType;
    
    println!("   🔧 Command breakdown:");
    
    match command.command_type {
        CommandType::Rustc => {
            println!("      • command: rustc");
            
            // Parse rustc-specific arguments
            let mut has_test = false;
            let mut has_crate_type = false;
            let mut crate_name = None;
            let mut output_name = None;
            let mut source_file = None;
            let mut extra_args = Vec::new();
            
            let mut i = 0;
            while i < command.args.len() {
                let arg = &command.args[i];
                
                if arg == "--test" {
                    has_test = true;
                } else if arg == "--crate-type" && i + 1 < command.args.len() {
                    has_crate_type = true;
                    i += 1; // Skip the value
                } else if arg == "--crate-name" && i + 1 < command.args.len() {
                    crate_name = Some(command.args[i + 1].clone());
                    i += 1;
                } else if arg == "-o" && i + 1 < command.args.len() {
                    output_name = Some(command.args[i + 1].clone());
                    i += 1;
                } else if !arg.starts_with('-') && source_file.is_none() {
                    source_file = Some(arg.clone());
                } else if arg.starts_with('-') {
                    extra_args.push(arg.clone());
                }
                
                i += 1;
            }
            
            if has_test {
                println!("      • mode: test");
            } else if has_crate_type {
                println!("      • mode: binary");
            }
            
            if let Some(name) = crate_name {
                println!("      • crate-name: {}", name);
            }
            
            if let Some(name) = output_name {
                println!("      • output: {}", name);
            }
            
            if let Some(file) = source_file {
                println!("      • source: {}", file);
            }
            
            if !extra_args.is_empty() {
                println!("      • extraArgs: {:?}", extra_args);
            }
            
            if let Some(test_filter) = &command.test_filter {
                println!("      • testFilter: {}", test_filter);
            }
            
            // Check for test binary args in env
            let has_test_extra_args = command.env.iter().find(|(k, _)| k == "_RUSTC_TEST_EXTRA_ARGS");
            if let Some((_, extra_args)) = has_test_extra_args {
                let args: Vec<&str> = extra_args.split_whitespace().collect();
                if !args.is_empty() {
                    println!("      • extraTestBinaryArgs: {:?}", args);
                }
            }
        }
        _ => {
            // Original cargo command parsing
            let args = &command.args;
            let (subcommand, package, extra_args, test_binary_args) = parse_cargo_command(args);
            
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
        }
    }

    // Show environment variables (excluding internal ones)
    if !command.env.is_empty() {
        let visible_env: Vec<_> = command.env.iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .collect();
        
        if !visible_env.is_empty() {
            println!("      • extraEnv:");
            for (key, value) in visible_env {
                println!("         - {}={}", key, value);
            }
        }
    }

    println!("   🚀 Final command: {}", command.to_shell_command());
}

fn parse_cargo_command(
    args: &[String],
) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
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
        } else if subcommand.is_none() && !arg.starts_with('-') && !arg.starts_with('+') {
            // Handle commands like "test", "run", etc.
            subcommand = Some(arg.clone());
        } else if arg.starts_with('+') && subcommand.is_none() {
            // Handle toolchain overrides like "+nightly"
            // This is part of cargo invocation, not a subcommand
            extra_args.push(arg.clone());
        } else if arg == "--package" || arg == "-p" {
            if i + 1 < args.len() {
                package = Some(args[i + 1].clone());
                i += 1;
            }
        } else if arg.starts_with("--package=") {
            package = Some(arg.strip_prefix("--package=").unwrap().to_string());
        } else if arg.starts_with('-') {
            // Skip the value if this is a known flag that takes a value
            if matches!(
                arg.as_str(),
                "--bin" | "--example" | "--test" | "--bench" | "--features"
            ) {
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
        cargo_runner_core::RunnableKind::Test {
            test_name,
            is_async,
        } => {
            print!("Test function '{}'", test_name);
            if *is_async {
                print!(" (async)");
            }
            println!();
        }
        cargo_runner_core::RunnableKind::DocTest {
            struct_or_module_name,
            method_name,
        } => {
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
        cargo_runner_core::RunnableKind::Standalone { has_tests } => {
            print!("Standalone Rust file");
            if *has_tests {
                print!(" (with tests)");
            }
            println!();
        }
        cargo_runner_core::RunnableKind::SingleFileScript { shebang } => {
            println!("Cargo script file");
            println!("   🔧 Shebang: {}", shebang);
        }
    }
}

fn parse_filepath_with_line(filepath_arg: &str) -> (String, Option<usize>) {
    if let Some(colon_pos) = filepath_arg.rfind(':') {
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
    }
}

fn run_command(filepath_arg: &str, dry_run: bool) -> Result<()> {
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

fn init_command(cwd: Option<&str>, force: bool, rustc: bool, single_file_script: bool) -> Result<()> {
    use walkdir::WalkDir;

    // Determine the project root
    let project_root = if let Some(cwd) = cwd {
        PathBuf::from(cwd)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    let project_root = project_root
        .canonicalize()
        .context("Failed to canonicalize project root")?;

    // Handle special config types
    if rustc || single_file_script {
        // Generate a single config file in the current directory
        let config_path = project_root.join(".cargo-runner.json");
        
        if config_path.exists() && !force {
            println!("❌ Config already exists at: {}", config_path.display());
            println!("   Use --force to overwrite");
            return Ok(());
        }
        
        let config = if rustc && single_file_script {
            println!("🦀 Generating combined rustc and single-file-script configuration");
            create_combined_config()
        } else if rustc {
            println!("🦀 Generating rustc configuration for standalone files");
            create_rustc_config()
        } else {
            println!("📜 Generating single-file-script configuration");
            create_single_file_script_config()
        };
        
        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
        
        println!("✅ Created config: {}", config_path.display());
        
        // Print example usage
        if rustc {
            println!("\n📌 Example rustc config usage:");
            println!("   Add your rustc-specific settings to the 'rustc' section");
            println!("   Configure test_framework, binary_framework, etc.");
        } else {
            println!("\n📌 Example single-file-script config usage:");
            println!("   Add cargo script settings to the 'single_file_script' section");
            println!("   Configure extra_args, extra_env, etc.");
        }
        
        return Ok(());
    }

    // Normal cargo project initialization
    println!(
        "🚀 Initializing cargo-runner in: {}",
        project_root.display()
    );

    // Set PROJECT_ROOT environment variable
    unsafe {
        env::set_var("PROJECT_ROOT", &project_root);
    }
    println!("✅ Set PROJECT_ROOT to: {}", project_root.display());

    // Find all Cargo.toml files recursively
    let mut cargo_tomls = Vec::new();
    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "Cargo.toml" {
            cargo_tomls.push(entry.path().to_path_buf());
        }
    }

    println!("📦 Found {} Cargo.toml files", cargo_tomls.len());

    // Generate .cargo-runner.json for each project
    let mut created = 0;
    let mut skipped = 0;

    // Create root config with linkedProjects
    let root_config_path = project_root.join(".cargo-runner.json");
    if !root_config_path.exists() || force {
        let root_config = create_root_config(&project_root, &cargo_tomls)?;
        fs::write(&root_config_path, root_config).with_context(|| {
            format!(
                "Failed to write root config to {}",
                root_config_path.display()
            )
        })?;
        info!("Created root config: {}", root_config_path.display());
        created += 1;
    } else {
        info!(
            "Skipping existing root config: {}",
            root_config_path.display()
        );
        skipped += 1;
    }

    // Generate configs for each sub-project
    for cargo_toml in &cargo_tomls {
        // Skip if this is the root Cargo.toml
        if cargo_toml == &project_root.join("Cargo.toml") {
            continue;
        }

        let project_dir = cargo_toml.parent().unwrap();
        let config_path = project_dir.join(".cargo-runner.json");

        // Check if config already exists
        if config_path.exists() && !force {
            info!("Skipping existing config: {}", config_path.display());
            skipped += 1;
            continue;
        }

        // Check if this is a workspace-only Cargo.toml
        let config = if is_workspace_only(cargo_toml)? {
            // Create workspace config (no package name)
            create_workspace_config()
        } else {
            // Read package name from Cargo.toml
            let package_name = get_package_name(cargo_toml)?;
            // Create package configuration
            create_default_config(&package_name)
        };

        // Write configuration file
        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        info!("Created config: {}", config_path.display());
        created += 1;
    }

    println!("\n✅ Initialization complete!");
    println!("   • Created {} config files", created);
    if skipped > 0 {
        println!(
            "   • Skipped {} existing configs (use --force to overwrite)",
            skipped
        );
    }

    // Print instructions for persisting PROJECT_ROOT
    println!("\n📌 To persist PROJECT_ROOT, add to your shell profile:");
    println!("   export PROJECT_ROOT=\"{}\"", project_root.display());

    Ok(())
}

fn unset_command(clean: bool) -> Result<()> {
    println!("🔧 Unsetting cargo-runner configuration...");

    // Get current PROJECT_ROOT if set
    let project_root = env::var("PROJECT_ROOT").ok();

    if let Some(root) = &project_root {
        println!("📍 Current PROJECT_ROOT: {}", root);

        if clean {
            println!("🧹 Cleaning .cargo-runner.json files...");

            use walkdir::WalkDir;
            let mut removed = 0;

            for entry in WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == ".cargo-runner.json" {
                    if let Err(e) = fs::remove_file(entry.path()) {
                        eprintln!("   ⚠️  Failed to remove {}: {}", entry.path().display(), e);
                    } else {
                        info!("Removed: {}", entry.path().display());
                        removed += 1;
                    }
                }
            }

            println!("   • Removed {} config files", removed);
        }
    } else {
        println!("ℹ️  PROJECT_ROOT is not currently set");
    }

    // Note: We can't actually unset the environment variable for the parent shell
    println!("\n📌 To unset PROJECT_ROOT, run in your shell:");
    println!("   unset PROJECT_ROOT");

    Ok(())
}

fn is_workspace_only(cargo_toml: &Path) -> Result<bool> {
    let contents = fs::read_to_string(cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;
    
    // Check if it has [workspace] but no [package]
    let has_workspace = contents.contains("[workspace]");
    let has_package = contents.contains("[package]");
    
    Ok(has_workspace && !has_package)
}

fn get_package_name(cargo_toml: &Path) -> Result<String> {
    let contents = fs::read_to_string(cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    // Simple TOML parsing for package name
    for line in contents.lines() {
        if let Some(name) = line.strip_prefix("name = ") {
            let name = name.trim().trim_matches('"');
            return Ok(name.to_string());
        }
    }

    // Fallback to directory name
    Ok(cargo_toml
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string())
}

fn create_rustc_config() -> String {
    // Use raw JSON string to preserve exact field order
    r#"{
  "rustc": {
    "benchmark_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_bench"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_bench",
        "args": [
          "--bench"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--crate-type",
          "bin",
          "--crate-name",
          "{file_name}",
          "-o",
          "{parent_dir}/{file_name}"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}",
        "extra_env": {
          "RUST_LOG": "debug"
        }
      }
    },
    "test_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_test"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_test",
        "extra_test_binary_args": [
          "--show-output"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    }
  },
  "overrides": []
}"#.to_string()
}

fn create_combined_config() -> String {
    // Combine both rustc and single-file-script configs
    r#"{
  "rustc": {
    "benchmark_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_bench"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_bench",
        "args": [
          "--bench"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--crate-type",
          "bin",
          "--crate-name",
          "{file_name}",
          "-o",
          "{parent_dir}/{file_name}"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}",
        "extra_env": {
          "RUST_LOG": "debug"
        }
      }
    },
    "test_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_test"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_test",
        "extra_test_binary_args": [
          "--show-output"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    }
  },
  "single_file_script": {
    "extra_args": [
      "-Zscript"
    ],
    "extra_env": {
      "CARGO_TARGET_DIR": "target/rust-analyzer"
    },
    "extra_test_binary_args": [
      "--show-output"
    ]
  },
  "overrides": []
}"#.to_string()
}

fn create_single_file_script_config() -> String {
    use serde_json::{Map, Value, json};
    
    // Create a config with single_file_script section
    let mut config = Map::new();
    
    // Create single file script config
    let sfs_config = json!({
        "extra_args": ["--edition=2024"],
        "extra_env": {
            "RUST_BACKTRACE": "1"
        },
        "extra_test_binary_args": ["--show-output"]
    });
    
    config.insert("single_file_script".to_string(), sfs_config);
    config.insert("overrides".to_string(), json!([]));
    
    // Pretty print the JSON
    serde_json::to_string_pretty(&config).unwrap()
}

fn create_workspace_config() -> String {
    use serde_json::{Map, Value, json};
    
    // Create a config with new nested structure for workspace
    let mut config = Map::new();
    
    // Create cargo config section without package name
    let mut cargo_config = Map::new();
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));
    
    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));
    
    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));
    
    // Pretty print the JSON
    serde_json::to_string_pretty(&config).unwrap()
}

fn create_default_config(package_name: &str) -> String {
    use serde_json::{Map, Value, json};

    // Create a config with new nested structure
    let mut config = Map::new();

    // Create cargo config section
    let mut cargo_config = Map::new();
    cargo_config.insert("package".to_string(), json!(package_name));
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));
    
    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));
    
    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));

    // Example test_frameworks configuration (commented out by default)
    // Uncomment and modify as needed:
    /*
    config.insert("test_frameworks".to_string(), json!({
        "command": "cargo",
        "subcommand": "nextest run",
        "args": ["-j10"],
        "extra_env": {
            "RUST_BACKTRACE": "full"
        }
    }));
    */

    let config_value = Value::Object(config);
    serde_json::to_string_pretty(&config_value).unwrap()
}

fn create_root_config(project_root: &Path, cargo_tomls: &[PathBuf]) -> Result<String> {
    use serde_json::{Map, Value, json};

    // Get the root package name if available
    let root_cargo_toml = project_root.join("Cargo.toml");
    let package_name = if root_cargo_toml.exists() {
        // Check if this is a workspace-only Cargo.toml
        if is_workspace_only(&root_cargo_toml)? {
            None  // Workspaces don't have package names
        } else {
            Some(get_package_name(&root_cargo_toml)?)
        }
    } else {
        None
    };

    // Convert all Cargo.toml paths to strings
    let linked_projects: Vec<String> = cargo_tomls
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    // Create root configuration with new nested structure
    let mut config = Map::new();

    // Create cargo config section
    let mut cargo_config = Map::new();
    
    // Only add package if we have one
    if let Some(pkg) = package_name {
        cargo_config.insert("package".to_string(), json!(pkg));
    }
    
    // Add linked projects to cargo config
    cargo_config.insert("linked_projects".to_string(), json!(linked_projects));
    
    // Add empty defaults for cargo config
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));
    
    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));
    
    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));

    // Example test_frameworks configuration with miri and nextest
    // Uncomment and modify as needed:
    /*
    config.insert("test_frameworks".to_string(), json!({
        "command": "cargo",
        "subcommand": "miri nextest run",
        "channel": "nightly",
        "args": ["-j10"],
        "extra_env": {
            "MIRIFLAGS": "-Zmiri-disable-isolation",
            "RUST_BACKTRACE": "full"
        }
    }));
    */

    let config_value = Value::Object(config);
    Ok(serde_json::to_string_pretty(&config_value).unwrap())
}

fn print_config_details(_runner: &cargo_runner_core::CargoRunner, filepath: &str) -> Result<()> {
    use cargo_runner_core::config::ConfigMerger;
    use std::path::Path;

    println!("\n📁 Configuration Details:");
    println!("   {}", "-".repeat(75));

    // Get the merged configs
    let path = Path::new(filepath);
    let mut merger = ConfigMerger::new();
    merger.load_configs_for_path(path)?;

    // Show which configs were loaded
    let config_info = merger.get_config_info();

    if let Some(root_path) = &config_info.root_config_path {
        println!("   🏯 Root config: {}", root_path.display());
    } else {
        println!("   🏯 Root config: None");
    }

    if let Some(workspace_path) = &config_info.workspace_config_path {
        println!("   📦 Workspace config: {}", workspace_path.display());
    } else {
        println!("   📦 Workspace config: None");
    }

    if let Some(package_path) = &config_info.package_config_path {
        println!("   📦 Package config: {}", package_path.display());
    } else {
        println!("   📦 Package config: None");
    }

    // Show the merged config summary
    let merged_config = merger.get_merged_config();
    println!("\n   🔀 Merged configuration:");

    // Show cargo configuration if present
    if let Some(cargo_config) = &merged_config.cargo {
        if let Some(command) = &cargo_config.command {
            println!("      • command: {}", command);
        }
        if let Some(subcommand) = &cargo_config.subcommand {
            println!("      • subcommand: {}", subcommand);
        }
        if let Some(channel) = &cargo_config.channel {
            println!("      • channel: {}", channel);
        }
        if let Some(features) = &cargo_config.features {
            match features {
                cargo_runner_core::config::Features::All(s) if s == "all" => {
                    println!("      • features: all");
                }
                cargo_runner_core::config::Features::Selected(selected) => {
                    println!("      • features: {:?}", selected);
                }
                _ => {}
            }
        }
        if let Some(extra_args) = &cargo_config.extra_args {
            if !extra_args.is_empty() {
                println!("      • extra_args: {:?}", extra_args);
            }
        }
        if let Some(extra_env) = &cargo_config.extra_env {
            if !extra_env.is_empty() {
                println!("      • extra_env: {} variables", extra_env.len());
            }
        }
        if let Some(linked_projects) = &cargo_config.linked_projects {
            println!(
                "      • linked_projects: {} projects",
                linked_projects.len()
            );
        }
    }

    // Show rustc configuration if present
    if let Some(rustc_config) = &merged_config.rustc {
        println!("      • rustc config:");
        if rustc_config.test_framework.is_some() {
            println!("         - test_framework: configured");
        }
        if rustc_config.binary_framework.is_some() {
            println!("         - binary_framework: configured");
        }
        if rustc_config.benchmark_framework.is_some() {
            println!("         - benchmark_framework: configured");
        }
    }

    // Show single file script configuration if present
    if let Some(sfs_config) = &merged_config.single_file_script {
        println!("      • single_file_script config:");
        if let Some(extra_args) = &sfs_config.extra_args {
            if !extra_args.is_empty() {
                println!("         - extra_args: {:?}", extra_args);
            }
        }
        if let Some(extra_env) = &sfs_config.extra_env {
            if !extra_env.is_empty() {
                println!("         - extra_env: {} variables", extra_env.len());
            }
        }
    }
    if !merged_config.overrides.is_empty() {
        println!(
            "      • overrides: {} configured",
            merged_config.overrides.len()
        );
    }

    println!();
    Ok(())
}

fn override_command(filepath_arg: &str, root: bool, override_args: Vec<String>) -> Result<()> {
    use serde_json::{Map, Value, json};
    
    // Parse filepath and line number
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    
    println!("🔧 Creating override configuration...");
    println!("   📍 File: {}", filepath);
    if let Some(line_num) = line {
        println!("   📍 Line: {}", line_num + 1); // Convert back to 1-based
    }
    
    // Create a runner to detect the runnable at the given location
    let mut runner = cargo_runner_core::CargoRunner::new()?;
    
    // Resolve the file path
    let resolved_path = runner.resolve_file_path(&filepath)?;
    
    // Get the runnable at the specified line
    let runnable = if let Some(line_num) = line {
        runner.get_best_runnable_at_line(&resolved_path, line_num as u32)?
    } else {
        // If no line specified, try to get any runnable from the file
        let all_runnables = runner.detect_all_runnables(&resolved_path)?;
        all_runnables.into_iter().next()
    };
    
    let runnable = runnable.ok_or_else(|| {
        anyhow::anyhow!("No runnable found at the specified location")
    })?;
    
    println!("   🎯 Found: {:?}", runnable.kind);
    
    // Detect file type based on the runnable
    let file_type = runner.detect_file_type(&resolved_path)?;
    println!("   📝 File type: {:?}", file_type);
    
    // Create function identity for the override
    let identity = cargo_runner_core::FunctionIdentity {
        package: runner.get_package_name_str(&resolved_path).ok(),
        module_path: if runnable.module_path.is_empty() {
            None
        } else {
            Some(runnable.module_path.clone())
        },
        file_path: Some(resolved_path.clone()),
        function_name: runnable.get_function_name(),
        file_type: Some(file_type),
    };
    
    println!("   🔑 Function identity:");
    if let Some(pkg) = &identity.package {
        println!("      - package: {}", pkg);
    }
    if let Some(module) = &identity.module_path {
        println!("      - module_path: {}", module);
    }
    if let Some(func) = &identity.function_name {
        println!("      - function_name: {}", func);
    }
    println!("      - file_type: {:?}", file_type);
    
    // Create the override configuration based on file type
    let mut override_config = Map::new();
    
    // Add matcher
    let mut matcher = Map::new();
    if let Some(pkg) = &identity.package {
        matcher.insert("package".to_string(), json!(pkg));
    }
    if let Some(module) = &identity.module_path {
        matcher.insert("module_path".to_string(), json!(module));
    }
    if let Some(func) = &identity.function_name {
        matcher.insert("function_name".to_string(), json!(func));
    }
    // Always add file_path for precise matching
    matcher.insert("file_path".to_string(), json!(resolved_path.to_str().unwrap()));
    
    override_config.insert("match".to_string(), Value::Object(matcher));
    
    // Parse override arguments
    let parsed_args = parse_override_args(&override_args);
    
    // Add configuration based on file type
    match file_type {
        cargo_runner_core::FileType::CargoProject => {
            let mut cargo_config = Map::new();
            if let Some(extra_args) = parsed_args.get("extra_args") {
                cargo_config.insert("extra_args".to_string(), json!(extra_args));
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                cargo_config.insert("extra_env".to_string(), json!(extra_env));
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                cargo_config.insert("extra_test_binary_args".to_string(), json!(extra_test_binary_args));
            }
            override_config.insert("cargo".to_string(), Value::Object(cargo_config));
        }
        cargo_runner_core::FileType::Standalone => {
            // For rustc
            let mut rustc_config = Map::new();
            
            // Determine which framework to configure based on runnable kind
            let framework_key = match &runnable.kind {
                cargo_runner_core::RunnableKind::Test { .. } |
                cargo_runner_core::RunnableKind::ModuleTests { .. } => "test_framework",
                cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark_framework",
                _ => "binary_framework",
            };
            
            let mut framework = Map::new();
            let mut build = Map::new();
            let mut exec = Map::new();
            
            // Add parsed arguments to appropriate sections
            if let Some(extra_args) = parsed_args.get("extra_args") {
                build.insert("extra_args".to_string(), json!(extra_args));
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                exec.insert("extra_env".to_string(), json!(extra_env));
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                exec.insert("extra_test_binary_args".to_string(), json!(extra_test_binary_args));
            }
            
            if !build.is_empty() {
                framework.insert("build".to_string(), Value::Object(build));
            }
            if !exec.is_empty() {
                framework.insert("exec".to_string(), Value::Object(exec));
            }
            
            rustc_config.insert(framework_key.to_string(), Value::Object(framework));
            override_config.insert("rustc".to_string(), Value::Object(rustc_config));
        }
        cargo_runner_core::FileType::SingleFileScript => {
            let mut script_config = Map::new();
            if let Some(extra_args) = parsed_args.get("extra_args") {
                script_config.insert("extra_args".to_string(), json!(extra_args));
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                script_config.insert("extra_env".to_string(), json!(extra_env));
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                script_config.insert("extra_test_binary_args".to_string(), json!(extra_test_binary_args));
            }
            override_config.insert("single_file_script".to_string(), Value::Object(script_config));
        }
    }
    
    // Load existing config
    let config_path = if root {
        // Use PROJECT_ROOT or current directory for root config
        let root_path = env::var("PROJECT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap());
        root_path.join(".cargo-runner.json")
    } else {
        // Find the closest project config
        runner.find_config_path(&resolved_path)?
            .ok_or_else(|| anyhow::anyhow!("No config file found. Run 'cargo runner init' first"))?
    };
    
    println!("   📄 Config file: {}", config_path.display());
    
    // Read existing config or create new one
    let mut config: Map<String, Value> = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        serde_json::from_str(&content)?
    } else {
        let mut new_config = Map::new();
        new_config.insert("overrides".to_string(), json!([]));
        new_config
    };
    
    // Add the override to the overrides array
    let overrides = config.get_mut("overrides")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow::anyhow!("Invalid config format: missing or invalid 'overrides' array"))?;
    
    overrides.push(Value::Object(override_config));
    
    // Write the updated config
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, json)?;
    
    println!("✅ Override added successfully!");
    println!("   • Arguments: {:?}", override_args);
    
    Ok(())
}

fn parse_override_args(args: &[String]) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::{Map, Value, json};
    
    let mut result = Map::new();
    let mut extra_args = Vec::new();
    let mut extra_env = Map::new();
    let mut extra_test_binary_args = Vec::new();
    
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        
        // Check for environment variables (KEY=VALUE format)
        if arg.contains('=') && !arg.starts_with('-') {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 {
                extra_env.insert(parts[0].to_string(), json!(parts[1]));
            }
        }
        // Check for test binary args
        else if arg == "--test-threads" || arg == "--nocapture" || arg == "--show-output" || arg == "--exact" {
            extra_test_binary_args.push(arg.clone());
            // Check if next arg is a value for --test-threads
            if arg == "--test-threads" && i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 1;
                extra_test_binary_args.push(args[i].clone());
            }
        }
        // Everything else goes to extra_args
        else {
            extra_args.push(arg.clone());
        }
        
        i += 1;
    }
    
    if !extra_args.is_empty() {
        result.insert("extra_args".to_string(), json!(extra_args));
    }
    if !extra_env.is_empty() {
        result.insert("extra_env".to_string(), Value::Object(extra_env));
    }
    if !extra_test_binary_args.is_empty() {
        result.insert("extra_test_binary_args".to_string(), json!(extra_test_binary_args));
    }
    
    result
}
