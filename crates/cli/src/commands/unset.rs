use anyhow::Result;
use std::{env, fs, path::PathBuf};
use tracing::info;
use walkdir::WalkDir;

pub fn unset_command(clean: bool) -> Result<()> {
    println!("🔧 Unsetting cargo-runner configuration...");

    // Prefer PROJECT_ROOT; fall back to cwd so `unset --clean` works without env.
    let project_root = env::var("PROJECT_ROOT")
        .ok()
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok());

    if let Some(root) = &project_root {
        if env::var("PROJECT_ROOT").is_ok() {
            println!("📍 Current PROJECT_ROOT: {}", root.display());
        } else {
            println!("📍 Using current directory: {}", root.display());
        }

        if clean {
            println!("🧹 Cleaning .cargo-runner.json / .cargo-runner.env files…");

            let mut removed = 0;
            for entry in WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let name = entry.file_name();
                if name == ".cargo-runner.json" || name == ".cargo-runner.env" {
                    if let Err(e) = fs::remove_file(entry.path()) {
                        eprintln!("   ⚠️  Failed to remove {}: {e}", entry.path().display());
                    } else {
                        info!("Removed: {}", entry.path().display());
                        println!("   • removed {}", entry.path().display());
                        removed += 1;
                    }
                }
            }

            println!("   • Removed {removed} file(s)");
        }
    } else {
        println!("ℹ️  Could not determine project root");
    }

    // Note: We can't actually unset the environment variable for the parent shell
    if env::var("PROJECT_ROOT").is_ok() {
        println!("\n📌 To unset PROJECT_ROOT, run in your shell:");
        println!("   unset PROJECT_ROOT");
    }

    Ok(())
}
