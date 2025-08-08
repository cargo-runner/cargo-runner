use cargo_runner::CargoRunner;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    
    // When invoked as `cargo x`, cargo passes "x" as args[1]
    // Remove it if present
    if args.len() > 1 && args[1] == "x" {
        args.remove(1);
    }
    
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    
    match args[1].as_str() {
        "show" => handle_show_command(&args),
        "help" | "--help" | "-h" => {
            print_usage(&args[0]);
            Ok(())
        }
        _ => {
            // Check if first argument looks like file:line format
            if args[1].contains(':') && !args[1].starts_with('-') {
                // Treat as "show file:line" command
                let mut show_args = vec![args[0].clone(), "show".to_string(), args[1].clone()];
                show_args.extend_from_slice(&args[2..]);
                handle_show_command(&show_args)
            } else {
                // Legacy mode: treat first arg as file path
                handle_legacy_mode(&args)
            }
        }
    }
}

fn handle_show_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        eprintln!("Error: 'show' command requires a file path");
        eprintln!("\nUsage: {} show <file-path>[:<line>] [--scope|--list]", args[0]);
        std::process::exit(1);
    }
    
    let file_spec = &args[2];
    
    // Check for flags
    let show_scope = args.iter().any(|arg| arg == "--scope");
    let show_list = args.iter().any(|arg| arg == "--list");
    
    // Parse file:line format
    let (file_path, line_number) = parse_file_spec(file_spec);
    
    let path = Path::new(&file_path);
    if !path.exists() {
        eprintln!("Error: File not found: {}", path.display());
        std::process::exit(1);
    }
    
    let mut runner = CargoRunner::new()?;
    
    if let Some(line) = line_number {
        // Show best runnable at specific line
        if show_scope {
            show_scope_at_line(&mut runner, path, line)
        } else if show_list {
            show_runnables_at_line(&mut runner, path, line)
        } else {
            show_runnable_at_line(&mut runner, path, line)
        }
    } else {
        // No line number provided
        if show_list {
            eprintln!("Error: --list requires a line number");
            eprintln!("\nUsage: {} <file>:<line> --list", args[0]);
            std::process::exit(1);
        } else if show_scope {
            show_all_scopes(&mut runner, path)
        } else {
            show_all_runnables(&mut runner, path)
        }
    }
}

fn handle_legacy_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = Path::new(&args[1]);
    let line_number = args.get(2).and_then(|s| s.parse::<u32>().ok());
    
    if !file_path.exists() {
        eprintln!("Error: File not found: {}", file_path.display());
        std::process::exit(1);
    }
    
    let mut runner = CargoRunner::new()?;
    
    if let Some(line) = line_number {
        show_runnable_at_line(&mut runner, file_path, line)
    } else {
        show_all_runnables(&mut runner, file_path)
    }
}

fn parse_file_spec(spec: &str) -> (String, Option<u32>) {
    // Try to find the last colon that could be a line separator
    if let Some(colon_pos) = spec.rfind(':') {
        let file_part = &spec[..colon_pos];
        let line_part = &spec[colon_pos + 1..];
        
        // Check if the part after colon is a valid line number
        if let Ok(line) = line_part.parse::<u32>() {
            // Make sure we're not splitting a Windows drive letter like C:\
            if colon_pos > 1 || !file_part.chars().nth(0).map_or(false, |c| c.is_alphabetic()) {
                // Verify the file part actually exists (this handles the case where :22 is part of filename)
                let path = Path::new(file_part);
                if path.exists() || !Path::new(spec).exists() {
                    // Either the file exists, or the full spec doesn't exist either
                    // (in which case we want to report the cleaner error)
                    return (file_part.to_string(), Some(line));
                }
            }
        }
    }
    
    (spec.to_string(), None)
}


fn show_runnable_at_line(
    runner: &mut CargoRunner,
    file_path: &Path,
    line: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(runnable) = runner.get_best_runnable_at_line(file_path, line)? {
        // Quick mode: just show the command
        if std::env::var("CARGO_RUNNER_QUICK").is_ok() {
            if let Some(command) = runner.build_command_for_runnable(&runnable)? {
                println!("{}", command.to_shell_command());
            }
        } else {
            // Detailed mode
            println!("🎯 Best runnable at line {}:", line);
            println!();
            print_runnable_details(runner, &runnable)?;
        }
    } else {
        // No runnable found, try fallback
        if let Some(command) = runner.get_fallback_command(file_path)? {
            if std::env::var("CARGO_RUNNER_QUICK").is_ok() {
                println!("{}", command.to_shell_command());
            } else {
                println!("⚠️  No specific runnable found at line {}", line);
                println!("📦 Using fallback command based on file location:");
                println!("🚀 Command: {}", command.to_shell_command());
                println!("\n💡 Try running without line number to see all available runnables.");
            }
        } else {
            if std::env::var("CARGO_RUNNER_QUICK").is_ok() {
                // In quick mode, output nothing for easy scripting
                std::process::exit(1);
            } else {
                eprintln!("❌ No runnable found at line {}", line);
                eprintln!("\n💡 Try running without line number to see all available runnables.");
                std::process::exit(1);
            }
        }
    }
    
    Ok(())
}

fn show_all_runnables(
    runner: &mut CargoRunner,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Scanning: {}", file_path.display());
    println!("{}", "=".repeat(80));
    
    let runnables = runner.detect_all_runnables(file_path)?;
    
    if runnables.is_empty() {
        println!("❌ No runnables found in this file.");
        
        // Check if there's a fallback command
        if let Some(fallback_cmd) = runner.get_fallback_command(file_path)? {
            println!("\n📦 Fallback command based on file location:");
            println!("   🚀 {}", fallback_cmd.to_shell_command());
        }
        
        println!("\n💡 Runnables include:");
        println!("   - Functions with #[test] attribute");
        println!("   - Functions with #[bench] attribute");
        println!("   - main() functions");
        println!("   - Doc tests in /// comments");
        println!("   - Test modules");
    } else {
        println!("✅ Found {} runnable(s):\n", runnables.len());
        
        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);
            
            // Use extended scope if available
            if let Some(ref ext_scope) = runnable.extended_scope {
                println!("   📏 Scope: lines {}-{}", 
                    ext_scope.scope.start.line + 1,
                    ext_scope.scope.end.line + 1
                );
                if ext_scope.doc_comment_lines > 0 {
                    println!("   📝 Doc comments: {} lines", ext_scope.doc_comment_lines);
                }
                if ext_scope.attribute_lines > 0 {
                    println!("   🏷️  Attributes: {} lines", ext_scope.attribute_lines);
                }
                if ext_scope.has_doc_tests {
                    println!("   🧪 Contains doc tests");
                }
            } else {
                println!("   📏 Scope: lines {}-{}", 
                    runnable.scope.start.line + 1,
                    runnable.scope.end.line + 1
                );
            }
            
            // Build and display command
            if let Some(command) = runner.build_command_for_runnable(runnable)? {
                println!("   🚀 Command: {}", command.to_shell_command());
            }
            
            // Show type
            print!("   📦 Type: ");
            print_runnable_type(&runnable.kind);
            
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

fn print_runnable_details(
    runner: &CargoRunner,
    runnable: &cargo_runner::Runnable,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 {}", runnable.label);
    
    // Use extended scope if available
    if let Some(ref ext_scope) = runnable.extended_scope {
        println!("📏 Scope: lines {}-{}", 
            ext_scope.scope.start.line + 1,
            ext_scope.scope.end.line + 1
        );
        if ext_scope.doc_comment_lines > 0 {
            println!("📝 Doc comments: {} lines", ext_scope.doc_comment_lines);
        }
        if ext_scope.attribute_lines > 0 {
            println!("🏷️  Attributes: {} lines", ext_scope.attribute_lines);
        }
        if ext_scope.has_doc_tests {
            println!("🧪 Contains doc tests");
        }
    } else {
        println!("📏 Scope: lines {}-{}", 
            runnable.scope.start.line + 1,
            runnable.scope.end.line + 1
        );
    }
    
    if let Some(command) = runner.build_command_for_runnable(runnable)? {
        println!("🚀 Command: {}", command.to_shell_command());
    }
    
    print!("🏷️  Type: ");
    print_runnable_type(&runnable.kind);
    
    if !runnable.module_path.is_empty() {
        println!("📁 Module path: {}", runnable.module_path);
    }
    
    Ok(())
}

fn print_runnable_type(kind: &cargo_runner::RunnableKind) {
    match kind {
        cargo_runner::RunnableKind::Test { test_name, is_async } => {
            print!("Test function '{}'", test_name);
            if *is_async {
                print!(" (async)");
            }
            println!();
        }
        cargo_runner::RunnableKind::DocTest { struct_or_module_name, method_name } => {
            print!("Doc test for '{}'", struct_or_module_name);
            if let Some(method) = method_name {
                print!("::{}", method);
            }
            println!();
        }
        cargo_runner::RunnableKind::Benchmark { bench_name } => {
            println!("Benchmark '{}'", bench_name);
        }
        cargo_runner::RunnableKind::Binary { bin_name } => {
            print!("Binary");
            if let Some(name) = bin_name {
                print!(" '{}'", name);
            }
            println!();
        }
        cargo_runner::RunnableKind::ModuleTests { module_name } => {
            println!("Test module '{}'", module_name);
        }
    }
}

fn show_scope_at_line(
    _runner: &mut CargoRunner,
    file_path: &Path,
    line: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Showing scopes at line {} in {}", line, file_path.display());
    println!("{}", "=".repeat(80));
    
    let source = std::fs::read_to_string(file_path)?;
    let mut parser = cargo_runner::parser::RustParser::new()?;
    let extended_scopes = parser.get_extended_scopes(&source, file_path)?;
    
    // Find scopes containing the line
    let mut containing_scopes: Vec<_> = extended_scopes
        .iter()
        .filter(|s| s.scope.contains_line(line))
        .collect();
    
    // Sort by scope size (smallest first - most specific)
    containing_scopes.sort_by_key(|s| s.scope.end.line - s.scope.start.line);
    
    if containing_scopes.is_empty() {
        println!("❌ No scopes found at line {}", line);
    } else {
        println!("✅ Found {} scope(s) containing line {}:\n", containing_scopes.len(), line);
        
        for (i, extended_scope) in containing_scopes.iter().enumerate() {
            let scope = &extended_scope.scope;
            println!("{}. {:?} {}", 
                i + 1,
                scope.kind,
                scope.name.as_deref().unwrap_or("<unnamed>")
            );
            println!("   📏 Range: lines {}-{}", 
                scope.start.line + 1,
                scope.end.line + 1
            );
            
            if extended_scope.doc_comment_lines > 0 {
                println!("   📝 Doc comments: {} lines", extended_scope.doc_comment_lines);
            }
            if extended_scope.attribute_lines > 0 {
                println!("   🏷️  Attributes: {} lines", extended_scope.attribute_lines);
            }
            if extended_scope.has_doc_tests {
                println!("   🧪 Contains doc tests");
            }
            
            println!("   🎯 Original start: line {}", extended_scope.original_start.line + 1);
            
            if i < containing_scopes.len() - 1 {
                println!();
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    Ok(())
}

fn show_all_scopes(
    _runner: &mut CargoRunner,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 All scopes in: {}", file_path.display());
    println!("{}", "=".repeat(80));
    
    let source = std::fs::read_to_string(file_path)?;
    let mut parser = cargo_runner::parser::RustParser::new()?;
    let extended_scopes = parser.get_extended_scopes(&source, file_path)?;
    
    if extended_scopes.is_empty() {
        println!("❌ No scopes found in this file.");
    } else {
        println!("✅ Found {} scope(s):\n", extended_scopes.len());
        
        for (i, extended_scope) in extended_scopes.iter().enumerate() {
            let scope = &extended_scope.scope;
            println!("{}. {:?} {}", 
                i + 1,
                scope.kind,
                scope.name.as_deref().unwrap_or("<unnamed>")
            );
            println!("   📏 Range: lines {}-{}", 
                scope.start.line + 1,
                scope.end.line + 1
            );
            
            if extended_scope.doc_comment_lines > 0 {
                println!("   📝 Doc comments: {} lines", extended_scope.doc_comment_lines);
            }
            if extended_scope.attribute_lines > 0 {
                println!("   🏷️  Attributes: {} lines", extended_scope.attribute_lines);
            }
            if extended_scope.has_doc_tests {
                println!("   🧪 Contains doc tests");
            }
            
            if i < extended_scopes.len() - 1 {
                println!();
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    Ok(())
}

fn show_runnables_at_line(
    runner: &mut CargoRunner,
    file_path: &Path,
    line: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 All runnables at line {} in {}", line, file_path.display());
    println!("{}", "=".repeat(80));
    
    let runnables = runner.detect_runnables_at_line(file_path, line)?;
    
    if runnables.is_empty() {
        println!("❌ No runnables found at line {}", line);
        println!("\n💡 Use --scope to see what scopes exist at this line.");
    } else {
        // Get extended scopes for better range information
        let source = std::fs::read_to_string(file_path)?;
        let mut parser = cargo_runner::parser::RustParser::new()?;
        let extended_scopes = parser.get_extended_scopes(&source, file_path)?;
        
        println!("✅ Found {} runnable(s) at line {}:\n", runnables.len(), line);
        
        for (i, runnable) in runnables.iter().enumerate() {
            // Find the corresponding extended scope
            let extended_scope = extended_scopes.iter()
                .find(|es| es.scope.name == runnable.scope.name 
                    && es.scope.kind == runnable.scope.kind
                    && es.scope.start.line == runnable.scope.start.line)
                .map(|es| es.clone())
                .unwrap_or_else(|| cargo_runner::ExtendedScope::from(runnable.scope.clone()));
            
            println!("{}. {}", i + 1, runnable.label);
            println!("   📏 Scope: lines {}-{}", 
                extended_scope.scope.start.line + 1,
                extended_scope.scope.end.line + 1
            );
            
            if extended_scope.doc_comment_lines > 0 {
                println!("   📝 Doc comments: {} lines", extended_scope.doc_comment_lines);
            }
            if extended_scope.attribute_lines > 0 {
                println!("   🏷️  Attributes: {} lines", extended_scope.attribute_lines);
            }
            if extended_scope.has_doc_tests {
                println!("   🧪 Contains doc tests");
            }
            
            if let Some(command) = runner.build_command_for_runnable(runnable)? {
                println!("   🚀 Command: {}", command.to_shell_command());
            }
            
            print!("   📦 Type: ");
            print_runnable_type(&runnable.kind);
            
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

fn print_usage(program_name: &str) {
    println!("cargo-x - Detect and run Rust code at specific locations");
    println!();
    println!("USAGE:");
    println!("    {} <file>:<line>                  Show runnable at specific line", program_name);
    println!("    {} <file>                         Show all runnables in file", program_name);
    println!("    {} show <file>[:<line>] [OPTIONS] Explicit show command", program_name);
    println!("    {} help                           Show this help message", program_name);
    println!();
    println!("OPTIONS:");
    println!("    --scope    Show scope information (ranges, doc comments, attributes)");
    println!("    --list     List all runnables at the given location");
    println!();
    println!("EXAMPLES:");
    println!("    {} src/lib.rs:42                  Get runnable at line 42", program_name);
    println!("    {} src/lib.rs                     List all runnables", program_name);
    println!("    {} src/lib.rs:42 --scope          Show scope info at line 42", program_name);
    println!("    {} src/lib.rs:42 --list           List all runnables at line 42", program_name);
    println!("    {} /absolute/path.rs:15           Works with absolute paths", program_name);
    println!();
    println!("ENVIRONMENT:");
    println!("    CARGO_RUNNER_QUICK=1              Output only the command (for scripting)");
    println!();
    println!("SCRIPTING EXAMPLE:");
    println!("    # Run test at cursor position");
    println!("    $(CARGO_RUNNER_QUICK=1 {} src/lib.rs:42)", program_name);
    println!();
    println!("DETECTABLE RUNNABLES:");
    println!("    - Test functions (#[test])");
    println!("    - Async tests (#[tokio::test])");
    println!("    - Benchmarks (#[bench])");
    println!("    - Binary/main functions");
    println!("    - Doc tests in /// comments");
    println!("    - Test modules");
}