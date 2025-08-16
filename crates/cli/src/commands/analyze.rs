use anyhow::Result;
use std::path::Path;
use tracing::debug;

use crate::display::command_breakdown::print_command_breakdown;
use crate::display::formatter::{determine_file_type, print_runnable_type};
use crate::utils::parser::parse_filepath_with_line;

pub fn analyze_command(filepath_arg: &str, verbose: bool, show_config: bool) -> Result<()> {
    debug!("Analyzing file: {}", filepath_arg);

    // Parse filepath and line number first
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    
    // Check if file exists - resolve to absolute path
    let filepath_path = Path::new(&filepath);
    let absolute_path = if filepath_path.is_absolute() {
        filepath_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(filepath_path)
    };
    
    if !absolute_path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", absolute_path.display()));
    }

    let mut runner = cargo_runner_core::UnifiedRunner::with_path(&absolute_path)?;

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

pub fn print_formatted_analysis(
    runner: &mut cargo_runner_core::UnifiedRunner,
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
        print_v2_config_details(runner, filepath)?;
    }

    let path = Path::new(filepath);

    // Always show file-level command as it represents the entire file scope
    // Get file-level command
    match runner.get_file_command(path) {
        Ok(Some(cmd)) => {
            println!("\n📄 File-level command:");
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
        Ok(None) => {
            println!("\n📄 File-level command: None");
        }
        Err(e) => {
            println!("\n📄 File-level command: Error - {}", e);
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
            println!(
                "   📏 Scope: lines {}-{}",
                runnable.scope.start.line + 1,
                runnable.scope.end.line + 1
            );

            // Show module path if applicable
            if !runnable.module_path.is_empty() {
                println!("   📍 Module path: {}", runnable.module_path);
            }

            // Show extended scope info if present
            if let Some(ref extended) = runnable.extended_scope {
                if extended.attribute_lines > 0 {
                    println!("   🏷️  Attributes: {} lines", extended.attribute_lines);
                }
                if extended.has_doc_tests {
                    println!("   🧪 Contains doc tests");
                }
            }

            // Build and show command
            if let Ok(command) = runner.build_command(runnable) {
                print_command_breakdown(&command);
                // Store the final command
                final_command = Some(command.to_shell_command());
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

    // Show the final command to run
    if let Some(cmd) = final_command {
        println!("\n🎯 Command to run:");
        println!("   {}", cmd);
    }

    println!("\n{}", "=".repeat(80));

    Ok(())
}

fn print_v2_config_details(runner: &cargo_runner_core::UnifiedRunner, _filepath: &str) -> Result<()> {

    println!("\n📁 Configuration Details:");
    println!("   {}", "-".repeat(75));

    // Get v2 config if available
    if let Some(v2_config) = Some(runner.v2_config()) {
        println!("   🆕 V2 Config loaded");
        
        // Show workspace configuration
        if let Some(workspace_layer) = v2_config.layers().iter().find(|l| matches!(l.scope, cargo_runner_core::config::v2::Scope::Workspace)) {
            println!("\n   📦 Workspace configuration:");
            
            // Build system
            if let Some(build_system) = &workspace_layer.config.build_system {
                println!("      • build_system: {:?}", build_system);
            }
            
            // Frameworks
            let frameworks = &workspace_layer.config.frameworks;
            if frameworks.test.is_some() || frameworks.binary.is_some() || 
               frameworks.benchmark.is_some() || frameworks.doctest.is_some() || 
               frameworks.build.is_some() {
                println!("      • frameworks:");
                if let Some(test) = &frameworks.test {
                    println!("        - test: {}", test);
                }
                if let Some(binary) = &frameworks.binary {
                    println!("        - binary: {}", binary);
                }
                if let Some(benchmark) = &frameworks.benchmark {
                    println!("        - benchmark: {}", benchmark);
                }
                if let Some(doctest) = &frameworks.doctest {
                    println!("        - doctest: {}", doctest);
                }
                if let Some(build) = &frameworks.build {
                    println!("        - build: {}", build);
                }
            }
            
            // Arguments
            let args = &workspace_layer.config.args;
            if args.all.is_some() || args.test.is_some() || args.binary.is_some() || 
               args.benchmark.is_some() || args.build.is_some() || args.test_binary.is_some() {
                println!("      • args:");
                if let Some(all) = &args.all {
                    println!("        - all: {:?}", all);
                }
                if let Some(test) = &args.test {
                    println!("        - test: {:?}", test);
                }
                if let Some(binary) = &args.binary {
                    println!("        - binary: {:?}", binary);
                }
                if let Some(benchmark) = &args.benchmark {
                    println!("        - benchmark: {:?}", benchmark);
                }
                if let Some(build) = &args.build {
                    println!("        - build: {:?}", build);
                }
                if let Some(test_binary) = &args.test_binary {
                    println!("        - test_binary: {:?}", test_binary);
                }
            }
            
            // Environment
            if !workspace_layer.config.env.vars.is_empty() {
                println!("      • env: {} variables", workspace_layer.config.env.vars.len());
                for (key, value) in &workspace_layer.config.env.vars {
                    println!("        - {}: {}", key, value);
                }
            }
        }
        
        // Show other layers if present
        let other_layers: Vec<_> = v2_config.layers().iter()
            .filter(|l| !matches!(l.scope, cargo_runner_core::config::v2::Scope::Workspace))
            .collect();
            
        if !other_layers.is_empty() {
            println!("\n   📑 Additional config layers: {}", other_layers.len());
            for layer in other_layers {
                println!("      • {:?}", layer.scope);
            }
        }
    } else {
        println!("   ❌ No V2 configuration found");
        println!("   💡 Create a .cargo-runner-v2.json file to configure the runner");
    }

    println!();
    Ok(())
}