use crate::{
    command::Command,
    error::{Error, Result},
    plugins::{
        CommandSpec, CommandStrategy, ProjectContext, RustSourceAnalyzer, SourceAnalyzer, TargetRef,
    },
    runners::{
        bazel_runner::BazelRunner, cargo_runner::CargoRunner, rustc_runner::RustcRunner,
        traits::CommandRunner,
    },
    types::{FileType, Runnable, RunnableKind},
};

pub struct BazelPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: BazelRunner,
}

pub struct CargoPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: CargoRunner,
}

pub struct RustcPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: RustcRunner,
}

pub struct DioxusOverlayPlugin;
pub struct LeptosOverlayPlugin;
pub struct TauriOverlayPlugin;

impl Default for BazelPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BazelPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: BazelRunner,
        }
    }
}

impl Default for CargoPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: CargoRunner,
        }
    }
}

impl Default for RustcPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RustcPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: RustcRunner,
        }
    }
}

impl Default for DioxusOverlayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DioxusOverlayPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeptosOverlayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LeptosOverlayPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TauriOverlayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TauriOverlayPlugin {
    pub fn new() -> Self {
        Self
    }
}

/// Returns true when the nearest crate `Cargo.toml` declares `crate_name` as a
/// direct dependency (or dependency feature table).
///
/// Avoids false positives from comments / unrelated strings like `"my-leptos-notes"`.
fn cargo_depends_on(cargo_toml: &std::path::Path, crate_name: &str) -> bool {
    let Ok(content) = std::fs::read_to_string(cargo_toml) else {
        return false;
    };
    // Prefer structured parse when possible
    if let Ok(manifest) = cargo_toml::Manifest::from_str(&content) {
        let has = |deps: &cargo_toml::DepsSet| deps.contains_key(crate_name);
        if has(&manifest.dependencies)
            || has(&manifest.dev_dependencies)
            || has(&manifest.build_dependencies)
        {
            return true;
        }
        // workspace.dependencies (Cargo workspaces)
        if let Some(ws) = &manifest.workspace
            && has(&ws.dependencies)
        {
            return true;
        }
    }

    // Fallback: line-oriented match for `name =` under a dependency-like section
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[build-dependencies]"
                || trimmed.starts_with("[dependencies.")
                || trimmed == "[workspace.dependencies]";
            continue;
        }
        if !in_deps || trimmed.starts_with('#') {
            continue;
        }
        // `leptos = "…"` or `leptos = { … }` or `"leptos" = …`
        if let Some(rest) = trimmed.strip_prefix(crate_name)
            && (rest.starts_with('=') || rest.starts_with(' ') || rest.starts_with('\t'))
        {
            return true;
        }
        let quoted = format!("\"{crate_name}\"");
        if let Some(rest) = trimmed.strip_prefix(&quoted)
            && (rest.starts_with('=') || rest.trim_start().starts_with('='))
        {
            return true;
        }
    }
    false
}

/// True when this path is a Tauri app: `tauri.conf.json` nearby, or a `tauri` crate dep.
fn is_tauri_project(ctx: &ProjectContext) -> bool {
    if ctx.has_manifest("tauri.conf.json") {
        return true;
    }
    // Common layout: src-tauri/tauri.conf.json while editing src/
    let start = if ctx.file_path.is_file() {
        ctx.file_path.parent().unwrap_or(&ctx.file_path)
    } else {
        &ctx.file_path
    };
    for ancestor in start.ancestors() {
        if ancestor.join("tauri.conf.json").exists()
            || ancestor.join("src-tauri/tauri.conf.json").exists()
        {
            return true;
        }
        if let Some(cargo) = ancestor
            .join("Cargo.toml")
            .exists()
            .then(|| ancestor.join("Cargo.toml"))
            && cargo_depends_on(&cargo, "tauri")
        {
            return true;
        }
        if ancestor.join("MODULE.bazel").exists() || ancestor.join("WORKSPACE").exists() {
            break;
        }
    }
    ctx.manifest("Cargo.toml")
        .is_some_and(|p| cargo_depends_on(p, "tauri"))
}

/// Returns true when `file_path` is inside a Cargo *crate* (a directory that has
/// its own `Cargo.toml` with a `[package]` section), as opposed to being a loose
/// `.rs` file inside a workspace root whose `Cargo.toml` only has `[workspace]`.
fn is_cargo_owned_rs(file_path: &std::path::Path) -> bool {
    let start = if file_path.is_file() {
        file_path.parent().unwrap_or(file_path)
    } else {
        file_path
    };

    for ancestor in start.ancestors() {
        let candidate = ancestor.join("Cargo.toml");
        if candidate.exists() {
            // Read the Cargo.toml and check for [package]
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[package]") {
                    return true;
                }
                // It's a workspace-only manifest — keep walking up to see if
                // there's a real crate above, but typically there won't be.
                // Treat this as "not owned" and stop.
                return false;
            }
        }
    }
    false
}

fn file_type_for_runnable(runnable: &Runnable) -> FileType {
    match runnable.kind {
        RunnableKind::SingleFileScript { .. } => FileType::SingleFileScript,
        RunnableKind::Standalone { .. } => FileType::Standalone,
        _ => FileType::CargoProject,
    }
}

fn runnable_from_target(target: &TargetRef) -> Result<&Runnable> {
    target
        .runnable
        .as_ref()
        .ok_or_else(|| Error::TargetNotRunnable {
            label: target.label.to_string(),
        })
}

fn to_spec(command: Command) -> CommandSpec {
    CommandSpec::from(command)
}

fn identity_for_override(
    runnable: &Runnable,
    ctx: &ProjectContext,
) -> crate::types::FunctionIdentity {
    let package = ctx
        .config
        .cargo
        .as_ref()
        .and_then(|c| c.package.clone())
        .or_else(|| crate::runners::common::get_cargo_package_name(&runnable.file_path));
    crate::types::FunctionIdentity {
        package,
        module_path: if runnable.module_path.is_empty() {
            None
        } else {
            Some(runnable.module_path.clone())
        },
        file_path: Some(runnable.file_path.clone()),
        function_name: runnable.get_function_name(),
        file_type: Some(file_type_for_runnable(runnable)),
    }
}

/// Build a shell command from a cargo-section override when `command` is not `cargo`
/// (e.g. `@spin.build --up` → `spin build --up`). Used so framework overlays
/// (Leptos/Tauri) do not force `cargo leptos` / `cargo tauri` over custom tools.
fn shell_command_from_cargo_override(
    base: CommandSpec,
    ov: &crate::config::CargoConfig,
    cmd: &str,
) -> CommandSpec {
    let mut args = Vec::new();
    if let Some(sub) = &ov.subcommand {
        args.extend(sub.split_whitespace().map(String::from));
    }
    if let Some(extra) = &ov.extra_args {
        args.extend(extra.clone());
    }
    let mut env = base.env.clone();
    if let Some(extra_env) = &ov.extra_env {
        env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    CommandSpec {
        strategy: CommandStrategy::Shell,
        program: cmd.to_string(),
        args,
        working_dir: base.working_dir,
        env,
        test_filter: base.test_filter,
        exec_args: base.exec_args,
        pipe_command: base.pipe_command,
        test_binary_args: base.test_binary_args,
    }
}

impl crate::plugins::registry::PrimaryPlugin for BazelPrimaryPlugin {
    fn id(&self) -> &'static str {
        "bazel"
    }

    fn default_priority(&self) -> i32 {
        300
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        if !ctx.has_manifest("BUILD.bazel") && !ctx.has_manifest("BUILD") {
            return false;
        }

        let file_dir = if ctx.file_path.is_file() {
            ctx.file_path.parent().unwrap_or(&ctx.file_path)
        } else {
            &ctx.file_path
        };

        for ancestor in file_dir.ancestors() {
            let is_workspace_root =
                ancestor.join("MODULE.bazel").exists() || ancestor.join("WORKSPACE").exists();
            let has_build_file =
                ancestor.join("BUILD.bazel").exists() || ancestor.join("BUILD").exists();

            if is_workspace_root {
                // If we reach the workspace root and it has a BUILD file, we only match
                // if BazelTargetFinder actually finds a target for this file.
                // Otherwise, it's likely an ad-hoc or standard rust file at the root.
                if has_build_file
                    && let Ok(mut finder) = crate::bazel::BazelTargetFinder::new()
                    && let Ok(targets) = finder.find_targets_for_file(&ctx.file_path, ancestor)
                {
                    return !targets.is_empty();
                }
                return false;
            }

            if has_build_file {
                return true;
            }
        }
        false
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        self.runner.validate_command(&command)?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::PrimaryPlugin for CargoPrimaryPlugin {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn default_priority(&self) -> i32 {
        200
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        if !ctx.has_manifest("Cargo.toml") {
            return false;
        }

        // If the file is a .rs source file, verify it is actually owned by a Cargo
        // crate (i.e., it lives inside a directory that contains, or whose ancestor
        // contains, a Cargo.toml with [package] — not just the workspace root).
        // Standalone scripts placed at the workspace root should fall through to
        // the RustcPrimaryPlugin instead.
        if ctx.file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            return is_cargo_owned_rs(&ctx.file_path);
        }

        true
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        self.runner.validate_command(&command)?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::PrimaryPlugin for RustcPrimaryPlugin {
    fn id(&self) -> &'static str {
        "rustc"
    }

    fn default_priority(&self) -> i32 {
        100
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        ctx.file_path.extension().and_then(|s| s.to_str()) == Some("rs")
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        self.runner.validate_command(&command)?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::OverlayPlugin for DioxusOverlayPlugin {
    fn id(&self) -> &'static str {
        "dioxus"
    }

    fn default_priority(&self) -> i32 {
        200
    }

    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool {
        primary_id == "cargo"
            && ctx.has_manifest("Dioxus.toml")
            && target
                .runnable
                .as_ref()
                .is_some_and(|r| matches!(r.kind, RunnableKind::Binary { .. }))
    }

    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        if !matches!(runnable.kind, RunnableKind::Binary { .. }) {
            return Ok(command);
        }

        let mut command_name = "dx".to_string();
        let mut subcommand = "serve".to_string();
        let mut extra_args = Vec::new();
        let mut env = command.env.clone();

        if let Some(override_config) = ctx
            .config
            .get_override_for(&identity_for_override(runnable, ctx))
            .and_then(|o| o.cargo.as_ref())
        {
            if let Some(cmd) = &override_config.command {
                command_name = cmd.clone();
            }
            if let Some(sub) = &override_config.subcommand {
                subcommand = sub.clone();
            }
            if let Some(args) = &override_config.extra_args {
                extra_args.extend(args.clone());
            }
            if let Some(extra_env) = &override_config.extra_env {
                env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        Ok(CommandSpec {
            strategy: CommandStrategy::Shell,
            program: command_name,
            args: {
                let mut args = vec![subcommand];
                args.extend(extra_args);
                args
            },
            working_dir: command.working_dir,
            env,
            test_filter: command.test_filter,
            exec_args: command.exec_args,
            pipe_command: command.pipe_command,
            test_binary_args: command.test_binary_args,
        })
    }
}

impl crate::plugins::registry::OverlayPlugin for LeptosOverlayPlugin {
    fn id(&self) -> &'static str {
        "leptos"
    }

    fn default_priority(&self) -> i32 {
        150
    }

    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool {
        primary_id == "cargo"
            && ctx
                .manifest("Cargo.toml")
                .is_some_and(|cargo_toml| cargo_depends_on(cargo_toml, "leptos"))
            && target
                .runnable
                .as_ref()
                .is_some_and(|r| matches!(r.kind, RunnableKind::Binary { .. }))
    }

    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        if !matches!(runnable.kind, RunnableKind::Binary { .. }) {
            return Ok(command);
        }

        let override_config = ctx
            .config
            .get_override_for(&identity_for_override(runnable, ctx));

        // Custom program override (e.g. @spin.build --up) must win over cargo leptos.
        if let Some(ov) = override_config.and_then(|o| o.cargo.as_ref())
            && let Some(cmd) = &ov.command
            && cmd != "cargo"
        {
            return Ok(shell_command_from_cargo_override(command, ov, cmd));
        }

        let mut subcommand = "serve".to_string();
        let mut extra_args = Vec::new();
        let mut env = command.env.clone();

        if let Some(ov) = override_config.and_then(|o| o.cargo.as_ref()) {
            if let Some(sub) = &ov.subcommand {
                subcommand = if let Some(stripped) = sub.strip_prefix("leptos ") {
                    stripped.to_string()
                } else {
                    sub.clone()
                };
            }
            if let Some(args) = &ov.extra_args {
                extra_args.extend(args.clone());
            }
            if let Some(extra_env) = &ov.extra_env {
                env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        Ok(CommandSpec {
            strategy: CommandStrategy::Cargo,
            program: "cargo".to_string(),
            args: {
                let mut args = vec!["leptos".to_string(), subcommand];
                args.extend(extra_args);
                args
            },
            working_dir: command.working_dir,
            env,
            test_filter: command.test_filter,
            exec_args: command.exec_args,
            pipe_command: command.pipe_command,
            test_binary_args: command.test_binary_args,
        })
    }
}

impl crate::plugins::registry::OverlayPlugin for TauriOverlayPlugin {
    fn id(&self) -> &'static str {
        "tauri"
    }

    fn default_priority(&self) -> i32 {
        // Between Dioxus (200) and Leptos (150)
        175
    }

    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool {
        primary_id == "cargo"
            && is_tauri_project(ctx)
            && target
                .runnable
                .as_ref()
                .is_some_and(|r| matches!(r.kind, RunnableKind::Binary { .. }))
    }

    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        if !matches!(runnable.kind, RunnableKind::Binary { .. }) {
            return Ok(command);
        }

        // Default: cargo tauri dev (hot reload). Override with e.g. @cargo.tauri build
        // Custom program (e.g. @spin.serve) must win over cargo tauri.
        let override_cargo = ctx
            .config
            .get_override_for(&identity_for_override(runnable, ctx))
            .and_then(|o| o.cargo.as_ref());

        if let Some(ov) = override_cargo
            && let Some(cmd) = &ov.command
            && cmd != "cargo"
        {
            return Ok(shell_command_from_cargo_override(command, ov, cmd));
        }

        let mut subcommand = "dev".to_string();
        let mut extra_args = Vec::new();
        let mut env = command.env.clone();
        let mut channel: Option<String> = None;

        if let Some(ov) = override_cargo {
            if let Some(sub) = &ov.subcommand {
                // Accept "tauri dev", "tauri build", or bare "dev"/"build"
                subcommand = if let Some(stripped) = sub.strip_prefix("tauri ") {
                    stripped.to_string()
                } else {
                    sub.clone()
                };
            }
            if let Some(args) = &ov.extra_args {
                extra_args.extend(args.clone());
            }
            if let Some(extra_env) = &ov.extra_env {
                env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            if let Some(ch) = &ov.channel {
                channel = Some(ch.clone());
            }
        }

        let mut args = Vec::new();
        if let Some(ch) = channel {
            args.push(format!("+{ch}"));
        }
        args.push("tauri".to_string());
        args.extend(subcommand.split_whitespace().map(String::from));
        args.extend(extra_args);

        Ok(CommandSpec {
            strategy: CommandStrategy::Cargo,
            program: "cargo".to_string(),
            args,
            working_dir: command.working_dir,
            env,
            test_filter: command.test_filter,
            exec_args: command.exec_args,
            pipe_command: command.pipe_command,
            test_binary_args: command.test_binary_args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn cargo_depends_on_detects_table_and_simple_dep() {
        let tmp = TempDir::new().unwrap();
        let cargo = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo,
            r#"[package]
name = "app"
version = "0.1.0"

[dependencies]
leptos = "0.6"
serde = { version = "1", features = ["derive"] }
"#,
        )
        .unwrap();
        assert!(cargo_depends_on(&cargo, "leptos"));
        assert!(cargo_depends_on(&cargo, "serde"));
        assert!(!cargo_depends_on(&cargo, "tauri"));
    }

    #[test]
    fn cargo_depends_on_ignores_comment_mentions() {
        let tmp = TempDir::new().unwrap();
        let cargo = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo,
            r#"[package]
name = "app"
version = "0.1.0"

[dependencies]
# we used to use leptos here
serde = "1"
"#,
        )
        .unwrap();
        assert!(!cargo_depends_on(&cargo, "leptos"));
    }

    #[test]
    fn is_tauri_project_finds_conf_json() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname=\"a\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        fs::write(tmp.path().join("tauri.conf.json"), "{}\n").unwrap();
        let rs = tmp.path().join("src/main.rs");
        fs::create_dir_all(rs.parent().unwrap()).unwrap();
        fs::write(&rs, "fn main() {}\n").unwrap();

        let ctx =
            ProjectContext::from_path(&rs, std::sync::Arc::new(crate::config::Config::default()));
        assert!(is_tauri_project(&ctx));
    }
}
