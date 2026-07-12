//! `cargo runner agent-init` — install cargo-runner agent instructions into
//! AGENTS.md / CLAUDE.md / Cursor rules / etc.
//!
//! Symlinks are followed; each real file is updated at most once.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const BEGIN: &str = "<!-- BEGIN cargo-runner agent instructions -->";
const END: &str = "<!-- END cargo-runner agent instructions -->";

/// Embedded copy of docs/AGENTS.cargo-runner.md (kept in crate assets for crates.io).
const EMBEDDED_DOC: &str = include_str!("../assets/AGENTS.cargo-runner.md");

const COMMON_NAMES: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    "GEMINI.md",
    "AGENT.md",
    ".cursorrules",
    ".windsurfrules",
    ".github/copilot-instructions.md",
    ".github/instructions/cargo-runner.instructions.md",
];

pub struct AgentInitOptions {
    pub root: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub dry_run: bool,
    pub create_agents: bool,
    pub source: Option<PathBuf>,
}

pub fn agent_init_command(opts: AgentInitOptions) -> Result<()> {
    let root = resolve_root(opts.root.as_deref())?;
    let source_text = load_source(opts.source.as_deref())?;
    let block = make_block(&source_text);

    println!("Project root : {}", root.display());
    if opts.dry_run {
        println!("Mode         : dry-run");
    }

    let explicit = !opts.paths.is_empty();
    let mut candidates: Vec<PathBuf> = if explicit {
        opts.paths
            .into_iter()
            .map(|p| {
                if p.is_absolute() {
                    p
                } else {
                    root.join(p)
                }
            })
            .collect()
    } else {
        scan_candidates(&root)?
    };

    // realpath string -> list of alias paths (for display)
    let mut by_real: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut missing_explicit: Vec<PathBuf> = Vec::new();

    for c in candidates.drain(..) {
        if c.is_symlink() {
            match fs::symlink_metadata(&c) {
                Ok(m) if m.file_type().is_symlink() => {
                    if !c.exists() {
                        eprintln!("  · skip broken symlink: {}", c.display());
                        continue;
                    }
                }
                _ => {}
            }
        }
        if !c.exists() {
            if explicit && opts.create_agents {
                missing_explicit.push(c);
            }
            continue;
        }
        if c.is_dir() {
            continue;
        }
        let real = fs::canonicalize(&c).with_context(|| format!("canonicalize {}", c.display()))?;
        if real.is_dir() {
            continue;
        }
        by_real.entry(real).or_default().push(c);
    }

    for c in missing_explicit {
        // Create as a normal file path (do not create symlink)
        by_real.entry(c.clone()).or_default().push(c);
    }

    if by_real.is_empty() && !explicit && opts.create_agents {
        let agents = root.join("AGENTS.md");
        println!(
            "No agent files found; will create {}",
            rel_display(&agents, &root)
        );
        by_real.insert(agents.clone(), vec![agents]);
    }

    if by_real.is_empty() {
        println!("Nothing to update.");
        anyhow::bail!("no agent instruction files updated");
    }

    let mut updated = 0usize;
    let mut skipped = 0usize;

    for (real, aliases) in &by_real {
        let display = format_aliases(aliases, &root);
        let link_note = symlink_note(aliases);

        let write_path = if real.exists() {
            real.clone()
        } else {
            // Prefer a non-symlink alias for create
            aliases
                .iter()
                .find(|a| !a.is_symlink())
                .cloned()
                .unwrap_or_else(|| aliases[0].clone())
        };

        let (action, new_text) = if write_path.exists() {
            let text = fs::read_to_string(&write_path)
                .with_context(|| format!("read {}", write_path.display()))?;
            if text.contains(BEGIN) && text.contains(END) {
                let new_text = replace_block(&text, &block);
                if new_text == text {
                    ("unchanged", text)
                } else {
                    ("update", new_text)
                }
            } else {
                let mut t = text;
                if !t.is_empty() && !t.ends_with('\n') {
                    t.push('\n');
                }
                if !t.trim().is_empty() {
                    t.push('\n');
                }
                t.push_str(&block);
                ("append", t)
            }
        } else {
            ("create", block.clone())
        };

        if action == "unchanged" {
            println!("= skip (already current): {display}{link_note}");
            skipped += 1;
            continue;
        }

        if opts.dry_run {
            println!("~ would {action}: {display}{link_note}");
            println!("    → {}", write_path.display());
            updated += 1;
            continue;
        }

        if let Some(parent) = write_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        // Write through to real file; if write_path is somehow a symlink, write via canonicalize
        let out = if write_path.is_symlink() {
            fs::canonicalize(&write_path)?
        } else {
            write_path.clone()
        };
        fs::write(&out, &new_text).with_context(|| format!("write {}", out.display()))?;
        println!("✓ {action}: {display}{link_note}");
        updated += 1;
    }

    println!();
    println!(
        "Done. updated={updated} skipped={skipped} unique_targets={}",
        by_real.len()
    );
    if opts.dry_run {
        println!("(dry-run: no files written)");
    }
    Ok(())
}

fn resolve_root(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(r) = explicit {
        return Ok(fs::canonicalize(r).unwrap_or_else(|_| r.to_path_buf()));
    }
    if let Ok(out) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        && out.status.success()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Ok(PathBuf::from(s));
        }
    }
    std::env::current_dir().context("cwd")
}

fn load_source(path: Option<&Path>) -> Result<String> {
    if let Some(p) = path {
        return fs::read_to_string(p).with_context(|| format!("read source {}", p.display()));
    }
    // Prefer docs next to a source checkout when developing
    let near = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/AGENTS.cargo-runner.md");
    if near.is_file() {
        if let Ok(s) = fs::read_to_string(&near) {
            return Ok(s);
        }
    }
    Ok(EMBEDDED_DOC.to_string())
}

fn make_block(source: &str) -> String {
    let body = if let Some(idx) = source.find("## Golden rule") {
        format!(
            "# cargo-runner — agent instructions\n\n{}",
            &source[idx..]
        )
    } else {
        source.to_string()
    };
    format!("{BEGIN}\n\n{}\n{END}\n", body.trim())
}

fn scan_candidates(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for name in COMMON_NAMES {
        out.push(root.join(name));
    }
    let cursor_rules = root.join(".cursor/rules");
    if cursor_rules.is_dir() {
        for entry in walkdir_shallow(&cursor_rules, 2)? {
            let ext = entry
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if matches!(ext, "md" | "mdc" | "markdown") {
                out.push(entry);
            }
        }
    }
    let claude = root.join(".claude");
    if claude.is_dir() {
        for entry in walkdir_shallow(&claude, 2)? {
            if entry.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(entry);
            }
        }
    }
    Ok(out)
}

fn walkdir_shallow(dir: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn rec(dir: &Path, depth: usize, max: usize, out: &mut Vec<PathBuf>) -> Result<()> {
        if depth > max {
            return Ok(());
        }
        for ent in fs::read_dir(dir)? {
            let ent = ent?;
            let p = ent.path();
            if p.is_dir() {
                rec(&p, depth + 1, max, out)?;
            } else if p.is_file() || p.is_symlink() {
                out.push(p);
            }
        }
        Ok(())
    }
    rec(dir, 1, max_depth, &mut out)?;
    Ok(out)
}

fn replace_block(text: &str, block: &str) -> String {
    let start = match text.find(BEGIN) {
        Some(i) => i,
        None => return text.to_string(),
    };
    let end_rel = match text[start..].find(END) {
        Some(i) => i,
        None => return text.to_string(),
    };
    let end = start + end_rel + END.len();
    // include trailing newline after END if present
    let mut end = end;
    if text[end..].starts_with('\n') {
        end += 1;
    }
    let mut out = String::new();
    out.push_str(&text[..start]);
    out.push_str(block.trim_end());
    out.push('\n');
    out.push_str(&text[end..]);
    out
}

fn rel_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn format_aliases(aliases: &[PathBuf], root: &Path) -> String {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for a in aliases {
        names.insert(rel_display(a, root));
    }
    names.into_iter().collect::<Vec<_>>().join(", ")
}

fn symlink_note(aliases: &[PathBuf]) -> String {
    let mut bits = Vec::new();
    for a in aliases {
        if a.is_symlink() {
            match fs::read_link(a) {
                Ok(t) => bits.push(format!(
                    "{}→{}",
                    a.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                    t.display()
                )),
                Err(_) => bits.push(format!(
                    "{}→?",
                    a.file_name().and_then(|s| s.to_str()).unwrap_or("?")
                )),
            }
        }
    }
    if bits.is_empty() {
        String::new()
    } else {
        format!(" (symlinks: {})", bits.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn replace_block_updates_middle() {
        let text = format!("# Head\n\n{BEGIN}\n old \n{END}\n\n# Tail\n");
        let block = format!("{BEGIN}\n\nnew\n\n{END}\n");
        let out = replace_block(&text, &block);
        assert!(out.contains("new"));
        assert!(!out.contains(" old "));
        assert!(out.contains("# Head"));
        assert!(out.contains("# Tail"));
    }

    #[test]
    fn agent_init_dedupes_symlinks() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("CLAUDE.md"), "# Project\n").unwrap();
        symlink("CLAUDE.md", root.join("AGENTS.md")).unwrap();
        symlink("CLAUDE.md", root.join("GEMINI.md")).unwrap();

        agent_init_command(AgentInitOptions {
            root: Some(root.to_path_buf()),
            paths: vec![],
            dry_run: false,
            create_agents: true,
            source: None,
        })
        .unwrap();

        let claude = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert!(claude.contains(BEGIN));
        assert!(claude.contains("Golden rule"));
        // symlinks still point at CLAUDE
        assert!(root.join("AGENTS.md").is_symlink());
        // only one managed block
        assert_eq!(claude.matches(BEGIN).count(), 1);

        // idempotent
        agent_init_command(AgentInitOptions {
            root: Some(root.to_path_buf()),
            paths: vec![],
            dry_run: false,
            create_agents: true,
            source: None,
        })
        .unwrap();
        let claude2 = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
        assert_eq!(claude2.matches(BEGIN).count(), 1);
    }
}
