use cargo_runner::{CargoRunner, ExtendedScope};
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    
    match args[1].as_str() {
        "help" | "--help" | "-h" => {
            print_usage(&args[0]);
            Ok(())
        }
        file_path => {
            let path = Path::new(file_path);
            if !path.exists() {
                eprintln!("Error: File not found: {}", path.display());
                std::process::exit(1);
            }
            
            show_all_runnables_extended(path)
        }
    }
}

fn show_all_runnables_extended(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing: {}", file_path.display());
    println!("{}", "=".repeat(80));
    
    let mut runner = CargoRunner::new()?;
    
    // First, show the file-level command
    show_file_level_command(&mut runner, file_path)?;
    
    // Then show all runnables with extended scopes
    let runnables = runner.detect_all_runnables(file_path)?;
    
    // Get extended scopes for better range information
    let source = std::fs::read_to_string(file_path)?;
    let mut parser = cargo_runner::parser::RustParser::new()?;
    let extended_scopes = parser.get_extended_scopes(&source, file_path)?;
    
    if runnables.is_empty() {
        println!("\n❌ No specific runnables found in this file.");
    } else {
        println!("\n✅ Found {} runnable(s):\n", runnables.len());
        
        for (i, runnable) in runnables.iter().enumerate() {
            // Use the runnable's extended scope if available, otherwise find it
            let extended_scope = if let Some(ref ext) = runnable.extended_scope {
                ext.clone()
            } else {
                extended_scopes.iter()
                    .find(|es| es.scope.name == runnable.scope.name 
                        && es.scope.kind == runnable.scope.kind
                        && es.scope.start.line == runnable.scope.start.line)
                    .map(|es| es.clone())
                    .unwrap_or_else(|| ExtendedScope::from(runnable.scope.clone()))
            };
            
            println!("{}. {}", i + 1, runnable.label);
            
            // Always use the extended scope for display
            let display_scope = &extended_scope.scope;
            
            println!("   📏 Scope: lines {}-{}", 
                display_scope.start.line + 1,
                display_scope.end.line + 1
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

fn show_file_level_command(runner: &mut CargoRunner, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("📄 File-level command:");
    
    // Get the file type by parsing the file
    let source = std::fs::read_to_string(file_path)?;
    let mut parser = cargo_runner::parser::RustParser::new()?;
    let extended_scopes = parser.get_extended_scopes(&source, file_path)?;
    
    // The first scope should be the file scope
    let (file_type_str, file_scope_lines) = if let Some(file_scope) = extended_scopes.first() {
        let line_info = format!("lines {}-{}", file_scope.scope.start.line + 1, file_scope.scope.end.line + 1);
        let type_str = match &file_scope.scope.kind {
            cargo_runner::ScopeKind::File(file_type) => {
                match file_type {
                    cargo_runner::FileScope::Lib => "Library (lib.rs)".to_string(),
                    cargo_runner::FileScope::Bin { name } => {
                        if let Some(n) = name {
                            format!("Binary '{}'", n)
                        } else {
                            "Binary".to_string()
                        }
                    },
                    cargo_runner::FileScope::Test { name } => {
                        if let Some(n) = name {
                            format!("Integration test '{}'", n)
                        } else {
                            "Integration test".to_string()
                        }
                    },
                    cargo_runner::FileScope::Bench { name } => {
                        if let Some(n) = name {
                            format!("Benchmark '{}'", n)
                        } else {
                            "Benchmark".to_string()
                        }
                    },
                    cargo_runner::FileScope::Example { name } => {
                        if let Some(n) = name {
                            format!("Example '{}'", n)
                        } else {
                            "Example".to_string()
                        }
                    },
                    cargo_runner::FileScope::Build => "Build script (build.rs)".to_string(),
                    cargo_runner::FileScope::Standalone { name } => {
                        if let Some(n) = name {
                            format!("Standalone Rust file '{}'", n)
                        } else {
                            "Standalone Rust file".to_string()
                        }
                    },
                    cargo_runner::FileScope::Unknown => "Unknown file type".to_string(),
                }
            },
            _ => "Unknown scope type".to_string()
        };
        (type_str, line_info)
    } else {
        ("Unknown file".to_string(), "".to_string())
    };
    
    if let Some(command) = runner.get_file_command(file_path)? {
        println!("   🚀 Command: {}", command.to_shell_command());
        println!("   📦 Type: {}", file_type_str);
        if !file_scope_lines.is_empty() {
            println!("   📏 Scope: {}", file_scope_lines);
        }
    } else {
        println!("   ❌ No file-level command available");
        println!("   📦 Type: {}", file_type_str);
        if !file_scope_lines.is_empty() {
            println!("   📏 Scope: {}", file_scope_lines);
        }
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

fn print_usage(program_name: &str) {
    println!("cargo-runnables - List all runnables in a Rust file with extended scope information");
    println!();
    println!("USAGE:");
    println!("    {} <file-path>    List all runnables in the file", program_name);
    println!("    {} help           Show this help message", program_name);
    println!();
    println!("FEATURES:");
    println!("    - Shows file-level command based on location patterns");
    println!("    - Lists all runnables with extended scope ranges");
    println!("    - Includes doc comments and attributes in scope");
    println!("    - Detects test functions, benchmarks, binaries, and doc tests");
    println!();
    println!("EXAMPLES:");
    println!("    {} src/lib.rs", program_name);
    println!("    {} src/bin/main.rs", program_name);
    println!("    {} tests/integration.rs", program_name);
}