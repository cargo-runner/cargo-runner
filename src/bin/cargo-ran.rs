use cargo_runner::{CargoRunner, Config};
use std::env;
use std::path::Path;
use tracing::debug;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: cargo-ran [--show] <file.rs[:line_number]>");
        eprintln!("Examples:");
        eprintln!("  cargo-ran src/main.rs");
        eprintln!("  cargo-ran src/main.rs:42");
        eprintln!("  cargo-ran --show src/main.rs:42");
        eprintln!("  cargo-ran tests/integration_test.rs:15");
        std::process::exit(1);
    }
    
    // Check for --show flag
    let (show_only, input) = if args[1] == "--show" {
        if args.len() < 3 {
            eprintln!("Error: Missing file path after --show");
            std::process::exit(1);
        }
        (true, &args[2])
    } else {
        (false, &args[1])
    };
    
    // Parse file path and optional line number
    let (file_path, line_number) = if let Some(colon_pos) = input.rfind(':') {
        let path_part = &input[..colon_pos];
        let line_part = &input[colon_pos + 1..];
        
        // Check if the part after colon is a valid number
        if let Ok(line) = line_part.parse::<u32>() {
            (path_part, Some(line))
        } else {
            // Not a valid line number, treat the whole thing as a path
            (input.as_str(), None)
        }
    } else {
        (input.as_str(), None)
    };
    
    let path = Path::new(file_path);
    
    // Check if file exists
    if !path.exists() {
        eprintln!("Error: File not found: {}", file_path);
        std::process::exit(1);
    }
    
    // Create runner
    let mut runner = CargoRunner::new()?;
    
    // Load config if available
    if let Some(config_path) = Config::find_config_file(path) {
        if let Ok(config) = Config::load_from_file(&config_path) {
            runner = CargoRunner::with_config(config)?;
        }
    }
    
    // Build command
    let command = if let Some(line) = line_number {
        println!("🎯 Finding runnable at {}:{}", file_path, line);
        
        // Convert to 0-based line number for internal use
        let line_0based = line.saturating_sub(1);
        debug!("User provided line: {}, converted to 0-based: {}", line, line_0based);
        
        // Always use build_command which has the fallback logic
        let command = runner.build_command(path, line_0based)?;
        
        if show_only {
            // When showing, also display what runnables were found
            let runnables = runner.detect_runnables_at_line(path, line_0based)?;
            debug!("Found {} runnables at line {}", runnables.len(), line);
            
            if runnables.is_empty() {
                println!("❌ No specific runnables found at line {}", line);
                
                // Let's also check what runnables exist in the file
                let all_runnables = runner.detect_all_runnables(path)?;
                if !all_runnables.is_empty() {
                    println!("\n📋 Available runnables in this file:");
                    for runnable in &all_runnables {
                        println!("  - {} (lines {}-{})", 
                            runnable.label,
                            runnable.scope.start.line + 1,
                            runnable.scope.end.line + 1);
                    }
                }
                
                println!("\n📄 Using file-level command");
            } else {
                // Show all runnables at this line
                println!("📊 Found {} runnable(s) at line {}:", runnables.len(), line);
                for (i, runnable) in runnables.iter().enumerate() {
                    let range_size = runnable.scope.end.line - runnable.scope.start.line;
                    println!("  {}. {} (lines {}-{}, size: {})", 
                        i + 1,
                        runnable.label,
                        runnable.scope.start.line + 1,
                        runnable.scope.end.line + 1,
                        range_size);
                }
            }
        }
        
        command
    } else {
        println!("📄 Getting file-level command for {}", file_path);
        runner.get_file_command(path)?
    };
    
    // Execute or show command
    if let Some(cmd) = command {
        if show_only {
            println!("🚀 Command: {}", cmd.to_shell_command());
        } else {
            println!("🚀 Executing: {}", cmd.to_shell_command());
            println!();
            
            // Execute the command
            let status = cmd.execute()?;
            
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
    } else {
        eprintln!("❌ No runnable found at specified location");
        std::process::exit(1);
    }
    
    Ok(())
}