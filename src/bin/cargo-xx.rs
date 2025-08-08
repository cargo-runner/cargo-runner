use cargo_runner::CargoRunner;
use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().collect();
    
    // When invoked as `cargo xx`, cargo passes "xx" as args[1]
    // Remove it if present
    if args.len() > 1 && args[1] == "xx" {
        args.remove(1);
    }
    
    if args.len() < 2 {
        eprintln!("Usage: {} <rust-file-path> [line-number]", args[0]);
        eprintln!("\nExamples:");
        eprintln!("  {} src/lib.rs              # Show all runnables in file", args[0]);
        eprintln!("  {} src/lib.rs 42           # Show runnable at line 42", args[0]);
        eprintln!("  {} src/tests/mod.rs        # Show test runnables", args[0]);
        std::process::exit(1);
    }
    
    let file_path = Path::new(&args[1]);
    let line_number = args.get(2).and_then(|s| s.parse::<u32>().ok());
    
    if !file_path.exists() {
        eprintln!("Error: File not found: {}", file_path.display());
        std::process::exit(1);
    }
    
    // Create runner
    let mut runner = CargoRunner::new()?;
    
    println!("🔍 Scanning: {}", file_path.display());
    println!("{}", "=".repeat(80));
    
    if let Some(line) = line_number {
        // Show runnable at specific line
        println!("📍 Looking for runnable at line {}...\n", line);
        
        if let Some(runnable) = runner.get_best_runnable_at_line(file_path, line)? {
            print_runnable(&runner, &runnable)?;
        } else {
            println!("❌ No runnable found at line {}", line);
            println!("\n💡 Tip: Try running without line number to see all available runnables.");
        }
    } else {
        // Show all runnables
        let runnables = runner.detect_all_runnables(file_path)?;
        
        if runnables.is_empty() {
            println!("❌ No runnables found in this file.");
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
                
                // Show runnable type details
                match &runnable.kind {
                    cargo_runner::RunnableKind::Test { test_name, is_async } => {
                        println!("   📦 Type: Test function '{}'", test_name);
                        if *is_async {
                            println!("   ⚡ Async: Yes");
                        }
                    }
                    cargo_runner::RunnableKind::DocTest { struct_or_module_name, method_name } => {
                        print!("   📦 Type: Doc test for '{}'", struct_or_module_name);
                        if let Some(method) = method_name {
                            print!("::{}", method);
                        }
                        println!();
                    }
                    cargo_runner::RunnableKind::Benchmark { bench_name } => {
                        println!("   📦 Type: Benchmark '{}'", bench_name);
                    }
                    cargo_runner::RunnableKind::Binary { bin_name } => {
                        print!("   📦 Type: Binary");
                        if let Some(name) = bin_name {
                            print!(" '{}'", name);
                        }
                        println!();
                    }
                    cargo_runner::RunnableKind::ModuleTests { module_name } => {
                        println!("   📦 Type: Test module '{}'", module_name);
                    }
                }
                
                if !runnable.module_path.is_empty() {
                    println!("   📁 Module path: {}", runnable.module_path);
                }
                
                if i < runnables.len() - 1 {
                    println!();
                }
            }
        }
    }
    
    println!("\n{}", "=".repeat(80));
    
    Ok(())
}

fn print_runnable(runner: &CargoRunner, runnable: &cargo_runner::Runnable) -> Result<(), Box<dyn std::error::Error>> {
    println!("✅ Found: {}", runnable.label);
    println!("📏 Scope: lines {}-{}", 
        runnable.scope.start.line + 1,
        runnable.scope.end.line + 1
    );
    
    if let Some(command) = runner.build_command_for_runnable(runnable)? {
        println!("🚀 Command: {}", command.to_shell_command());
    }
    
    match &runnable.kind {
        cargo_runner::RunnableKind::Test { test_name, is_async } => {
            println!("📦 Type: Test function '{}'", test_name);
            if *is_async {
                println!("⚡ Async: Yes");
            }
        }
        cargo_runner::RunnableKind::DocTest { struct_or_module_name, method_name } => {
            print!("📦 Type: Doc test for '{}'", struct_or_module_name);
            if let Some(method) = method_name {
                print!("::{}", method);
            }
            println!();
        }
        cargo_runner::RunnableKind::Benchmark { bench_name } => {
            println!("📦 Type: Benchmark '{}'", bench_name);
        }
        cargo_runner::RunnableKind::Binary { bin_name } => {
            print!("📦 Type: Binary");
            if let Some(name) = bin_name {
                print!(" '{}'", name);
            }
            println!();
        }
        cargo_runner::RunnableKind::ModuleTests { module_name } => {
            println!("📦 Type: Test module '{}'", module_name);
        }
    }
    
    if !runnable.module_path.is_empty() {
        println!("📁 Module path: {}", runnable.module_path);
    }
    
    Ok(())
}