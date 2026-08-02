use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    process::{self, ExitStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStrategy {
    Cargo,
    CargoScript,
    Rustc,
    Shell,
    Bazel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub strategy: CommandStrategy,
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub test_filter: Option<String>,
    /// Rustc-specific: args to pass to the compiled binary during execution
    pub exec_args: Option<Vec<String>>,
    /// Rustc-specific: pipe output through this command
    pub pipe_command: Option<String>,
    /// Rustc-specific: extra args for test binary
    pub test_binary_args: Option<Vec<String>>,
    /// Rustc-specific: wrapper that runs the compiler, e.g.
    /// `["rustup", "run", "nightly"]`.
    ///
    /// Kept separate from `program`/`args` so a toolchain channel does not have
    /// to change the strategy. Rewriting the strategy to `Shell` to fit `rustup`
    /// in dropped every rustc-specific field below and skipped running the
    /// compiled binary entirely.
    pub compiler_prefix: Option<Vec<String>>,
}

/// Quote a value for `sh -c`.
///
/// Single quotes suppress every expansion; `'\''` closes, escapes a literal
/// quote, and reopens. Used both for the rustc pipe path — which really does
/// execute through a shell — and for rendered previews, so a copy-pasted
/// command means what it shows.
fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '=' | '+' | ','))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

impl Command {
    pub fn new(strategy: CommandStrategy, program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            strategy,
            program: program.into(),
            args,
            working_dir: None,
            env: HashMap::new(),
            test_filter: None,
            exec_args: None,
            pipe_command: None,
            test_binary_args: None,
            compiler_prefix: None,
        }
    }

    pub fn cargo(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Cargo, "cargo", args)
    }

    pub fn rustc(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Rustc, "rustc", args)
    }

    pub fn shell(program: impl Into<String>, args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Shell, program, args)
    }

    pub fn cargo_script(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::CargoScript, "cargo", args)
    }

    pub fn bazel(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Bazel, "bazel", args)
    }

    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_test_filter(mut self, filter: impl Into<String>) -> Self {
        self.test_filter = Some(filter.into());
        self
    }

    pub fn to_shell_command(&self) -> String {
        match self.strategy {
            CommandStrategy::Rustc => {
                let mut cmd = match self.compiler_prefix.as_deref() {
                    Some(prefix) if !prefix.is_empty() => {
                        format!("{} rustc", prefix.join(" "))
                    }
                    _ => String::from("rustc"),
                };
                for arg in &self.args {
                    cmd.push(' ');
                    cmd.push_str(&shell_quote(arg));
                }

                // Extract output name and append run command
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        let output = &self.args[i + 1];
                        // Check if output is an absolute path
                        let exec_path = if output.starts_with('/') || output.starts_with("./") {
                            output.to_string()
                        } else {
                            format!("./{output}")
                        };
                        cmd.push_str(&format!(" && {exec_path}"));

                        // If this is a test command with a filter, add it
                        if self.args.contains(&"--test".to_string()) {
                            // Check if we have exec phase args (like --bench)
                            if let Some(exec_args) = &self.exec_args {
                                // Add exec args BEFORE the test filter
                                for arg in exec_args {
                                    if arg != "{bench_name}" && arg != "{test_name}" {
                                        cmd.push_str(&format!(" {arg}"));
                                    }
                                }
                            }

                            if let Some(ref test_filter) = self.test_filter {
                                cmd.push_str(&format!(" {test_filter}"));
                            }

                            // Add extra test binary args if present
                            if let Some(extra_args) = &self.test_binary_args {
                                // No separator for test binaries - args are mixed with test names
                                for arg in extra_args {
                                    cmd.push_str(&format!(" {arg}"));
                                }
                            }
                        }

                        // Add pipe command if present
                        if let Some(pipe_cmd) = &self.pipe_command {
                            cmd.push_str(&format!(" | {pipe_cmd}"));
                        }

                        break;
                    }
                }
                cmd
            }
            CommandStrategy::Shell => {
                // For shell commands, first arg is the command itself
                let mut cmd = self.program.clone();
                for arg in &self.args {
                    cmd.push(' ');
                    cmd.push_str(&shell_quote(arg));
                }
                cmd
            }
            CommandStrategy::CargoScript | CommandStrategy::Cargo => {
                let mut cmd = String::from("cargo");
                for arg in &self.args {
                    cmd.push(' ');
                    cmd.push_str(&shell_quote(arg));
                }
                cmd
            }
            CommandStrategy::Bazel => {
                let mut cmd = String::from("bazel");
                for arg in &self.args {
                    cmd.push(' ');
                    cmd.push_str(&shell_quote(arg));
                }
                cmd
            }
        }
    }

    fn build_process(&self, program: &str, add_args: bool) -> process::Command {
        let mut cmd = process::Command::new(program);
        if add_args {
            cmd.args(&self.args);
        }
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd
    }

    /// Append exec-phase args to a string destined for `sh -c`.
    ///
    /// Every value is quoted. `test_filter` in particular is derived from test
    /// names in the repository's own source, so an unquoted splice here would
    /// let a test name inject shell syntax once the pipe has been approved.
    fn apply_test_args_to_shell_cmd(&self, shell_cmd: &mut String) {
        if self.args.contains(&"--test".to_string()) {
            if let Some(exec_args) = &self.exec_args {
                for arg in exec_args {
                    if arg != "{bench_name}" && arg != "{test_name}" {
                        shell_cmd.push_str(&format!(" {}", shell_quote(arg)));
                    }
                }
            }
            if let Some(ref test_filter) = self.test_filter {
                shell_cmd.push_str(&format!(" {}", shell_quote(test_filter)));
            }
            if let Some(extra_args) = &self.test_binary_args {
                for arg in extra_args {
                    shell_cmd.push_str(&format!(" {}", shell_quote(arg)));
                }
            }
        }
    }

    fn apply_test_args_to_run_cmd(&self, run_cmd: &mut process::Command) {
        if self.args.contains(&"--test".to_string()) {
            if let Some(exec_args) = &self.exec_args {
                for arg in exec_args {
                    if arg != "{bench_name}" && arg != "{test_name}" {
                        run_cmd.arg(arg);
                    }
                }
            }
            if let Some(ref test_filter) = self.test_filter {
                run_cmd.arg(test_filter);
            }
            if let Some(extra_args) = &self.test_binary_args {
                for arg in extra_args {
                    run_cmd.arg(arg);
                }
            }
        }
    }

    /// Program this command will actually launch.
    ///
    /// The strategy decides whether `program` or a fixed tool name is used, so
    /// the trust check has to ask rather than read `self.program` directly.
    pub fn resolved_program(&self) -> &str {
        match self.strategy {
            CommandStrategy::Rustc => "rustc",
            CommandStrategy::Shell => &self.program,
            CommandStrategy::CargoScript | CommandStrategy::Cargo => "cargo",
            CommandStrategy::Bazel => "bazel",
        }
    }

    /// Fail unless this command is either innocuous or already approved.
    ///
    /// This is the last line of defence, not the place users are asked: the
    /// CLI prompts before getting here. Keeping the check inside `execute`
    /// means any future call site inherits it instead of quietly bypassing it.
    fn authorize(&self) -> io::Result<()> {
        if crate::trust::bypass_requested() {
            return Ok(());
        }
        let program = self.resolved_program();
        let pipe = self.pipe_command.as_deref();
        let crate::trust::Verdict::NeedsConsent(reasons) =
            crate::trust::evaluate(program, &self.env, pipe)
        else {
            return Ok(());
        };

        let approval =
            crate::trust::approval_for(self.working_dir.as_deref(), program, &self.env, pipe);
        if crate::trust::approved_in_process(&approval)
            || crate::trust::TrustStore::load().contains(&approval)
        {
            return Ok(());
        }

        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to run a command this project configured but you have not approved:\n  {}\n\
                 Approve it with `cargo runner trust` (or re-run with --trust-config), \
                 or set CARGO_RUNNER_TRUST=1 in trusted automation.",
                reasons.join("\n  ")
            ),
        ))
    }

    pub fn execute(&self) -> io::Result<ExitStatus> {
        self.authorize()?;
        match self.strategy {
            CommandStrategy::Rustc => {
                let mut output_name = None;
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        output_name = Some(&self.args[i + 1]);
                        break;
                    }
                }

                // With a toolchain prefix the compile step becomes
                // `rustup run <channel> rustc <args>`; without one it is plain
                // `rustc <args>`. Either way the run-the-output tail below is
                // unchanged, which is the whole point of not rewriting the
                // strategy.
                let mut rustc_cmd = match self.compiler_prefix.as_deref() {
                    Some([head, rest @ ..]) => {
                        let mut c = self.build_process(head, false);
                        c.args(rest);
                        c.arg("rustc");
                        c.args(&self.args);
                        c
                    }
                    _ => self.build_process("rustc", true),
                };
                let compile_status = rustc_cmd.status()?;
                if !compile_status.success() {
                    return Ok(compile_status);
                }

                if let Some(output) = output_name {
                    let exec_path = if output.starts_with('/') || output.starts_with("./") {
                        output.to_string()
                    } else {
                        format!("./{output}")
                    };

                    let mut run_cmd = if self.pipe_command.is_some() {
                        let mut cmd = self.build_process("sh", false);
                        cmd.arg("-c");
                        cmd
                    } else {
                        self.build_process(&exec_path, false)
                    };

                    if let Some(pipe_to) = &self.pipe_command {
                        let mut shell_cmd = exec_path;
                        self.apply_test_args_to_shell_cmd(&mut shell_cmd);
                        // The pipe is a shell fragment by design and consented to as
                        // such, so it is not quoted — everything spliced around it is.
                        shell_cmd.push_str(&format!(" | {pipe_to}"));
                        run_cmd.arg(shell_cmd);
                    } else {
                        self.apply_test_args_to_run_cmd(&mut run_cmd);
                    }

                    run_cmd.status()
                } else {
                    Ok(compile_status)
                }
            }
            CommandStrategy::Shell => self.build_process(&self.program, true).status(),
            CommandStrategy::CargoScript | CommandStrategy::Cargo => {
                self.build_process("cargo", true).status()
            }
            CommandStrategy::Bazel => self.build_process("bazel", true).status(),
        }
    }
}

#[cfg(test)]
mod shell_quote_tests {
    use super::*;

    #[test]
    fn ordinary_values_are_left_bare() {
        for v in ["cargo", "--release", "src/lib.rs", "a::b::c", "KEY=value", "1.2.3"] {
            assert_eq!(shell_quote(v), v, "{v} should not need quoting");
        }
    }

    #[test]
    fn shell_metacharacters_are_neutralised() {
        // test_filter comes from names in the repository's own source, so this
        // is the case that matters once a pipe has been approved.
        assert_eq!(shell_quote("a;id"), "'a;id'");
        assert_eq!(shell_quote("$(id)"), "'$(id)'");
        assert_eq!(shell_quote("`id`"), "'`id`'");
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn embedded_single_quotes_cannot_close_the_quoting() {
        // The bug class that defeated two earlier attempts on this branch: a
        // value containing a quote must not be able to end its own quoted span.
        // Verified against a real `sh`, which round-trips this back to the
        // literal input without executing `id`.
        let quoted = shell_quote("x';id;'");
        assert_eq!(quoted, r#"'x'\'';id;'\'''"#);

        // Every `'` in the payload is either the opening/closing wrapper or
        // part of an escape sequence — none of them leaves quoting open.
        assert!(quoted.starts_with('\'') && quoted.ends_with('\''));
    }
}

#[cfg(test)]
mod compiler_prefix_tests {
    use super::*;

    fn rustc_build() -> Command {
        Command::rustc(vec![
            "--test".to_string(),
            "/tmp/x.rs".to_string(),
            "-o".to_string(),
            "/tmp/x_test".to_string(),
        ])
    }

    #[test]
    fn without_a_prefix_the_rendering_is_unchanged() {
        let cmd = rustc_build();
        let shell = cmd.to_shell_command();
        assert!(shell.starts_with("rustc --test"), "got: {shell}");
        assert!(shell.contains("&& /tmp/x_test"), "got: {shell}");
    }

    #[test]
    fn a_prefix_wraps_only_the_compile_step() {
        let mut cmd = rustc_build();
        cmd.compiler_prefix = Some(vec![
            "rustup".to_string(),
            "run".to_string(),
            "nightly".to_string(),
        ]);
        let shell = cmd.to_shell_command();

        assert!(
            shell.starts_with("rustup run nightly rustc --test"),
            "compile step should be wrapped: {shell}"
        );
        // The compiled binary is still executed — the bug this replaced dropped
        // this tail entirely by rewriting the strategy to Shell.
        assert!(
            shell.contains("&& /tmp/x_test"),
            "the output binary must still run: {shell}"
        );
    }

    #[test]
    fn a_prefix_does_not_change_the_strategy_or_resolved_program() {
        let mut cmd = rustc_build();
        cmd.compiler_prefix = Some(vec!["rustup".into(), "run".into(), "nightly".into()]);

        // Strategy must stay Rustc: RustcRunner::validate_command rejects
        // anything else, which is what made `+nightly` a hard error.
        assert_eq!(cmd.strategy, CommandStrategy::Rustc);
        // And the trust gate should still see rustc, not rustup (which is not
        // in KNOWN_PROGRAMS and would prompt on every nightly run).
        assert_eq!(cmd.resolved_program(), "rustc");
    }

    #[test]
    fn a_prefix_preserves_the_rustc_specific_fields() {
        let mut cmd = rustc_build();
        cmd.compiler_prefix = Some(vec!["rustup".into(), "run".into(), "nightly".into()]);
        cmd.test_filter = Some("mod::it_works".to_string());
        cmd.exec_args = Some(vec!["--nocapture".to_string()]);
        cmd.test_binary_args = Some(vec!["--ignored".to_string()]);

        let shell = cmd.to_shell_command();
        for expected in ["--nocapture", "mod::it_works", "--ignored"] {
            assert!(
                shell.contains(expected),
                "{expected} should survive: {shell}"
            );
        }
    }
}
