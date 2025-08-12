use cargo_runner_core::UnifiedRunner;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new UnifiedRunner instance
    let mut runner = UnifiedRunner::new()?;

    // Example: Detect runnables at a specific line in a file
    let file_path = Path::new("src/lib.rs");
    let line = 10;

    if file_path.exists() {
        // Get the best runnable at the specified line
        if let Some(runnable) = runner.get_best_runnable_at_line(file_path, line)? {
            println!("Found runnable: {}", runnable.label);

            // Build the cargo command
            if let Some(command) = runner.build_command_for_runnable(&runnable)? {
                println!("Command: {}", command.to_shell_command());
            }
        } else {
            println!("No runnable found at line {}", line);
        }

        // Detect all runnables in the file
        println!("\nAll runnables in {}:", file_path.display());
        let all_runnables = runner.detect_all_runnables(file_path)?;
        for runnable in all_runnables {
            println!(
                "  - {} (lines {}-{})",
                runnable.label, runnable.scope.start.line, runnable.scope.end.line
            );
        }
    } else {
        println!("File not found: {}", file_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_example() {
        assert_eq!(2 + 2, 4);
    }
}
