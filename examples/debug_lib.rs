use cargo_runner_core::runners::UnifiedRunner;
use std::path::Path;

fn main() {
    // Create runner
    let mut runner = UnifiedRunner::new();
    
    // Test lib.rs
    let lib_path = Path::new("crates/core/src/lib.rs");
    
    println!("=== Testing lib.rs ===");
    
    // Get runnables
    match runner.detect_runnables(lib_path) {
        Ok(runnables) => {
            println!("Found {} runnables:", runnables.len());
            for (i, r) in runnables.iter().enumerate() {
                println!("  [{}] {:?} - {}", i, r.kind, r.label);
                println!("      scope.name: {:?}", r.scope.name);
                println!("      module_path: {}", r.module_path);
            }
        }
        Err(e) => println!("Error detecting runnables: {}", e),
    }
    
    // Get file command
    println!("\n=== File command ===");
    match runner.get_file_command(lib_path) {
        Ok(Some(cmd)) => {
            println!("Command: {}", cmd.to_shell_command());
            println!("Args: {:?}", cmd.args);
        }
        Ok(None) => println!("No file command"),
        Err(e) => println!("Error: {}", e),
    }
}