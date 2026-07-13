//! `cargo runner nvim|vim|editor` — install/uninstall/status for the Neovim plugin.
//!
//! Packpath target (default):
//!   nvim: ~/.local/share/nvim/site/pack/cargo-runner/start/cargo-runner
//!   vim:  ~/.vim/pack/cargo-runner/start/cargo-runner

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MARKER_NAME: &str = ".cargo-runner-editor-install.json";
const PACK_VENDOR: &str = "cargo-runner";
const PACK_NAME: &str = "cargo-runner";

/// Files embedded for crates.io / binary installs (synced from extensions/nvim).
fn embedded_plugin_files() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "README.md",
            include_str!("../assets/nvim-plugin/README.md"),
        ),
        (
            "plugin/cargo-runner.lua",
            include_str!("../assets/nvim-plugin/plugin/cargo-runner.lua"),
        ),
        (
            "doc/cargo-runner.txt",
            include_str!("../assets/nvim-plugin/doc/cargo-runner.txt"),
        ),
        (
            "lua/cargo_runner/init.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/init.lua"),
        ),
        (
            "lua/cargo_runner/config.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/config.lua"),
        ),
        (
            "lua/cargo_runner/cli.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/cli.lua"),
        ),
        (
            "lua/cargo_runner/run.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/run.lua"),
        ),
        (
            "lua/cargo_runner/override.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/override.lua"),
        ),
        (
            "lua/cargo_runner/notify.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/notify.lua"),
        ),
        (
            "lua/cargo_runner/progress.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/progress.lua"),
        ),
        (
            "lua/cargo_runner/kind.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/kind.lua"),
        ),
        (
            "lua/cargo_runner/jobs.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/jobs.lua"),
        ),
        (
            "lua/cargo_runner/job.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/job.lua"),
        ),
        (
            "lua/cargo_runner/state.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/state.lua"),
        ),
        (
            "lua/cargo_runner/ui/hud.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/ui/hud.lua"),
        ),
        (
            "lua/cargo_runner/ui/peek.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/ui/peek.lua"),
        ),
        (
            "lua/cargo_runner/ui/error_float.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/ui/error_float.lua"),
        ),
        (
            "lua/cargo_runner/ui/panel.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/ui/panel.lua"),
        ),
        (
            "lua/cargo_runner/ui/toast.lua",
            include_str!("../assets/nvim-plugin/lua/cargo_runner/ui/toast.lua"),
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    Nvim,
    Vim,
}

impl EditorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EditorKind::Nvim => "nvim",
            EditorKind::Vim => "vim",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorAction {
    Install,
    Uninstall,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMethod {
    Pack,
    Print,
}

#[derive(Debug, Clone)]
pub struct EditorInstallOptions {
    pub action: EditorAction,
    /// User-facing request: nvim | vim | auto
    pub requested: String,
    pub method: InstallMethod,
    pub dry_run: bool,
    pub follow_shell_alias: bool,
    pub strict_vim: bool,
    /// Prefer symlink to monorepo extensions/nvim when available
    pub prefer_symlink: bool,
    /// Exact plugin pack directory (overrides data-home / app-name / vim-dir)
    pub pack_dir: Option<PathBuf>,
    /// Override XDG_DATA_HOME for Neovim packpath
    pub data_home: Option<PathBuf>,
    /// Neovim app name (`NVIM_APPNAME`), e.g. `nvim`, `nvim-lazy`, `astronvim`
    pub app_name: Option<String>,
    /// Path to user config dir (for status / setup snippet hints only)
    pub config_dir: Option<PathBuf>,
    /// Classic Vim runtime root (default `~/.vim`)
    pub vim_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct InstallMarker {
    version: u32,
    editor: String,
    installed_at: String,
    method: String,
    source: String,
    pack_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    config_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_name: Option<String>,
}

#[derive(Debug)]
struct ResolvedEditor {
    kind: EditorKind,
    binary: PathBuf,
    version_line: String,
    notes: Vec<String>,
}

pub fn editor_install_command(opts: EditorInstallOptions) -> Result<()> {
    let resolved = resolve_editor(
        &opts.requested,
        opts.follow_shell_alias,
        opts.strict_vim,
    )?;

    match opts.action {
        EditorAction::Status => status(&resolved, &opts),
        EditorAction::Install => install(&resolved, &opts),
        EditorAction::Uninstall => uninstall(&resolved, &opts),
    }
}

fn resolve_editor(
    requested: &str,
    follow_shell_alias: bool,
    strict_vim: bool,
) -> Result<ResolvedEditor> {
    let req = requested.trim().to_ascii_lowercase();
    match req.as_str() {
        "nvim" | "neovim" => resolve_nvim(Vec::new()),
        "vim" => resolve_vim(follow_shell_alias, strict_vim),
        "auto" | "" => {
            if which("nvim").is_some() {
                resolve_nvim(vec![
                    "auto: preferred nvim (found on PATH)".into(),
                ])
            } else if which("vim").is_some() {
                resolve_vim(follow_shell_alias, strict_vim)
            } else {
                bail!("neither nvim nor vim found on PATH");
            }
        }
        other => bail!("unknown editor '{other}' (use nvim, vim, or auto)"),
    }
}

fn resolve_nvim(mut notes: Vec<String>) -> Result<ResolvedEditor> {
    let binary = which("nvim").context("nvim not found on PATH")?;
    let version_line = version_first_line(&binary)?;
    if !version_line.to_ascii_uppercase().contains("NVIM") {
        notes.push(format!(
            "warning: `nvim --version` did not look like Neovim: {version_line}"
        ));
    }
    Ok(ResolvedEditor {
        kind: EditorKind::Nvim,
        binary,
        version_line,
        notes,
    })
}

fn resolve_vim(follow_shell_alias: bool, strict_vim: bool) -> Result<ResolvedEditor> {
    let mut notes = Vec::new();

    if follow_shell_alias {
        if let Some(alias_target) = shell_alias_target("vim") {
            notes.push(format!(
                "shell alias: vim → {alias_target} (--follow-shell-alias)"
            ));
            let target = alias_target.trim().trim_matches('\'');
            if target == "nvim" || target.ends_with("/nvim") || target.contains("nvim") {
                let mut n = notes;
                n.push("treating as Neovim because of shell alias".into());
                return resolve_nvim(n);
            }
        }
    }

    // Direct binary on PATH (ignores interactive aliases)
    let vim_bin = which("vim");
    let nvim_bin = which("nvim");

    if let Some(ref v) = vim_bin {
        // Some systems ship vim that is actually nvim (rare) — check version
        if let Ok(line) = version_first_line(v) {
            if line.to_ascii_uppercase().contains("NVIM") {
                notes.push(format!(
                    "`vim` binary reports Neovim: {}",
                    v.display()
                ));
                return Ok(ResolvedEditor {
                    kind: EditorKind::Nvim,
                    binary: v.clone(),
                    version_line: line,
                    notes,
                });
            }
        }
    }

    // Prefer Neovim when both exist (common macOS setup: real /usr/bin/vim + brew nvim,
    // with interactive `alias vim=nvim`). Use --strict-vim for classic Vim packpath.
    if !strict_vim {
        if let (Some(_vim), Some(nvim)) = (&vim_bin, &nvim_bin) {
            notes.push(
                "note: both `vim` and `nvim` are on PATH. Installing for Neovim. \
                 Pass --strict-vim to force classic Vim packpath (~/.vim/pack/...)."
                    .into(),
            );
            let mut n = notes;
            n.push(format!("using nvim at {}", nvim.display()));
            return resolve_nvim(n);
        }
    }

    let binary = vim_bin.context("vim not found on PATH")?;
    let version_line = version_first_line(&binary)?;
    Ok(ResolvedEditor {
        kind: EditorKind::Vim,
        binary,
        version_line,
        notes,
    })
}

/// Run interactive login shell so user aliases (e.g. `alias vim=nvim`) load.
fn shell_alias_target(name: &str) -> Option<String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    // -l login, -i interactive (aliases), -c command
    let output = Command::new(&shell)
        .args(["-lic"])
        .arg(format!(
            "alias {name} 2>/dev/null; type {name} 2>/dev/null; command -v {name} 2>/dev/null"
        ))
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    // zsh: "vim is an alias for nvim" or "vim=nvim"
    // bash: "alias vim='nvim'" / "vim is aliased to `nvim'"
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("alias ") {
            if let Some((_, rhs)) = line.split_once('=') {
                return Some(rhs.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }
        if lower.contains("alias") {
            if let Some(idx) = lower.find(" for ") {
                return Some(line[idx + 5..].trim().trim_matches('\'').to_string());
            }
            if let Some(idx) = lower.find(" to ") {
                let rest = line[idx + 4..].trim();
                return Some(rest.trim_matches('`').trim_matches('\'').to_string());
            }
            if let Some((_, rhs)) = line.split_once('=') {
                return Some(rhs.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }
        // bare `vim=nvim` from `alias vim`
        if let Some((lhs, rhs)) = line.split_once('=') {
            if lhs.trim() == name {
                return Some(rhs.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }
    }
    None
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            // executable bit best-effort
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = candidate.metadata() {
                    if meta.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
            }
            return Some(candidate);
        }
    }
    None
}

fn version_first_line(binary: &Path) -> Result<String> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .with_context(|| format!("failed to run {} --version", binary.display()))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .next()
        .unwrap_or("(no version output)")
        .trim()
        .to_string();
    Ok(line)
}

/// Resolve where the pack plugin tree is installed.
///
/// Precedence:
/// 1. `--pack-dir` (exact path to `…/start/cargo-runner`)
/// 2. Neovim: `{data_home}/{app_name}/site/pack/cargo-runner/start/cargo-runner`
///    - `data_home`: `--data-home` → `$XDG_DATA_HOME` → `~/.local/share`
///    - `app_name`: `--app-name` → `$NVIM_APPNAME` → `nvim`
/// 3. Vim: `{vim_dir}/pack/cargo-runner/start/cargo-runner`
///    - `vim_dir`: `--vim-dir` → `~/.vim`
fn pack_root(kind: EditorKind, opts: &EditorInstallOptions) -> Result<PathBuf> {
    if let Some(ref p) = opts.pack_dir {
        return Ok(expand_user(p));
    }
    match kind {
        EditorKind::Nvim => {
            let data = if let Some(ref d) = opts.data_home {
                expand_user(d)
            } else if let Ok(xdg) = env::var("XDG_DATA_HOME") {
                PathBuf::from(xdg)
            } else {
                dirs_home()?.join(".local/share")
            };
            let app = opts
                .app_name
                .clone()
                .or_else(|| env::var("NVIM_APPNAME").ok())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "nvim".into());
            Ok(data
                .join(app)
                .join("site/pack")
                .join(PACK_VENDOR)
                .join("start")
                .join(PACK_NAME))
        }
        EditorKind::Vim => {
            let vim_root = if let Some(ref d) = opts.vim_dir {
                expand_user(d)
            } else {
                dirs_home()?.join(".vim")
            };
            Ok(vim_root
                .join("pack")
                .join(PACK_VENDOR)
                .join("start")
                .join(PACK_NAME))
        }
    }
}

/// Default or user-provided config directory (for setup snippets / status).
fn config_root(kind: EditorKind, opts: &EditorInstallOptions) -> Result<PathBuf> {
    if let Some(ref c) = opts.config_dir {
        return Ok(expand_user(c));
    }
    match kind {
        EditorKind::Nvim => {
            let cfg_home = if let Ok(x) = env::var("XDG_CONFIG_HOME") {
                PathBuf::from(x)
            } else {
                dirs_home()?.join(".config")
            };
            let app = opts
                .app_name
                .clone()
                .or_else(|| env::var("NVIM_APPNAME").ok())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "nvim".into());
            Ok(cfg_home.join(app))
        }
        EditorKind::Vim => {
            if let Some(ref d) = opts.vim_dir {
                Ok(expand_user(d))
            } else {
                Ok(dirs_home()?.join(".vim"))
            }
        }
    }
}

fn expand_user(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = dirs_home() {
            return home.join(rest);
        }
    }
    if s == "~" {
        if let Ok(home) = dirs_home() {
            return home;
        }
    }
    p.to_path_buf()
}

fn print_setup_hint(kind: EditorKind, config_dir: &Path, pack_dir: &Path) {
    println!();
    println!("Config dir  : {}", config_dir.display());
    println!("Pack dir    : {}", pack_dir.display());
    match kind {
        EditorKind::Nvim => {
            let init = config_dir.join("init.lua");
            println!();
            println!("Optional setup (only needed for custom options):");
            println!("  # {}", init.display());
            println!("  require(\"cargo_runner\").setup({{");
            println!("    -- binary = vim.fn.expand(\"~/.cargo/bin/cargo-runner\"),");
            println!("    -- map_super = \"auto\",");
            println!("  }})");
            println!();
            println!("Packpath install loads the plugin without editing init.lua.");
            println!("If you use a custom data dir / NVIM_APPNAME, pass the same flags");
            println!("to install/status so paths stay in sync.");
        }
        EditorKind::Vim => {
            println!();
            println!("Classic Vim packpath: {}", pack_dir.display());
            println!("Plugin is Lua-first (Neovim 0.9+ recommended).");
        }
    }
}

fn dirs_home() -> Result<PathBuf> {
    if let Ok(h) = env::var("HOME") {
        return Ok(PathBuf::from(h));
    }
    #[cfg(windows)]
    {
        if let Ok(h) = env::var("USERPROFILE") {
            return Ok(PathBuf::from(h));
        }
    }
    bail!("HOME is not set");
}

fn monorepo_plugin_dir() -> Option<PathBuf> {
    // Walk from cwd upward looking for extensions/nvim/lua/cargo_runner/init.lua
    let mut cur = env::current_dir().ok()?;
    for _ in 0..8 {
        let candidate = cur.join("extensions/nvim");
        if candidate.join("lua/cargo_runner/init.lua").is_file() {
            return Some(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    // Relative to this crate at compile time (dev builds only — may not exist after install)
    let manifest_adjacent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../extensions/nvim")
        .canonicalize()
        .ok();
    if let Some(ref p) = manifest_adjacent {
        if p.join("lua/cargo_runner/init.lua").is_file() {
            return Some(p.clone());
        }
    }
    None
}

fn install(resolved: &ResolvedEditor, opts: &EditorInstallOptions) -> Result<()> {
    for n in &resolved.notes {
        println!("  · {n}");
    }

    let dest = pack_root(resolved.kind, opts)?;
    let config_dir = config_root(resolved.kind, opts)?;

    if opts.method == InstallMethod::Print {
        print_plugin_manager_snippets(resolved.kind, &dest, &config_dir);
        print_setup_hint(resolved.kind, &config_dir, &dest);
        return Ok(());
    }

    println!("Editor     : {} ({})", resolved.kind.as_str(), resolved.binary.display());
    println!("Version    : {}", resolved.version_line);
    println!("Install to : {}", dest.display());
    println!("Config dir : {}", config_dir.display());
    if let Some(ref app) = opts.app_name {
        println!("App name   : {app}");
    } else if let Ok(app) = env::var("NVIM_APPNAME") {
        println!("App name   : {app} (from $NVIM_APPNAME)");
    }

    if opts.dry_run {
        println!("Mode       : dry-run (no files written)");
        if let Some(src) = monorepo_plugin_dir() {
            println!("Source     : symlink → {}", src.display());
        } else {
            println!("Source     : embedded plugin assets ({} files)", embedded_plugin_files().len());
        }
        println!("Keymaps    : <leader>r run · <leader>R override · <leader>ro peek");
        print_setup_hint(resolved.kind, &config_dir, &dest);
        return Ok(());
    }

    if dest.exists() {
        // Only remove if ours or empty-ish
        if dest.join(MARKER_NAME).is_file() || is_cargo_runner_pack(&dest) {
            fs::remove_dir_all(&dest)
                .with_context(|| format!("remove existing {}", dest.display()))?;
        } else {
            bail!(
                "refusing to overwrite {} (no {} marker). Remove it manually or pick another path.",
                dest.display(),
                MARKER_NAME
            );
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }

    let source_label;
    if opts.prefer_symlink {
        if let Some(src) = monorepo_plugin_dir() {
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&src, &dest).with_context(|| {
                    format!("symlink {} → {}", dest.display(), src.display())
                })?;
                source_label = format!("symlink:{}", src.display());
                println!("Installed  : symlink → {}", src.display());
            }
            #[cfg(not(unix))]
            {
                copy_dir_recursive(&src, &dest)?;
                source_label = format!("copy:{}", src.display());
                println!("Installed  : copied from {}", src.display());
            }
        } else {
            write_embedded(&dest)?;
            source_label = "embedded".into();
            println!(
                "Installed  : embedded assets ({} files)",
                embedded_plugin_files().len()
            );
        }
    } else if let Some(src) = monorepo_plugin_dir() {
        // Default: if monorepo present, still copy embedded for predictability unless prefer_symlink
        // Prefer copy from monorepo so local edits apply without rebuild
        copy_dir_recursive(&src, &dest)?;
        source_label = format!("copy:{}", src.display());
        println!("Installed  : copied from {}", src.display());
    } else {
        write_embedded(&dest)?;
        source_label = "embedded".into();
        println!(
            "Installed  : embedded assets ({} files)",
            embedded_plugin_files().len()
        );
    }

    let app_name = opts
        .app_name
        .clone()
        .or_else(|| env::var("NVIM_APPNAME").ok());
    let marker = InstallMarker {
        version: 2,
        editor: resolved.kind.as_str().to_string(),
        installed_at: chrono_like_now(),
        method: "pack".into(),
        source: source_label,
        pack_dir: dest.display().to_string(),
        config_dir: Some(config_dir.display().to_string()),
        app_name,
    };
    let marker_path = dest.join(MARKER_NAME);
    fs::write(
        &marker_path,
        serde_json::to_string_pretty(&marker).context("serialize marker")?,
    )
    .with_context(|| format!("write {}", marker_path.display()))?;

    println!();
    println!("Done. Restart Neovim (or :packloadall) and open a .rs file.");
    println!("  <leader>r    run at cursor (async, multi-job)");
    println!("  <leader>R    override at cursor");
    println!("  <leader>ro   peek live/history output");
    println!("  <leader>rj   job list");
    println!("  <leader>rk   kill focused job");
    println!("  :CargoRunnerStatus");
    if resolved.kind == EditorKind::Nvim {
        println!();
        println!("Status UI: one fixed-width top-right panel (jobs + notices).");
        println!("Cmd+R: Neovide/GUI via <D-r>; terminal nvim uses <leader>r.");
    }
    print_setup_hint(resolved.kind, &config_dir, &dest);
    Ok(())
}

fn is_cargo_runner_pack(dir: &Path) -> bool {
    dir.join("lua/cargo_runner/init.lua").is_file()
        || dir.join("plugin/cargo-runner.lua").is_file()
}

fn write_embedded(dest: &Path) -> Result<()> {
    for (rel, content) in embedded_plugin_files() {
        let path = dest.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)
            .with_context(|| format!("write {}", path.display()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for entry in walkdir::WalkDir::new(src)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        if rel.as_os_str().is_empty() {
            continue;
        }
        // skip hidden junk
        if rel
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let target = dest.join(rel);
        if path.is_dir() {
            fs::create_dir_all(&target)?;
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)
                .with_context(|| format!("copy {} → {}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

fn uninstall(resolved: &ResolvedEditor, opts: &EditorInstallOptions) -> Result<()> {
    for n in &resolved.notes {
        println!("  · {n}");
    }
    let dest = pack_root(resolved.kind, opts)?;
    println!("Editor     : {} ({})", resolved.kind.as_str(), resolved.binary.display());
    println!("Remove     : {}", dest.display());

    if !dest.exists() {
        println!("Not installed (path missing).");
        println!("Tip: pass the same --pack-dir / --app-name / --data-home used at install.");
        return Ok(());
    }

    if !dest.join(MARKER_NAME).is_file() && !is_cargo_runner_pack(&dest) {
        bail!(
            "refusing to remove {} — not a cargo-runner install (missing marker)",
            dest.display()
        );
    }

    if opts.dry_run {
        println!("Mode       : dry-run (would remove)");
        return Ok(());
    }

    // Symlink vs directory
    let meta = fs::symlink_metadata(&dest)
        .with_context(|| format!("stat {}", dest.display()))?;
    if meta.file_type().is_symlink() {
        fs::remove_file(&dest).with_context(|| format!("remove symlink {}", dest.display()))?;
    } else {
        fs::remove_dir_all(&dest)
            .with_context(|| format!("remove {}", dest.display()))?;
    }
    println!("Uninstalled.");
    Ok(())
}

fn status(resolved: &ResolvedEditor, opts: &EditorInstallOptions) -> Result<()> {
    for n in &resolved.notes {
        println!("  · {n}");
    }
    let dest = pack_root(resolved.kind, opts)?;
    let config_dir = config_root(resolved.kind, opts)?;
    let installed = dest.exists()
        && (dest.join(MARKER_NAME).is_file() || is_cargo_runner_pack(&dest));

    println!("Editor binary : {}", resolved.binary.display());
    println!("Editor kind   : {}", resolved.kind.as_str());
    println!("Version       : {}", resolved.version_line);
    println!(
        "Plugin path   : {} ({})",
        dest.display(),
        if installed { "installed" } else { "not installed" }
    );
    println!("Config dir    : {}", config_dir.display());
    if let Some(ref app) = opts.app_name {
        println!("App name      : {app}");
    } else if let Ok(app) = env::var("NVIM_APPNAME") {
        println!("App name      : {app} ($NVIM_APPNAME)");
    }

    if dest.join(MARKER_NAME).is_file() {
        if let Ok(text) = fs::read_to_string(dest.join(MARKER_NAME)) {
            println!("Marker        : {}", text.replace('\n', " "));
        }
    }

    // cargo-runner CLI presence
    match which("cargo-runner") {
        Some(p) => {
            let ver = version_first_line(&p).unwrap_or_else(|_| "?".into());
            println!("cargo-runner  : {} ({})", p.display(), ver);
        }
        None => {
            let cargo_bin = dirs_home()
                .map(|h| h.join(".cargo/bin/cargo-runner"))
                .ok();
            if let Some(p) = cargo_bin {
                if p.is_file() {
                    let ver = version_first_line(&p).unwrap_or_else(|_| "?".into());
                    println!(
                        "cargo-runner  : {} ({}) — not on PATH; plugin will auto-find ~/.cargo/bin",
                        p.display(),
                        ver
                    );
                } else {
                    println!("cargo-runner  : NOT on PATH (cargo binstall cargo-runner-cli)");
                }
            } else {
                println!("cargo-runner  : NOT on PATH (cargo binstall cargo-runner-cli)");
            }
        }
    }

    // Alias probe (informational)
    if let Some(alias) = shell_alias_target("vim") {
        println!("shell alias   : vim → {alias}");
    }

    println!();
    println!("Keymaps (plugin defaults):");
    println!("  <leader>r     run at cursor (async)");
    println!("  <leader>R     override modal");
    println!("  <leader>ro    peek output");
    println!("  <leader>rj    job list");
    println!("  <leader>rk    kill job");
    println!("  <D-r>/<D-S-r> Super/Cmd when GUI or map_super=true");
    println!();
    println!("Status panel: fixed-width top-right (jobs + notices). Docs: docs/nvim-status-panel.md");
    print_setup_hint(resolved.kind, &config_dir, &dest);
    Ok(())
}

fn print_plugin_manager_snippets(kind: EditorKind, pack_dir: &Path, config_dir: &Path) {
    println!("# Manual / plugin-manager install snippets");
    println!();
    println!("## packpath (same as `cargo runner {} install`)", kind.as_str());
    println!("# {}", pack_dir.display());
    println!();
    println!("## lazy.nvim");
    println!(
        r#"{{
  dir = [[{pack}]],
  config = function()
    require("cargo_runner").setup()
  end,
}}"#,
        pack = pack_dir.display()
    );
    println!();
    println!("## init.lua setup (config dir: {})", config_dir.display());
    println!("require(\"cargo_runner\").setup({{ }})");
    println!();
    println!("## vim-plug");
    println!("\" After pack install, packpath already loads the plugin.");
    println!("\" Or: Plug 'cargo-runner/cargo-runner', {{ 'rtp': 'extensions/nvim' }}");
}

fn chrono_like_now() -> String {
    // Avoid extra dep — RFC-ish local timestamp via system
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env-mutating tests
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn embedded_files_nonempty() {
        let files = embedded_plugin_files();
        assert!(files.len() >= 10);
        assert!(files.iter().any(|(p, _)| *p == "lua/cargo_runner/init.lua"));
        for (path, content) in files {
            assert!(!content.is_empty(), "{path} empty");
        }
    }

    #[test]
    fn write_embedded_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("plugin");
        write_embedded(&dest).unwrap();
        assert!(dest.join("lua/cargo_runner/init.lua").is_file());
        assert!(dest.join("plugin/cargo-runner.lua").is_file());
    }

    fn empty_opts(action: EditorAction) -> EditorInstallOptions {
        EditorInstallOptions {
            action,
            requested: "nvim".into(),
            method: InstallMethod::Pack,
            dry_run: false,
            follow_shell_alias: false,
            strict_vim: false,
            prefer_symlink: false,
            pack_dir: None,
            data_home: None,
            app_name: None,
            config_dir: None,
            vim_dir: None,
        }
    }

    #[test]
    fn pack_root_nvim_uses_xdg() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: tests serialize env mutation via ENV_LOCK
        unsafe {
            env::set_var("XDG_DATA_HOME", dir.path());
            env::remove_var("NVIM_APPNAME");
        }
        let opts = empty_opts(EditorAction::Status);
        let root = pack_root(EditorKind::Nvim, &opts).unwrap();
        assert!(root.starts_with(dir.path()));
        assert!(root.ends_with("nvim/site/pack/cargo-runner/start/cargo-runner"));
        unsafe {
            env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn pack_root_respects_app_name_and_pack_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut opts = empty_opts(EditorAction::Install);
        opts.data_home = Some(dir.path().to_path_buf());
        opts.app_name = Some("nvim-lazy".into());
        let root = pack_root(EditorKind::Nvim, &opts).unwrap();
        assert!(root.ends_with("nvim-lazy/site/pack/cargo-runner/start/cargo-runner"));

        let custom = dir.path().join("custom/pack/start/cargo-runner");
        opts.pack_dir = Some(custom.clone());
        assert_eq!(pack_root(EditorKind::Nvim, &opts).unwrap(), custom);
    }

    #[test]
    fn config_root_respects_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut opts = empty_opts(EditorAction::Status);
        opts.config_dir = Some(dir.path().join("my-nvim"));
        let cfg = config_root(EditorKind::Nvim, &opts).unwrap();
        assert_eq!(cfg, dir.path().join("my-nvim"));
    }

    #[test]
    fn install_uninstall_with_temp_home() {
        let _g = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        // SAFETY: tests serialize env mutation via ENV_LOCK
        unsafe {
            env::set_var("HOME", home.path());
            env::remove_var("XDG_DATA_HOME");
            env::remove_var("NVIM_APPNAME");
        }

        // Need nvim on PATH for resolve — if missing, skip
        if which("nvim").is_none() {
            unsafe {
                env::remove_var("HOME");
            }
            return;
        }

        let opts = empty_opts(EditorAction::Install);
        editor_install_command(opts).unwrap();
        let dest = pack_root(EditorKind::Nvim, &empty_opts(EditorAction::Status)).unwrap();
        assert!(dest.join("lua/cargo_runner/init.lua").is_file());
        assert!(dest.join(MARKER_NAME).is_file());

        editor_install_command(empty_opts(EditorAction::Uninstall)).unwrap();
        assert!(!dest.exists());

        unsafe {
            env::remove_var("HOME");
        }
    }
}
