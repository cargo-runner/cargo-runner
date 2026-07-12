//! `runnables` / analyze command orchestration.

use anyhow::Result;
use std::path::Path;
use tracing::debug;

use crate::commands::workspace::{
    find_files_for_module_path, workspace_rs_files, workspace_scan_roots,
};
use crate::config::bazel_workspace::find_cargo_workspace_root;
use crate::utils::parser::parse_filepath_with_line;

mod filters;
mod print_human;
mod print_json;

pub use filters::RunnableFilters;
pub use print_human::print_formatted_analysis;

use filters::describe_runnable_kind;
use print_json::emit_runnables_json;

pub fn runnables_command(
    filepath_arg: Option<&str>,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
    json: bool,
    with_commands: bool,
) -> Result<()> {
    if let Some(filepath_arg) = filepath_arg {
        return analyze_file_command(
            filepath_arg,
            filters,
            verbose,
            show_config,
            json,
            with_commands,
        );
    }

    analyze_workspace_command(filters, verbose, show_config, json, with_commands)
}

pub fn analyze_command(filepath_arg: &str, verbose: bool, show_config: bool) -> Result<()> {
    runnables_command(
        Some(filepath_arg),
        RunnableFilters::default(),
        verbose,
        show_config,
        false,
        false,
    )
}
fn analyze_file_command(
    filepath_arg: &str,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
    json: bool,
    with_commands: bool,
) -> Result<()> {
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
        if filepath.contains("::") {
            return analyze_module_path_command(
                &filepath,
                filters,
                verbose,
                show_config,
                json,
                with_commands,
            );
        }
        return Err(anyhow::anyhow!(
            "File not found: {}",
            absolute_path.display()
        ));
    }

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    if json || verbose {
        let mut runnables = if let Some(line_num) = line {
            runner.detect_runnables_at_line(&absolute_path, line_num as u32)?
        } else {
            runner.detect_all_runnables(&absolute_path)?
        };
        runnables.retain(|r| filters.matches(r));
        emit_runnables_json(&mut runner, runnables, json || with_commands, with_commands)?;
    } else {
        // Show formatted output
        print_formatted_analysis(&mut runner, &filepath, line, filters, show_config)?;
    }

    Ok(())
}
fn analyze_module_path_command(
    module_path: &str,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
    json: bool,
    with_commands: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut runner = cargo_runner_core::UnifiedRunner::new()?;
    let matches = find_files_for_module_path(&runner, module_path, &cwd)?;

    match matches.len() {
        0 => Err(anyhow::anyhow!(
            "No file found for module path: {module_path}"
        )),
        1 => {
            let path = matches.into_iter().next().expect("Checked len == 1");
            if json || verbose {
                let mut runnables = runner.detect_all_runnables(&path)?;
                runnables.retain(|r| filters.matches(r));
                emit_runnables_json(&mut runner, runnables, json || with_commands, with_commands)?;
            } else {
                print_formatted_analysis(
                    &mut runner,
                    path.to_str().unwrap_or_default(),
                    None,
                    filters,
                    show_config,
                )?;
            }
            Ok(())
        }
        _ => {
            let paths = matches
                .into_iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow::anyhow!(
                "Module path is ambiguous: {module_path}. Matches: {paths}"
            ))
        }
    }
}

fn analyze_workspace_command(
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
    json: bool,
    with_commands: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let workspace_root = find_cargo_workspace_root(&cwd).unwrap_or(cwd.clone());
    let scan_roots = workspace_scan_roots(&workspace_root)?;

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;
    let mut files = workspace_rs_files(&scan_roots);
    files.sort();

    if json {
        let mut all = Vec::new();
        for path in files {
            let Ok(mut runnables) = runner.detect_runnables(&path) else {
                continue;
            };
            runnables.retain(|r| filters.matches(r));
            all.extend(runnables);
        }
        return emit_runnables_json(&mut runner, all, true, with_commands);
    }

    println!("🔍 Scanning workspace: {}", workspace_root.display());
    println!("{}", "=".repeat(80));

    if files.is_empty() {
        println!("No Rust files found under {}", workspace_root.display());
        return Ok(());
    }

    let mut found_any = false;
    for path in files {
        let mut runnables = match runner.detect_runnables(&path) {
            Ok(runnables) if !runnables.is_empty() => runnables,
            _ => continue,
        };

        runnables.retain(|r| filters.matches(r));
        if runnables.is_empty() {
            continue;
        }

        found_any = true;
        println!();
        println!("📄 {}", path.display());
        println!("✅ Found {} runnable(s):\n", runnables.len());

        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);
            if verbose {
                println!(
                    "   📏 Scope: lines {}-{}",
                    runnable.scope.start.line + 1,
                    runnable.scope.end.line + 1
                );
            }
            if !runnable.module_path.is_empty() {
                println!("   📍 Module path: {}", runnable.module_path);
            }
            if show_config {
                println!("   📦 Type: {}", describe_runnable_kind(&runnable.kind));
            }
            if i < runnables.len() - 1 {
                println!();
            }
        }

        if verbose || show_config {
            println!();
        }
    }

    if !found_any {
        if filters.active() {
            println!(
                "No runnable items matched the filters in {}",
                workspace_root.display()
            );
        } else {
            println!("No runnable items found in {}", workspace_root.display());
        }
    }

    Ok(())
}
