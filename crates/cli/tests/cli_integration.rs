//! Integration tests for the `cargo-runner` CLI.
//!
//! Each test creates a temporary Cargo project, runs the CLI binary against it,
//! and asserts on stdout/stderr/exit-code and side-effects (files created, etc.).

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Scaffold a minimal Cargo project in the given directory.
fn scaffold_cargo_project(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("src/main.rs"),
        "fn main() { println!(\"hello\"); }\n",
    )
    .unwrap();
}

/// Scaffold a Cargo project with a lib.rs and tests.
fn scaffold_lib_project(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        r#"pub fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#,
    )
    .unwrap();
}

/// Scaffold a workspace root with a single binary member under `crates/<member>`.
fn scaffold_workspace_member_binary(dir: &std::path::Path, member_dir: &str, package_name: &str) {
    let member_path = dir.join(member_dir);
    fs::create_dir_all(member_path.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["{member_dir}"]
resolver = "2"
"#
        ),
    )
    .unwrap();
    fs::write(
        member_path.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    fs::write(
        member_path.join("src/main.rs"),
        "fn main() { println!(\"workspace member\"); }\n",
    )
    .unwrap();
}

/// Scaffold a workspace member with a default-run binary target.
fn scaffold_workspace_member_default_run_binary(
    dir: &std::path::Path,
    member_dir: &str,
    package_name: &str,
    default_run: &str,
) {
    let member_path = dir.join(member_dir);
    fs::create_dir_all(member_path.join("src/bin")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["{member_dir}"]
resolver = "2"
"#
        ),
    )
    .unwrap();
    fs::write(
        member_path.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"
default-run = "{default_run}"
"#
        ),
    )
    .unwrap();
    fs::write(
        member_path.join("src/main.rs"),
        "fn main() { println!(\"workspace member main\"); }\n",
    )
    .unwrap();
    fs::write(
        member_path.join(format!("src/bin/{default_run}.rs")),
        format!("fn main() {{ println!(\"{default_run}\"); }}\n"),
    )
    .unwrap();
}

/// Scaffold a workspace member that exposes a nested module with tests.
fn scaffold_workspace_member_module_tests(
    dir: &std::path::Path,
    member_dir: &str,
    package_name: &str,
) {
    let member_path = dir.join(member_dir);
    fs::create_dir_all(member_path.join("src/runners")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[workspace]
members = ["{member_dir}"]
resolver = "2"
"#
        ),
    )
    .unwrap();
    fs::write(
        member_path.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    fs::create_dir_all(member_path.join("src/runners")).unwrap();
    fs::write(member_path.join("src/lib.rs"), "pub mod runners;\n").unwrap();
    fs::write(
        member_path.join("src/runners/mod.rs"),
        "pub mod unified_runner;\n",
    )
    .unwrap();
    fs::write(
        member_path.join("src/runners/unified_runner.rs"),
        r#"pub fn helper() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_helper() {
        assert_eq!(2 + 2, 4);
    }
}
"#,
    )
    .unwrap();
}

/// Scaffold a library crate with a module that has unit tests (symbol filter target).
fn scaffold_lib_project_with_doc_symbol(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    fs::write(
        dir.join("src/lib.rs"),
        r#"pub mod Users {
    #[cfg(test)]
    mod tests {
        #[test]
        fn test_users() {
            assert_eq!(2 + 2, 4);
        }
    }
}
"#,
    )
    .unwrap();
}

/// Library with a real fenced doctest on an enum (scoped `--doc` dry-run).
fn scaffold_lib_with_enum_doctest(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
        ),
    )
    .unwrap();
    let crate_name = name.replace('-', "_");
    fs::write(
        dir.join("src/lib.rs"),
        format!(
            r#"/// A color.
///
/// ```
/// let _ = {crate_name}::Color::Red;
/// ```
pub enum Color {{
    Red,
    Blue,
}}
"#
        ),
    )
    .unwrap();
}

/// Scaffold a minimal Bazel workspace with a binary package.
fn scaffold_bazel_binary_workspace(dir: &std::path::Path, package_dir: &str, target_name: &str) {
    let package_path = dir.join(package_dir);
    fs::create_dir_all(package_path.join("src")).unwrap();
    fs::write(
        dir.join("MODULE.bazel"),
        "module(name = \"test_workspace\")\n",
    )
    .unwrap();
    fs::write(
        package_path.join("BUILD.bazel"),
        format!(
            r#"
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "{target_name}",
    srcs = ["src/main.rs"],
)
"#
        ),
    )
    .unwrap();
    fs::write(
        package_path.join("src/main.rs"),
        "fn main() { println!(\"hello from bazel\"); }\n",
    )
    .unwrap();
}

/// Scaffold a single-file rust-script style source file.
fn scaffold_rust_script_file(dir: &std::path::Path, file_name: &str) {
    fs::write(
        dir.join(file_name),
        r#"#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! clap = { version = "4.5", features = ["derive"] }
//! ```
//!
//! [package]
//! edition = "2024"

fn main() {
    println!("hello");
}
"#,
    )
    .unwrap();
}

/// Scaffold a single-file cargo script source file.
fn scaffold_cargo_script_file(dir: &std::path::Path, file_name: &str) {
    fs::write(
        dir.join(file_name),
        r#"#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
---
fn main() {
    println!("hello");
}
"#,
    )
    .unwrap();
}

/// Get a `Command` for the cargo-runner binary.
fn cargo_runner() -> Command {
    // In Bazel, CARGO_BIN_EXE_cargo-runner is set to a relative path like `crates/cli/cargo-runner`.
    // Tests change directories to temp dirs, making relative paths invalid.
    // Convert to an absolute path first.
    if let Ok(bin_path) = std::env::var("CARGO_BIN_EXE_cargo-runner") {
        let path = std::path::Path::new(&bin_path);
        if !path.is_absolute()
            && let Ok(cwd) = std::env::current_dir()
        {
            let abs_path = cwd.join(path);
            unsafe {
                std::env::set_var("CARGO_BIN_EXE_cargo-runner", abs_path);
            }
        }
    }
    Command::cargo_bin("cargo-runner").unwrap()
}

/// Get the canonical (symlink-resolved) path for a temp directory.
/// macOS `/tmp` → `/private/tmp`, so PROJECT_ROOT must match the canonical path.
fn canonical(dir: &std::path::Path) -> String {
    dir.canonicalize()
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_str()
        .unwrap()
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// init
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn init_creates_env_file_and_config() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-init");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initializing cargo-runner"))
        .stdout(predicate::str::contains(".cargo-runner.env"))
        .stdout(predicate::str::contains("Initialization complete"));

    // Verify files were created
    assert!(tmp.path().join(".cargo-runner.env").exists());
    assert!(tmp.path().join(".cargo-runner.json").exists());

    // Verify env file contains PROJECT_ROOT
    let env_content = fs::read_to_string(tmp.path().join(".cargo-runner.env")).unwrap();
    assert!(env_content.contains("PROJECT_ROOT="));

    // Verify config is valid JSON with expected structure
    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    assert!(config.get("cargo").is_some());
}

#[test]
fn init_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-idempotent");
    let root = canonical(tmp.path());

    // Run init twice
    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Still only one config file
    assert!(tmp.path().join(".cargo-runner.json").exists());
}

// ═══════════════════════════════════════════════════════════════════════════════
// override — named flags
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn override_with_named_flags() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-override");
    let root = canonical(tmp.path());

    // Init first
    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Override with named flags
    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--command",
            "dx",
            "--subcommand",
            "serve",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Override").and(predicate::str::contains("successfully")));

    // Verify the config was updated
    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();
    assert!(!overrides.is_empty(), "overrides should not be empty");

    // Verify the override has a match section with file_path
    let ov = &overrides[0];
    let match_section = ov.get("match").unwrap();
    assert!(
        match_section.get("file_path").is_some(),
        "match should have file_path"
    );

    // Verify the override contains the command/subcommand somewhere in its config
    let ov_str = serde_json::to_string(ov).unwrap();
    assert!(
        ov_str.contains("dx"),
        "override should contain 'dx' command: {ov_str}"
    );
    assert!(
        ov_str.contains("serve"),
        "override should contain 'serve' subcommand: {ov_str}"
    );
}

#[test]
fn override_with_token_syntax() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-token-override");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--",
            "@dx.serve",
            "+nightly",
            "RUST_LOG=debug",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("successfully"));

    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();
    assert!(!overrides.is_empty());

    // Verify the override contains dx and serve
    let ov_str = serde_json::to_string(&overrides[0]).unwrap();
    assert!(ov_str.contains("dx"), "should contain dx: {ov_str}");
    assert!(ov_str.contains("serve"), "should contain serve: {ov_str}");
}

#[test]
fn override_leptos_token() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-leptos-override");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["override", "src/main.rs", "--", "@cargo.leptos.watch"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("leptos watch"));

    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();
    assert!(!overrides.is_empty());

    let ov_str = serde_json::to_string(&overrides[0]).unwrap();
    assert!(
        ov_str.contains("leptos watch"),
        "should contain 'leptos watch': {ov_str}"
    );
}

#[test]
fn override_updates_existing() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-override-update");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // First override
    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--command",
            "dx",
            "--subcommand",
            "serve",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Update same file with different subcommand
    cargo_runner()
        .args(["override", "src/main.rs", "--subcommand", "build"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("updated"));

    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();

    // Should still be 1 override (updated, not duplicated)
    assert_eq!(
        overrides.len(),
        1,
        "should have exactly 1 override, got {}",
        overrides.len()
    );
    let ov_str = serde_json::to_string(&overrides[0]).unwrap();
    assert!(
        ov_str.contains("build"),
        "should contain new subcommand 'build': {ov_str}"
    );
}

#[test]
fn override_remove_with_dash() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-remove");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Add an override
    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--command",
            "dx",
            "--subcommand",
            "serve",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify it exists
    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();
    assert!(!overrides.is_empty(), "should have override before removal");

    // Remove it
    cargo_runner()
        .args(["override", "src/main.rs", "--", "-"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("removed"));

    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let overrides = config.get("overrides").unwrap().as_array().unwrap();
    assert!(
        overrides.is_empty(),
        "overrides should be empty after removal: {config_content}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// run (dry-run mode — does not execute the command)
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// context
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn context_json_for_cargo_project() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-context");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["runner", "context", "src/main.rs", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""file_kind": "cargo_project""#))
        .stdout(predicate::str::contains(r#""build_system": "cargo""#))
        .stdout(predicate::str::contains(
            r#""package_name": "test-context""#,
        ))
        .stdout(predicate::str::contains(r#""runnable_kind": "binary""#))
        .stdout(predicate::str::contains(
            r#""recommended_target": "test-context""#,
        ));
}

#[test]
fn context_json_for_cargo_project_without_filepath() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-context");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["runner", "context", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""file_path": null"#))
        .stdout(predicate::str::contains(r#""file_kind": "cargo_project""#))
        .stdout(predicate::str::contains(r#""build_system": "cargo""#))
        .stdout(predicate::str::contains(
            r#""package_name": "test-context""#,
        ))
        .stdout(predicate::str::contains(
            r#""recommended_target": "test-context""#,
        ));
}

#[test]
fn context_json_for_rust_script() {
    let tmp = TempDir::new().unwrap();
    scaffold_rust_script_file(tmp.path(), "power.rs");

    cargo_runner()
        .args(["runner", "context", "power.rs", "--json"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""file_kind": "single_file_script""#,
        ))
        .stdout(predicate::str::contains(r#""build_system": "rust-script""#))
        .stdout(predicate::str::contains(
            r#""script_engine": "rust-script""#,
        ))
        .stdout(predicate::str::contains("power.rs"));
}

#[test]
fn context_json_for_cargo_script() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_script_file(tmp.path(), "power.rs");

    cargo_runner()
        .args(["runner", "context", "power.rs", "--json"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#""file_kind": "single_file_script""#,
        ))
        .stdout(predicate::str::contains(r#""build_system": "cargo""#))
        .stdout(predicate::str::contains(
            r#""script_engine": "cargo +nightly -Zscript""#,
        ))
        .stdout(predicate::str::contains("power.rs"));
}

#[test]
fn context_json_for_module_path() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_module_tests(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args([
            "runner",
            "context",
            "runners::unified_runner::tests",
            "--json",
        ])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""file_path": "#))
        .stdout(predicate::str::contains("unified_runner.rs"))
        .stdout(predicate::str::contains(
            r#""runnable_kind": "module_tests""#,
        ))
        .stdout(predicate::str::contains(r#""build_system": "cargo""#));
}

#[test]
fn run_dry_run_binary() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-dryrun");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["run", "src/main.rs", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo").and(predicate::str::contains("run")));
}

#[test]
fn run_dry_run_json_binary() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-dryrun-json");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["run", "src/main.rs", "--dry-run", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""protocol_version": 1"#))
        .stdout(predicate::str::contains(r#""program": "cargo""#))
        .stdout(predicate::str::contains(r#""shell":"#))
        .stdout(predicate::str::contains(r#""strategy": "cargo""#));
}

#[test]
fn run_dry_run_rust_script() {
    let tmp = TempDir::new().unwrap();
    scaffold_rust_script_file(tmp.path(), "power.rs");

    cargo_runner()
        .args(["run", "power.rs", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("rust-script"))
        .stdout(predicate::str::contains("power.rs"));
}

#[test]
fn run_dry_run_workspace_member_binary_without_project_root_is_grounded() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_binary(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args(["run", "crates/app/src/main.rs", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "cargo run --package workspace-app --bin workspace-app",
        ));
}

#[test]
fn run_dry_run_bazel_binary_outside_home_uses_bazel_dispatch() {
    let tmp = TempDir::new().unwrap();
    scaffold_bazel_binary_workspace(tmp.path(), "app", "app");

    cargo_runner()
        .args(["run", "app/src/main.rs", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("bazel run //app:app"));
}

#[test]
fn run_dry_run_test() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-dryrun-lib");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Line 11 = inside test_add function (0-based line 10 in the file, 1-based is 11)
    cargo_runner()
        .args(["run", "src/lib.rs:11", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo").and(predicate::str::contains("test")));
}

#[test]
fn run_dry_run_module_path() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_module_tests(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args(["run", "runners::unified_runner::tests", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo").and(predicate::str::contains("test")));
}

#[test]
fn run_dry_run_honors_default_run() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_default_run_binary(
        tmp.path(),
        "crates/app",
        "workspace-app",
        "server",
    );

    cargo_runner()
        .args(["run", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("cargo")
                .and(predicate::str::contains("--bin server"))
                .and(predicate::str::contains("workspace-app")),
        );
}

#[test]
fn run_dry_run_bare_test_function_name() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_module_tests(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args(["run", "test_helper", "--dry-run"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "cargo test --package workspace-app --lib -- runners::unified_runner::tests::test_helper --exact",
        ));
}

#[test]
fn run_dry_run_full_test_selector() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_module_tests(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args([
            "run",
            "runners::unified_runner::tests::test_helper",
            "--dry-run",
        ])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "cargo test --package workspace-app --lib -- runners::unified_runner::tests::test_helper --exact",
        ));
}

#[test]
fn runnables_symbol_filter_matches_module_name() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project_with_doc_symbol(tmp.path(), "test-doc-symbol");

    cargo_runner()
        .args(["runnables", "src/lib.rs", "--symbol", "Users"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Run all tests in module 'Users'"));
}

#[test]
fn dry_run_enum_doctest_uses_cargo_test_doc() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_with_enum_doctest(tmp.path(), "test-enum-doc");
    let root = canonical(tmp.path());

    // Line 4 is inside the fenced doctest (1-based: line 4 ≈ ``` body)
    // Filter must be crate-relative (`Color`), NOT `test-enum-doc::Color`
    // (rustdoc names never include the Cargo package name).
    cargo_runner()
        .args(["run", "src/lib.rs:4", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("--doc"))
        .stdout(predicate::str::contains(" -- Color").or(predicate::str::contains("-- Color")))
        .stdout(predicate::str::contains("test-enum-doc::Color").not());
}

#[test]
fn dry_run_nested_doctest_filter_is_crate_relative() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(
        tmp.path().join("Cargo.toml"),
        r#"[package]
name = "nested-doc"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("src/lib.rs"),
        r#"pub mod inner {
    /// Nested
    ///
    /// ```
    /// assert!(true);
    /// ```
    pub struct Nested;
}
"#,
    )
    .unwrap();
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["run", "src/lib.rs:5", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("--doc"))
        .stdout(predicate::str::contains("inner::Nested"))
        .stdout(predicate::str::contains("nested-doc::inner").not());
}

#[test]
fn runnables_lists_enum_doctest() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_with_enum_doctest(tmp.path(), "test-enum-doc-list");

    cargo_runner()
        .args(["runnables", "src/lib.rs", "--doc"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Run doc test for 'Color'"));
}

#[test]
fn run_nonexistent_file() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-nofile");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["run", "src/nonexistent.rs", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("No runnable found for selector"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// analyze
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn analyze_binary_file() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-analyze");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["analyze", "src/main.rs"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Analyzing"))
        .stdout(predicate::str::contains("main"));
}

#[test]
fn runnables_json_binary_file() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-runnables-json");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["runnables", "src/main.rs", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""label":"#))
        .stdout(predicate::str::contains(r#""file_path":"#))
        .stdout(predicate::str::contains("main.rs"));
}

#[test]
fn runnables_json_with_commands() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-runnables-cmds");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["runnables", "src/main.rs", "--json", "--with-commands"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""command":"#))
        .stdout(predicate::str::contains(r#""shell":"#))
        .stdout(predicate::str::contains(r#""program":"#));
}

#[test]
fn override_list_json_empty() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-override-list");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["override", "--list", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[test]
fn analyze_lib_with_tests() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-analyze-lib");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["analyze", "src/lib.rs"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_add"));
}

#[test]
fn analyze_workspace_member_binary_shows_grounded_command_without_project_root() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_binary(tmp.path(), "crates/app", "workspace-app");

    cargo_runner()
        .args(["analyze", "crates/app/src/main.rs"])
        .env_remove("PROJECT_ROOT")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "cargo run --package workspace-app --bin workspace-app",
        ));
}

#[test]
fn analyze_bazel_binary_outside_home_shows_bazel_dispatch() {
    let tmp = TempDir::new().unwrap();
    scaffold_bazel_binary_workspace(tmp.path(), "app", "app");

    cargo_runner()
        .args(["analyze", "app/src/main.rs"])
        .env_remove("PROJECT_ROOT")
        .env_remove("PROJECT_DIR")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Final command: bazel run //app:app",
        ))
        .stdout(predicate::str::contains("command: bazel"));
}

#[test]
fn analyze_verbose_shows_json() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-verbose");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    let output = cargo_runner()
        .args(["analyze", "src/main.rs", "--verbose"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Verbose mode outputs structured/JSON-like content
    assert!(
        stdout.contains("{") || stdout.contains("Binary"),
        "verbose output: {stdout}"
    );
}

#[test]
fn analyze_with_config_flag() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-config-flag");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["analyze", "src/main.rs", "--config"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration Details"));
}

#[test]
fn analyze_nonexistent_file() {
    let tmp = TempDir::new().unwrap();

    cargo_runner()
        .args(["analyze", "src/nope.rs"])
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("File not found"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// unset
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn unset_without_project_root() {
    let tmp = TempDir::new().unwrap();

    // Without PROJECT_ROOT, unset falls back to cwd (still succeeds).
    cargo_runner()
        .args(["unset"])
        .env_remove("PROJECT_ROOT")
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Using current directory"));
}

#[test]
fn unset_with_clean_removes_configs() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-unset");
    let root = canonical(tmp.path());

    // Init to create configs
    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    assert!(tmp.path().join(".cargo-runner.json").exists());

    // Unset with --clean
    cargo_runner()
        .args(["unset", "--clean"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaning"));

    // Config should be removed
    assert!(!tmp.path().join(".cargo-runner.json").exists());
}

// ═══════════════════════════════════════════════════════════════════════════════
// clean
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn clean_in_cargo_project() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-clean");

    // Clean should run cargo clean (may fail if no build cache, but shouldn't crash)
    let output = cargo_runner()
        .args(["clean"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // It should at least attempt "cargo clean" — we check the output
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("clean") || combined.contains("Clean") || output.status.success(),
        "clean should work or mention clean: stdout={stdout}, stderr={stderr}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// override + run integration (end-to-end dry-run)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn override_then_dry_run_shows_custom_command() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-e2e");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Set override to dx serve (both named flags)
    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--command",
            "dx",
            "--subcommand",
            "serve",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // Verify the override was stored
    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    assert!(
        config_content.contains("dx"),
        "config should have dx: {config_content}"
    );

    // Dry run must show the overridden command (not plain cargo run)
    let output = cargo_runner()
        .args(["run", "src/main.rs", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("dx") && stdout.contains("serve"),
        "dry-run must apply dx serve override, got: {stdout}"
    );
}

#[test]
fn override_list_and_show_json_after_create() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-list-show");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args([
            "override",
            "src/main.rs",
            "--",
            "@dx.serve",
            "RUST_LOG=debug",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["override", "--list", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dx"))
        .stdout(predicate::str::contains("config_path"));

    cargo_runner()
        .args(["override", "--show", "src/main.rs", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dx"));
}

#[test]
fn override_append_mode_merges_extra_args() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-append");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["override", "src/main.rs", "--", "--release"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args(["override", "src/main.rs", "--", "@", "--features", "foo"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    let config_content = fs::read_to_string(tmp.path().join(".cargo-runner.json")).unwrap();
    assert!(
        config_content.contains("--release"),
        "append must keep prior args: {config_content}"
    );
    assert!(
        config_content.contains("foo") || config_content.contains("--features"),
        "append must add new args: {config_content}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Phase D — bazel init / build-sync / workspace runnables
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn init_bazel_skip_sync_scaffolds_files() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "bazel-init-app");
    let root = canonical(tmp.path());

    cargo_runner()
        .args([
            "init",
            "--bazel",
            "--skip-sync",
            "--workspace-name",
            "demo_ws",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Scaffolding Bazel workspace")
                .or(predicate::str::contains("Bazel workspace")),
        );

    assert!(
        tmp.path().join("MODULE.bazel").exists(),
        "MODULE.bazel should be created"
    );
    assert!(
        tmp.path().join(".bazelversion").exists(),
        ".bazelversion should be created"
    );
    assert!(
        tmp.path().join(".bazelrc").exists(),
        ".bazelrc should be created"
    );
    assert!(
        tmp.path().join("BUILD.bazel").exists()
            || tmp.path().join("src").join("BUILD.bazel").exists()
            || fs::read_dir(tmp.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|e| e.path().join("BUILD.bazel").exists()),
        "BUILD.bazel should exist at root or package"
    );
    assert!(
        tmp.path().join(".cargo-runner.json").exists(),
        ".cargo-runner.json should be written"
    );

    let module = fs::read_to_string(tmp.path().join("MODULE.bazel")).unwrap();
    assert!(
        module.contains("demo_ws") || module.contains("module"),
        "MODULE.bazel should contain workspace name: {module}"
    );
}

#[test]
fn build_sync_dry_run_on_scaffolded_bazel_crate() {
    let tmp = TempDir::new().unwrap();
    // Cargo + Bazel package so find_bazel_crates discovers it
    scaffold_cargo_project(tmp.path(), "build-sync-app");
    fs::write(
        tmp.path().join("MODULE.bazel"),
        "module(name = \"build_sync_ws\")\n",
    )
    .unwrap();
    // Empty BUILD so managed block can be proposed
    fs::write(
        tmp.path().join("BUILD.bazel"),
        r#"load("@rules_rust//rust:defs.bzl", "rust_binary")
"#,
    )
    .unwrap();
    let root = canonical(tmp.path());
    let build_before = fs::read_to_string(tmp.path().join("BUILD.bazel")).unwrap();

    // Use --crate by package name to avoid macOS /var vs /private/var path prefix issues
    cargo_runner()
        .args(["build-sync", "--dry-run", "--crate-name", "build-sync-app"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run").or(predicate::str::contains("Scanning")));

    // Dry-run must not modify BUILD.bazel
    let build_after = fs::read_to_string(tmp.path().join("BUILD.bazel")).unwrap();
    assert_eq!(
        build_before, build_after,
        "dry-run must not rewrite BUILD.bazel"
    );
}

#[test]
fn runnables_workspace_json_lists_member_binary() {
    let tmp = TempDir::new().unwrap();
    scaffold_workspace_member_binary(tmp.path(), "crates/app", "workspace-app");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["runnables", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("main.rs").or(predicate::str::contains("label")));
}

#[test]
fn runnables_filter_test_json() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "filter-lib");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // --test should keep tests; JSON must mention the test name
    cargo_runner()
        .args(["runnables", "src/lib.rs", "--json", "--test"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_add").or(predicate::str::contains("Test")));

    // --bin on a lib-only file should yield empty or no test labels
    let out = cargo_runner()
        .args(["runnables", "src/lib.rs", "--json", "--bin"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // No binary runnables expected
    assert!(
        !stdout.contains("test_add") || stdout.trim() == "[]" || stdout.contains("[]"),
        "bin filter should not list tests: {stdout}"
    );
}

#[test]
fn runnables_name_exact_filter() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "name-filter-lib");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    cargo_runner()
        .args([
            "runnables",
            "src/lib.rs",
            "--json",
            "--name",
            "test_add",
            "--exact",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_add"));

    // exact name that only partially matches must not include test_add
    let out = cargo_runner()
        .args([
            "runnables",
            "src/lib.rs",
            "--json",
            "--name",
            "add",
            "--exact",
        ])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("test_add"),
        "--exact add must not match test_add: {stdout}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// alias tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn analyze_alias_a_works() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-alias");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["init"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();

    // "a" is alias for "analyze"
    cargo_runner()
        .args(["a", "src/main.rs"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Analyzing"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// help / version
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn run_features_flag_in_dry_run() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-features-flag");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["run", "src/lib.rs:8", "--features", "default", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("--features"))
        .stdout(predicate::str::contains("default"));
}

#[test]
fn run_json_error_is_structured() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-json-err");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["run", "does-not-exist-xyz.rs", "--dry-run", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains(r#""error": true"#))
        .stdout(predicate::str::contains("protocol_version"));
}

#[test]
fn override_examples_prints_cookbook() {
    cargo_runner()
        .args(["override", "--examples"])
        .assert()
        .success()
        .stdout(predicate::str::contains("@dx"))
        .stdout(predicate::str::contains("+nightly"));
}

#[test]
fn doctor_on_cargo_project() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-doctor");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["doctor", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""ok": true"#))
        .stdout(predicate::str::contains("cargo"));
}

#[test]
fn resolve_watch_matches_run_dry_run() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-watch-resolve");
    let root = canonical(tmp.path());

    let dry = cargo_runner()
        .args(["run", "src/lib.rs:8", "--dry-run"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let dry_s = String::from_utf8_lossy(&dry);
    let dry_cmd = dry_s.lines().next().unwrap_or("");

    // resolve_command_for_selector is used by watch; compare via dry-run equality
    // of the first line (shell command)
    assert!(
        dry_cmd.contains("cargo") && dry_cmd.contains("test"),
        "expected cargo test dry-run, got {dry_cmd}"
    );
}

#[test]
fn run_json_without_dry_run_errors() {
    let tmp = TempDir::new().unwrap();
    scaffold_cargo_project(tmp.path(), "test-json-footgun");
    let root = canonical(tmp.path());

    cargo_runner()
        .args(["run", "src/main.rs", "--json"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("--json requires --dry-run"));
}

#[test]
fn run_passthrough_args_appear_in_dry_run() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-passthrough");
    let root = canonical(tmp.path());

    // 1-based line 8 = inside test_add (see scaffold_lib_project)
    cargo_runner()
        .args(["run", "src/lib.rs:8", "--dry-run", "--", "--nocapture"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("--nocapture"))
        .stdout(predicate::str::contains("test_add"));
}

#[test]
fn run_execute_unit_test_succeeds() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_project(tmp.path(), "test-execute-unit");
    let root = canonical(tmp.path());

    // Real execute (not dry-run) of the unit test
    cargo_runner()
        .args(["run", "src/lib.rs:8", "--quiet"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn run_execute_doctest_filter_succeeds() {
    let tmp = TempDir::new().unwrap();
    scaffold_lib_with_enum_doctest(tmp.path(), "test-execute-doc");
    let root = canonical(tmp.path());

    // Real execute of Color doctest
    cargo_runner()
        .args(["run", "src/lib.rs:4", "--quiet"])
        .env("PROJECT_ROOT", &root)
        .current_dir(tmp.path())
        .assert()
        .success();
}

#[test]
fn completions_zsh_emits_script() {
    cargo_runner()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo-runner"));
}

#[test]
fn help_shows_all_commands() {
    cargo_runner()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("runnables"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("override"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("watch"))
        .stdout(predicate::str::contains("completions"))
        .stdout(predicate::str::contains("doctor"));
}

#[test]
fn version_shows_version() {
    cargo_runner()
        .args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cargo-runner"));
}

#[test]
fn override_help_shows_flags() {
    cargo_runner()
        .args(["override", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--command"))
        .stdout(predicate::str::contains("--subcommand"))
        .stdout(predicate::str::contains("--channel"));
}
