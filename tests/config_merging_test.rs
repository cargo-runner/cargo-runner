//! Integration test for config merging functionality

use cargo_runner_core::config::ConfigMerger;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_hierarchy_merging() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    
    // Create workspace structure
    let workspace_dir = root_dir.join("workspace");
    let package_dir = workspace_dir.join("my-package");
    let src_dir = package_dir.join("src");
    
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir(root_dir.join("package")).unwrap();
    
    // Create root config with linked_projects
    let root_config = serde_json::json!({
        "linked_projects": ["workspace/Cargo.toml", "workspace/my-package/Cargo.toml"],
        "package": "root-workspace",
        "extra_args": ["--release"],
        "env": {
            "RUST_LOG": "info"
        },
        "overrides": []
    });
    
    fs::write(
        root_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&root_config).unwrap()
    ).unwrap();
    
    // Create workspace config
    let workspace_config = serde_json::json!({
        "package": "workspace",
        "channel": "nightly",
        "extra_args": ["--features", "workspace-feature"],
        "env": {
            "WORKSPACE_VAR": "workspace-value"
        },
        "test_frameworks": {
            "command": "cargo",
            "subcommand": "nextest run"
        },
        "overrides": []
    });
    
    fs::write(
        workspace_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&workspace_config).unwrap()
    ).unwrap();
    
    // Create package config
    let package_config = serde_json::json!({
        "package": "my-package",
        "extra_args": ["--features", "package-feature"],
        "env": {
            "PACKAGE_VAR": "package-value",
            "RUST_LOG": "debug"
        },
        "extra_test_binary_args": ["--test-threads=1"],
        "overrides": [{
            "match": {
                "function_name": "test_foo"
            },
            "extra_args": ["--nocapture"],
            "force_replace_args": true
        }]
    });
    
    fs::write(
        package_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&package_config).unwrap()
    ).unwrap();
    
    // Create dummy Cargo.toml files
    fs::write(workspace_dir.join("Cargo.toml"), "[workspace]\nmembers = [\"my-package\"]").unwrap();
    fs::write(package_dir.join("Cargo.toml"), "[package]\nname = \"my-package\"").unwrap();
    
    // Set PROJECT_ROOT
    unsafe {
        std::env::set_var("PROJECT_ROOT", root_dir);
    }
    
    // Test merging from a file in the package
    let test_file = src_dir.join("lib.rs");
    fs::write(&test_file, "// test file").unwrap();
    
    let mut merger = ConfigMerger::new();
    merger.load_configs_for_path(&test_file).unwrap();
    let merged = merger.get_merged_config();
    
    // Verify merged config
    assert_eq!(merged.package, Some("my-package".to_string())); // From package config
    assert_eq!(merged.channel, Some("nightly".to_string())); // From workspace config
    
    // Verify args are merged
    let extra_args = merged.extra_args.unwrap();
    assert!(extra_args.contains(&"--release".to_string())); // From root
    assert!(extra_args.contains(&"--features".to_string())); // From workspace and package
    assert!(extra_args.contains(&"workspace-feature".to_string())); // From workspace
    assert!(extra_args.contains(&"package-feature".to_string())); // From package
    
    // Verify env is merged with overrides
    let env = merged.extra_env.unwrap();
    assert_eq!(env.get("RUST_LOG"), Some(&"debug".to_string())); // Package overrides root
    assert_eq!(env.get("WORKSPACE_VAR"), Some(&"workspace-value".to_string())); // From workspace
    assert_eq!(env.get("PACKAGE_VAR"), Some(&"package-value".to_string())); // From package
    
    // Verify test_frameworks from workspace
    assert!(merged.test_framework.is_some());
    let test_fw = merged.test_framework.unwrap();
    assert_eq!(test_fw.command, Some("cargo".to_string()));
    assert_eq!(test_fw.subcommand, Some("nextest run".to_string()));
    
    // Verify extra_test_binary_args from package
    let test_args = merged.extra_test_binary_args.unwrap();
    assert_eq!(test_args, vec!["--test-threads=1"]);
    
    // Verify overrides from package
    assert_eq!(merged.overrides.len(), 1);
    let override_config = &merged.overrides[0];
    assert_eq!(override_config.identity.function_name, Some("test_foo".to_string()));
    assert_eq!(override_config.extra_args, Some(vec!["--nocapture".to_string()]));
    assert_eq!(override_config.force_replace_args, Some(true));
    
    // Clean up
    unsafe {
        std::env::remove_var("PROJECT_ROOT");
    }
}

#[test]
fn test_force_replace_in_overrides() {
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    
    // Create configs with overrides
    let root_config = serde_json::json!({
        "package": "root",
        "overrides": [{
            "match": {
                "function_name": "test_foo"
            },
            "extra_args": ["--arg1", "--arg2"],
            "env": {
                "VAR1": "value1"
            }
        }]
    });
    
    let package_config = serde_json::json!({
        "package": "package",
        "overrides": [{
            "match": {
                "function_name": "test_foo"
            },
            "extra_args": ["--arg3"],
            "env": {
                "VAR2": "value2"
            },
            "force_replace_args": true,
            "force_replace_env": false
        }]
    });
    
    fs::write(
        root_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&root_config).unwrap()
    ).unwrap();
    
    let package_dir = root_dir.join("package");
    fs::create_dir_all(&package_dir).unwrap();
    
    fs::write(
        package_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&package_config).unwrap()
    ).unwrap();
    fs::write(root_dir.join("package").join("Cargo.toml"), "[package]\nname = \"package\"").unwrap();
    
    unsafe {
        std::env::set_var("PROJECT_ROOT", root_dir);
    }
    
    let test_file = root_dir.join("package").join("src").join("lib.rs");
    fs::create_dir_all(test_file.parent().unwrap()).unwrap();
    fs::write(&test_file, "// test").unwrap();
    
    let mut merger = ConfigMerger::new();
    merger.load_configs_for_path(&test_file).unwrap();
    let merged = merger.get_merged_config();
    
    // Check that overrides were merged correctly
    assert_eq!(merged.overrides.len(), 1);
    let override_config = &merged.overrides[0];
    
    // Args should be replaced (force_replace_args = true)
    assert_eq!(override_config.extra_args, Some(vec!["--arg3".to_string()]));
    
    // Env should be merged (force_replace_env = false)
    let env = override_config.extra_env.as_ref().unwrap();
    assert_eq!(env.get("VAR1"), Some(&"value1".to_string()));
    assert_eq!(env.get("VAR2"), Some(&"value2".to_string()));
    
    unsafe {
        std::env::remove_var("PROJECT_ROOT");
    }
}