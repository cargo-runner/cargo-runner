//! Test for linked projects resolution

use cargo_runner_core::CargoRunner;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_linked_projects_resolution() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    
    // Create project structure
    let project_a_dir = root_dir.join("project-a");
    let project_b_dir = root_dir.join("project-b");
    let project_a_src = project_a_dir.join("src");
    let project_b_src = project_b_dir.join("src");
    
    fs::create_dir_all(&project_a_src).unwrap();
    fs::create_dir_all(&project_b_src).unwrap();
    
    // Create Cargo.toml files
    fs::write(
        project_a_dir.join("Cargo.toml"),
        r#"[package]
name = "project-a"
version = "0.1.0"
edition = "2021"
"#
    ).unwrap();
    
    fs::write(
        project_b_dir.join("Cargo.toml"),
        r#"[package]
name = "project-b"
version = "0.1.0"
edition = "2021"
"#
    ).unwrap();
    
    // Create test files
    fs::write(
        project_a_src.join("main.rs"),
        r#"
fn main() {
    println!("Hello from project A");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_a() {
        assert_eq!(2 + 2, 4);
    }
}
"#
    ).unwrap();
    
    fs::write(
        project_b_src.join("lib.rs"),
        r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_b() {
        assert_eq!(3 + 3, 6);
    }
}
"#
    ).unwrap();
    
    // Create root config with linked_projects
    let root_config = serde_json::json!({
        "linked_projects": [
            project_a_dir.join("Cargo.toml").display().to_string(),
            project_b_dir.join("Cargo.toml").display().to_string()
        ],
        "package": "root-workspace",
        "extra_args": [],
        "env": {},
        "overrides": []
    });
    
    fs::write(
        root_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&root_config).unwrap()
    ).unwrap();
    
    // Set PROJECT_ROOT
    unsafe {
        std::env::set_var("PROJECT_ROOT", root_dir);
    }
    
    // Test resolution from different working directories
    let original_dir = std::env::current_dir().unwrap();
    
    // Change to a different directory
    std::env::set_current_dir(&temp_dir).unwrap();
    
    // Create runner and test file resolution
    let mut runner = CargoRunner::new().unwrap();
    
    // Test 1: Resolve relative path "project-a/src/main.rs"
    let cmd = runner.get_command_at_position_with_dir("project-a/src/main.rs", Some(8)).unwrap();
    assert_eq!(cmd.working_dir, Some(project_a_dir.display().to_string()));
    assert!(cmd.args.contains(&"--package".to_string()));
    assert!(cmd.args.contains(&"project-a".to_string()));
    
    // Test 2: Resolve relative path "project-b/src/lib.rs"
    let cmd = runner.get_command_at_position_with_dir("project-b/src/lib.rs", Some(4)).unwrap();
    assert_eq!(cmd.working_dir, Some(project_b_dir.display().to_string()));
    assert!(cmd.args.contains(&"--package".to_string()));
    assert!(cmd.args.contains(&"project-b".to_string()));
    
    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();
    
    // Clean up
    unsafe {
        std::env::remove_var("PROJECT_ROOT");
    }
}

#[test]
fn test_project_root_fallback() {
    // Test that when there are no linked_projects, PROJECT_ROOT is used as working dir
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path();
    
    // Create a simple project structure
    let src_dir = root_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    
    fs::write(
        root_dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#
    ).unwrap();
    
    fs::write(
        src_dir.join("main.rs"),
        r#"
fn main() {
    println!("Hello");
}
"#
    ).unwrap();
    
    // Create config without linked_projects
    let config = serde_json::json!({
        "package": "test-project",
        "extra_args": [],
        "env": {},
        "overrides": []
    });
    
    fs::write(
        root_dir.join(".cargo-runner.json"),
        serde_json::to_string_pretty(&config).unwrap()
    ).unwrap();
    
    // Set PROJECT_ROOT
    unsafe {
        std::env::set_var("PROJECT_ROOT", root_dir);
    }
    
    // Change to the test directory to avoid picking up real project config
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root_dir).unwrap();
    
    let mut runner = CargoRunner::new().unwrap();
    
    // Even with a non-existent file, it should use PROJECT_ROOT as working dir
    let cmd = runner.get_command_at_position_with_dir("src/non_existent.rs", None).unwrap();
    
    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();
    assert_eq!(cmd.working_dir, Some(root_dir.display().to_string()));
    
    // Clean up
    unsafe {
        std::env::remove_var("PROJECT_ROOT");
    }
}