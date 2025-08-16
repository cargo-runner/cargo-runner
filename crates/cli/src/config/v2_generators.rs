use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};


/// Create a v2 workspace configuration
pub fn create_v2_workspace_config() -> String {
    let config = json!({
        "version": "2.0",
        "build_system": "Cargo",
        "frameworks": {
            "test": "cargo-test",
            "binary": "cargo-run",
            "benchmark": "cargo-bench",
            "doctest": "cargo-doctest",
            "build": "cargo-build"
        }
    });

    serde_json::to_string_pretty(&config).unwrap()
}

/// Create a v2 default configuration for a crate
pub fn create_v2_default_config(_package_name: &str) -> String {
    // Individual crate configs should not have a "crates" section
    // They just inherit from the workspace root config
    let config = json!({
        "version": "2.0",
        "build_system": "Cargo",
        "frameworks": {
            "test": "cargo-test",
            "binary": "cargo-run",
            "benchmark": "cargo-bench",
            "doctest": "cargo-doctest",
            "build": "cargo-build"
        }
    });

    serde_json::to_string_pretty(&config).unwrap()
}

/// Create a v2 root configuration
pub fn create_v2_root_config(project_root: &Path, cargo_tomls: &[PathBuf]) -> Result<String> {
    // Check if this is a Bazel project
    let is_bazel = project_root.join("BUILD.bazel").exists()
        || project_root.join("BUILD").exists()
        || project_root.join("MODULE.bazel").exists()
        || project_root.join("WORKSPACE").exists()
        || project_root.join("WORKSPACE.bazel").exists();

    let build_system = if is_bazel { "Bazel" } else { "Cargo" };
    
    // Convert cargo_tomls to linked_projects
    let linked_projects: Vec<String> = cargo_tomls
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    
    // Use appropriate strategies based on build system
    let frameworks = if is_bazel {
        json!({
            "test": "bazel-test",
            "binary": "bazel-run",
            "benchmark": "bazel-bench",
            "doctest": "bazel-test",  // Bazel doesn't have separate doctest
            "build": "bazel-build"
        })
    } else {
        json!({
            "test": "cargo-test",
            "binary": "cargo-run",
            "benchmark": "cargo-bench",
            "doctest": "cargo-doctest",
            "build": "cargo-build"
        })
    };
    
    let config = json!({
        "version": "2.0",
        "linked_projects": linked_projects,
        "build_system": build_system,
        "frameworks": frameworks
    });

    Ok(serde_json::to_string_pretty(&config).unwrap())
}

/// Create a v2 configuration for rustc standalone files
pub fn create_v2_rustc_config() -> String {
    let config = json!({
        "version": "2.0",
        "build_system": "Cargo",
        "frameworks": {
            "binary": "cargo-run",
            "test": "cargo-test",
            "build": "cargo-build"
        },
        "args": {
            "all": ["--release"]
        }
    });

    serde_json::to_string_pretty(&config).unwrap()
}

/// Create a v2 configuration for single-file scripts
pub fn create_v2_single_file_script_config() -> String {
    let config = json!({
        "version": "2.0",
        "build_system": "Cargo",
        "frameworks": {
            "binary": "cargo-run",
            "test": "cargo-test",
            "build": "cargo-build"
        },
        "env": {
            "CARGO_RUNNER_SINGLE_FILE": "true"
        }
    });

    serde_json::to_string_pretty(&config).unwrap()
}

/// Create a v2 combined configuration for rustc and single-file scripts
pub fn create_v2_combined_config() -> String {
    let config = json!({
        "version": "2.0",
        "build_system": "Cargo",
        "frameworks": {
            "binary": "cargo-run",
            "test": "cargo-test",  
            "build": "cargo-build"
        },
        "args": {
            "all": ["--release"]
        },
        "env": {
            "CARGO_RUNNER_SINGLE_FILE": "true"
        }
    });

    serde_json::to_string_pretty(&config).unwrap()
}