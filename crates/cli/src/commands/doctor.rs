//! `cargo runner doctor` — diagnose project + toolchain health.

use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::config::bazel_workspace::{find_cargo_workspace_root, find_module_bazel};
use crate::display::style;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub protocol_version: u32,
    pub checks: Vec<DoctorCheck>,
    pub ok: bool,
}

pub fn doctor_command(json: bool) -> Result<()> {
    if json {
        style::set_json_error_mode(true);
    }
    let report = run_checks()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human(&report);
    }
    if !report.ok {
        anyhow::bail!("doctor found failing checks");
    }
    Ok(())
}

fn run_checks() -> Result<DoctorReport> {
    let cwd = std::env::current_dir()?;
    let mut checks = Vec::new();

    // cwd
    checks.push(DoctorCheck {
        id: "cwd".into(),
        status: CheckStatus::Pass,
        message: format!("cwd: {}", cwd.display()),
    });

    // Cargo workspace
    if let Some(root) = find_cargo_workspace_root(&cwd) {
        checks.push(DoctorCheck {
            id: "cargo_workspace".into(),
            status: CheckStatus::Pass,
            message: format!("Cargo workspace root: {}", root.display()),
        });
        let cfg = root.join(".cargo-runner.json");
        if cfg.exists() {
            checks.push(DoctorCheck {
                id: "cargo_runner_config".into(),
                status: CheckStatus::Pass,
                message: format!("found {}", cfg.display()),
            });
        } else {
            checks.push(DoctorCheck {
                id: "cargo_runner_config".into(),
                status: CheckStatus::Warn,
                message: "no .cargo-runner.json (run `cargo runner init` if desired)".into(),
            });
        }
        // Framework hints
        if let Ok(toml) = std::fs::read_to_string(root.join("Cargo.toml")) {
            let mut frameworks = Vec::new();
            if toml.contains("dioxus") {
                frameworks.push("dioxus");
            }
            if toml.contains("leptos") {
                frameworks.push("leptos");
            }
            if toml.contains("tauri") {
                frameworks.push("tauri");
            }
            if !frameworks.is_empty() {
                checks.push(DoctorCheck {
                    id: "frameworks".into(),
                    status: CheckStatus::Pass,
                    message: format!("detected: {}", frameworks.join(", ")),
                });
            }
        }
    } else {
        checks.push(DoctorCheck {
            id: "cargo_workspace".into(),
            status: CheckStatus::Warn,
            message: "no Cargo.toml workspace found from cwd".into(),
        });
    }

    // cargo / rustc
    checks.push(tool_version("cargo", &["--version"]));
    checks.push(tool_version("rustc", &["--version"]));

    // Bazel
    if let Some(bazel_root) = find_module_bazel(&cwd) {
        checks.push(DoctorCheck {
            id: "bazel_workspace".into(),
            status: CheckStatus::Pass,
            message: format!("MODULE.bazel at {}", bazel_root.display()),
        });
        checks.push(tool_version("bazel", &["version"]));
        let rpj = bazel_root.join("rust-project.json");
        if rpj.exists() {
            checks.push(DoctorCheck {
                id: "rust_project_json".into(),
                status: CheckStatus::Pass,
                message: "rust-project.json present".into(),
            });
        } else {
            checks.push(DoctorCheck {
                id: "rust_project_json".into(),
                status: CheckStatus::Warn,
                message: "rust-project.json missing (IDE may lack analysis)".into(),
            });
        }
    } else {
        checks.push(DoctorCheck {
            id: "bazel_workspace".into(),
            status: CheckStatus::Skip,
            message: "not a Bazel workspace".into(),
        });
    }

    // nextest optional
    if nextest_ok() {
        checks.push(DoctorCheck {
            id: "nextest".into(),
            status: CheckStatus::Pass,
            message: "cargo nextest available".into(),
        });
    } else {
        checks.push(DoctorCheck {
            id: "nextest".into(),
            status: CheckStatus::Skip,
            message: "cargo nextest not installed".into(),
        });
    }

    // CLI version
    checks.push(DoctorCheck {
        id: "cli_version".into(),
        status: CheckStatus::Pass,
        message: format!("cargo-runner {}", env!("CARGO_PKG_VERSION")),
    });

    let ok = !checks.iter().any(|c| matches!(c.status, CheckStatus::Fail));
    Ok(DoctorReport {
        protocol_version: 1,
        checks,
        ok,
    })
}

fn tool_version(bin: &str, args: &[&str]) -> DoctorCheck {
    match Command::new(bin).args(args).output() {
        Ok(o) if o.status.success() => {
            let msg = String::from_utf8_lossy(&o.stdout);
            let first = msg.lines().next().unwrap_or("ok").trim();
            DoctorCheck {
                id: bin.into(),
                status: CheckStatus::Pass,
                message: first.to_string(),
            }
        }
        Ok(_) | Err(_) => DoctorCheck {
            id: bin.into(),
            status: CheckStatus::Fail,
            message: format!("`{bin}` not available or failed"),
        },
    }
}

fn nextest_ok() -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn print_human(report: &DoctorReport) {
    println!(
        "{}",
        style::banner(
            "🩺",
            &format!("cargo-runner doctor (v{})", env!("CARGO_PKG_VERSION"))
        )
    );
    for c in &report.checks {
        let mark = match c.status {
            CheckStatus::Pass => style::icon("✅"),
            CheckStatus::Warn => style::icon("⚠️"),
            CheckStatus::Fail => style::icon("❌"),
            CheckStatus::Skip => style::icon("⏭️"),
        };
        let status = format!("{:?}", c.status).to_lowercase();
        if mark.is_empty() {
            println!("[{status}] {}: {}", c.id, c.message);
        } else {
            println!("{mark} [{status}] {}: {}", c.id, c.message);
        }
    }
    if report.ok {
        println!("{}", style::banner("✅", "All critical checks passed"));
    } else {
        println!("{}", style::banner("❌", "Some checks failed"));
    }
    let _ = Path::new("."); // silence unused if any
}
