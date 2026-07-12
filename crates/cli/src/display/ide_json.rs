//! Machine-readable JSON shapes for IDE integrations (VS Code, etc.).

use cargo_runner_core::{Command, Runnable};
use serde::Serialize;
use std::collections::BTreeMap;

/// Structured error for IDE JSON modes (`protocol_version` aligned with dry-run).
#[derive(Debug, Clone, Serialize)]
pub struct ErrorOutput {
    pub protocol_version: u32,
    pub error: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ErrorOutput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            protocol_version: 1,
            error: true,
            message: message.into(),
            code: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn from_anyhow(err: &anyhow::Error) -> Self {
        // Prefer the top-level message; chain is available via Debug if needed later.
        Self::new(format!("{err:#}"))
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            r#"{"protocol_version":1,"error":true,"message":"failed to serialize error"}"#
                .to_string()
        })
    }
}

/// Versioned envelope for dry-run command previews.
#[derive(Debug, Clone, Serialize)]
pub struct DryRunOutput {
    pub protocol_version: u32,
    pub program: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub env: BTreeMap<String, String>,
    pub shell: String,
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runnable: Option<Runnable>,
    /// Human-readable warnings for IDE UI (e.g. Bazel doc-test limitations).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl DryRunOutput {
    pub fn from_command(command: &Command, runnable: Option<Runnable>) -> Self {
        let mut warnings = Vec::new();

        // Promote known internal markers to structured warnings before stripping env.
        if let Some(msg) = command.env.get("_BAZEL_DOC_TEST_LIMITATION") {
            warnings.push(msg.clone());
        }

        let mut env: BTreeMap<String, String> = command
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        // Internal markers should not be treated as real process env by IDEs.
        env.retain(|k, _| !k.starts_with('_'));

        Self {
            protocol_version: 1,
            program: command.program.clone(),
            args: command.args.clone(),
            cwd: command
                .working_dir
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            env,
            shell: command.to_shell_command(),
            strategy: strategy_name(command.strategy).to_string(),
            runnable,
            warnings,
        }
    }
}

/// Runnable entry with optional resolved command for GUI previews.
#[derive(Debug, Clone, Serialize)]
pub struct RunnableEntry {
    #[serde(flatten)]
    pub runnable: Runnable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<CommandPreview>,
}

/// Compact command preview attached to a runnable listing.
#[derive(Debug, Clone, Serialize)]
pub struct CommandPreview {
    pub program: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub shell: String,
}

impl CommandPreview {
    pub fn from_command(command: &Command) -> Self {
        Self {
            program: command.program.clone(),
            args: command.args.clone(),
            cwd: command
                .working_dir
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            shell: command.to_shell_command(),
        }
    }
}

fn strategy_name(strategy: cargo_runner_core::CommandStrategy) -> &'static str {
    match strategy {
        cargo_runner_core::CommandStrategy::Cargo => "cargo",
        cargo_runner_core::CommandStrategy::CargoScript => "cargo_script",
        cargo_runner_core::CommandStrategy::Rustc => "rustc",
        cargo_runner_core::CommandStrategy::Shell => "shell",
        cargo_runner_core::CommandStrategy::Bazel => "bazel",
    }
}
