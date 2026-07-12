//! `cargo runner watch [FILE]` — Unified file watcher for any project type.
//!
//! Watches `src/` (or the file's directory) for `.rs` changes and automatically
//! triggers a build, test, run, or a **resolved** cargo-runner command.
//!
//! ## Modes
//!
//! | Invocation | Behavior |
//! |------------|----------|
//! | `watch` | Project build |
//! | `watch --test` / `--run` | Project test / run |
//! | `watch src/lib.rs:12` | Resolve like `run` once, re-execute that command on change |
//! | `watch src/main.rs --run` | File-level command for that path, replayed on change |
//!
//! Uses `cargo watch` only for **project-level** cargo mode when installed.

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command as StdCommand;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::commands::run::{RunCargoFlags, resolve_command_for_selector};
use crate::config::bazel_workspace::{find_bazel_crates, find_module_bazel};
use crate::display::style;
use crate::utils::parse_filepath_with_line;

pub fn watch_command(
    filepath: Option<&str>,
    run_mode: bool,
    test_mode: bool,
    debounce_ms: u64,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Scoped / file replay: any filepath with a line, or a .rs file path
    if let Some(fp) = filepath {
        let (path_only, line) = parse_filepath_with_line(fp);
        let p = Path::new(&path_only);
        let looks_like_rust =
            p.extension().is_some_and(|e| e == "rs") || line.is_some() || fp.contains("::");
        if looks_like_rust {
            return watch_resolved_command(fp, &cwd, debounce_ms);
        }
    }

    // Determine watch directory from file path or cwd
    let watch_dir = if let Some(fp) = filepath {
        let p = Path::new(fp);
        if p.is_file() {
            p.parent().unwrap_or(&cwd).to_path_buf()
        } else {
            p.to_path_buf()
        }
    } else if cwd.join("src").is_dir() {
        cwd.join("src")
    } else {
        cwd.clone()
    };

    // Detect project type
    if let Some(bazel_root) = find_module_bazel(&cwd) {
        let target = resolve_bazel_target(&bazel_root, &cwd);
        let mode = mode_str(run_mode, test_mode);
        style::println_human(format!(
            "{} [Bazel] Watching: {}",
            style::icon("👀"),
            watch_dir.display()
        ));
        println!("   target: {target}  |  mode: bazel {mode}");
        run_notify_loop(&watch_dir, debounce_ms, move |_changed| {
            bazel_trigger(&bazel_root, &target, mode);
        })
    } else {
        let mode = mode_str(run_mode, test_mode);
        // cargo-watch only for project-level (no scoped selector)
        if cargo_watch_available() && filepath.is_none() {
            style::println_human(format!(
                "{} [Cargo] Watching via cargo-watch …",
                style::icon("👀")
            ));
            return run_cargo_watch(&cwd, mode);
        }
        style::println_human(format!(
            "{} [Cargo] Watching: {}",
            style::icon("👀"),
            watch_dir.display()
        ));
        println!("   mode: cargo {}", cargo_mode_cmd(mode));
        run_notify_loop(&watch_dir, debounce_ms, move |_changed| {
            cargo_trigger(&cwd, mode);
        })
    }
}

/// Resolve once like `run`, then re-execute that command on each .rs change.
fn watch_resolved_command(selector: &str, cwd: &Path, debounce_ms: u64) -> Result<()> {
    let command = resolve_command_for_selector(selector, RunCargoFlags::default(), &[])?;
    let shell = command.to_shell_command();
    let work_dir = command
        .working_dir
        .clone()
        .unwrap_or_else(|| cwd.to_path_buf());

    let (path_only, _) = parse_filepath_with_line(selector);
    let watch_dir = {
        let p = Path::new(&path_only);
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        };
        if abs.is_file() {
            abs.parent().unwrap_or(cwd).to_path_buf()
        } else if abs.is_dir() {
            abs
        } else {
            cwd.join("src")
        }
    };

    style::println_human(format!(
        "{} Replaying resolved command on change",
        style::icon("👀")
    ));
    println!("   watch: {}", watch_dir.display());
    println!("   cmd:   {shell}");
    if let Some(ref d) = command.working_dir {
        println!("   cwd:   {}", d.display());
    }

    let program = command.program.clone();
    let args = command.args.clone();
    let env = command.env.clone();
    let work_dir = work_dir.clone();

    run_notify_loop(&watch_dir, debounce_ms, move |_changed| {
        println!("─────────────────────────────────────────");
        let mut cmd = StdCommand::new(&program);
        cmd.args(&args).current_dir(&work_dir);
        for (k, v) in &env {
            if !k.starts_with('_') {
                cmd.env(k, v);
            }
        }
        match cmd.status() {
            Ok(s) if s.success() => {
                style::println_human(format!("{} replay succeeded", style::icon("✅")))
            }
            Ok(_) => style::println_human(format!(
                "{} replay failed — waiting for next change …",
                style::icon("❌")
            )),
            Err(e) => {
                style::println_human(format!("{} Failed to run command: {e}", style::icon("⚠️")))
            }
        }
    })
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn mode_str(run_mode: bool, test_mode: bool) -> &'static str {
    if run_mode {
        "run"
    } else if test_mode {
        "test"
    } else {
        "build"
    }
}

fn cargo_mode_cmd(mode: &str) -> &str {
    match mode {
        "run" => "run",
        "test" => "test",
        _ => "build",
    }
}

fn cargo_watch_available() -> bool {
    StdCommand::new("cargo")
        .args(["watch", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_cargo_watch(root: &Path, mode: &str) -> Result<()> {
    let x_arg = format!("-x{}", cargo_mode_cmd(mode));
    let status = StdCommand::new("cargo")
        .args(["watch", &x_arg])
        .current_dir(root)
        .status()
        .context("Failed to run `cargo watch`")?;
    if !status.success() {
        anyhow::bail!("`cargo watch` exited with error");
    }
    Ok(())
}

fn resolve_bazel_target(bazel_root: &Path, cwd: &Path) -> String {
    if let Ok(all) = find_bazel_crates(bazel_root)
        && let Some(krate) = all
            .into_iter()
            .find(|c| cwd.starts_with(&c.dir) || c.dir == cwd)
    {
        let rel = krate.dir.strip_prefix(bazel_root).unwrap_or(Path::new(""));
        let s = rel.to_string_lossy();
        return if s.is_empty() {
            "//...".to_string()
        } else {
            format!("//{s}:...")
        };
    }
    "//...".to_string()
}

fn bazel_trigger(root: &Path, target: &str, mode: &str) {
    println!("─────────────────────────────────────────");
    let mut cmd = StdCommand::new("bazel");
    cmd.arg(mode).arg(target).current_dir(root);
    if mode == "test" {
        cmd.arg("--test_output=errors");
    }
    match cmd.status() {
        Ok(s) if s.success() => {
            style::println_human(format!("{} bazel {mode} succeeded", style::icon("✅")))
        }
        Ok(_) => style::println_human(format!(
            "{} bazel {mode} failed — waiting for next change …",
            style::icon("❌")
        )),
        Err(e) => style::println_human(format!("{} Failed to run bazel: {e}", style::icon("⚠️"))),
    }
}

fn cargo_trigger(root: &Path, mode: &str) {
    println!("─────────────────────────────────────────");
    let cmd_name = cargo_mode_cmd(mode);
    match StdCommand::new("cargo")
        .arg(cmd_name)
        .current_dir(root)
        .status()
    {
        Ok(s) if s.success() => {
            style::println_human(format!("{} cargo {cmd_name} succeeded", style::icon("✅")))
        }
        Ok(_) => style::println_human(format!(
            "{} cargo {cmd_name} failed — waiting for next change …",
            style::icon("❌")
        )),
        Err(e) => style::println_human(format!("{} Failed to run cargo: {e}", style::icon("⚠️"))),
    }
}

fn run_notify_loop<F>(watch_dir: &Path, debounce_ms: u64, trigger: F) -> Result<()>
where
    F: Fn(&str) + Send + 'static,
{
    println!("Press Ctrl-C to stop.\n");

    // Trigger once immediately
    trigger("(initial)");

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher =
        notify::recommended_watcher(tx).context("Failed to create filesystem watcher")?;
    watcher
        .watch(watch_dir, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", watch_dir.display()))?;

    let debounce = Duration::from_millis(debounce_ms);
    let mut last_trigger = Instant::now() - debounce;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !is_rust_change(&event) {
                    continue;
                }
                if last_trigger.elapsed() < debounce {
                    continue;
                }
                last_trigger = Instant::now();

                let files: Vec<_> = event
                    .paths
                    .iter()
                    .map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    })
                    .collect();
                style::println_human(format!(
                    "{} Changed: {}",
                    style::icon("📝"),
                    files.join(", ")
                ));
                trigger(&files.join(", "));
            }
            Ok(Err(e)) => eprintln!("Watcher error: {e}"),
            Err(_) => break,
        }
    }
    Ok(())
}

fn is_rust_change(event: &Event) -> bool {
    matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
        && event
            .paths
            .iter()
            .any(|p| p.extension().is_some_and(|e| e == "rs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_str_run() {
        assert_eq!(mode_str(true, false), "run");
    }

    #[test]
    fn mode_str_test() {
        assert_eq!(mode_str(false, true), "test");
    }

    #[test]
    fn mode_str_default_build() {
        assert_eq!(mode_str(false, false), "build");
    }

    #[test]
    fn mode_str_run_takes_priority() {
        assert_eq!(mode_str(true, true), "run");
    }

    #[test]
    fn cargo_mode_cmd_run() {
        assert_eq!(cargo_mode_cmd("run"), "run");
    }

    #[test]
    fn cargo_mode_cmd_test() {
        assert_eq!(cargo_mode_cmd("test"), "test");
    }

    #[test]
    fn cargo_mode_cmd_build_default() {
        assert_eq!(cargo_mode_cmd("build"), "build");
    }

    #[test]
    fn cargo_mode_cmd_unknown_defaults_to_build() {
        assert_eq!(cargo_mode_cmd("anything"), "build");
    }
}
