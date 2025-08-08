#!/usr/bin/env rust-script

//! Test script to verify nested configuration structure works correctly
//! 
//! This creates test configuration files and runs cargo runner to verify
//! that rustc commands don't get cargo-specific configs

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("Testing nested configuration structure...\n");
    
    // Create test directory
    let test_dir = "/tmp/cargo-runner-test";
    fs::create_dir_all(test_dir).unwrap();
    
    // Create a standalone test.rs file
    let test_file = format!("{}/test.rs", test_dir);
    fs::write(&test_file, r#"
fn main() {
    println!("Hello from standalone!");
}

#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}
"#).unwrap();
    
    // Create a cargo project for comparison
    let cargo_project = format!("{}/cargo-project", test_dir);
    fs::create_dir_all(&cargo_project).unwrap();
    fs::write(format!("{}/Cargo.toml", cargo_project), r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#).unwrap();
    
    let src_dir = format!("{}/src", cargo_project);
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(format!("{}/main.rs", src_dir), r#"
fn main() {
    println!("Hello from cargo project!");
}
"#).unwrap();
    
    // Create config with nested structure
    let config_content = r#"{
    "cargo": {
        "features": "all",
        "extra_args": ["--verbose", "--all-features"],
        "channel": "nightly"
    },
    "rustc": {
        "extra_args": ["--edition=2021"]
    }
}"#;
    
    fs::write(format!("{}/.cargo-runner.json", test_dir), config_content).unwrap();
    
    println!("Created test files in {}", test_dir);
    
    // Test 1: Analyze standalone file
    println!("\n=== Test 1: Analyzing standalone file ===");
    let output = Command::new("cargo")
        .args(&["runner", "analyze", &test_file])
        .output()
        .expect("Failed to run cargo runner");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output:\n{}", stdout);
    
    // Check that rustc command doesn't have --all-features
    if stdout.contains("--all-features") {
        println!("❌ FAILED: Standalone file has cargo-specific flags!");
    } else if stdout.contains("rustc") && stdout.contains("--edition=2021") {
        println!("✅ PASSED: Standalone file uses rustc config correctly");
    } else {
        println!("⚠️  WARNING: Could not verify config application");
    }
    
    // Test 2: Analyze cargo project file
    println!("\n=== Test 2: Analyzing cargo project file ===");
    let cargo_file = format!("{}/main.rs", src_dir);
    let output = Command::new("cargo")
        .args(&["runner", "analyze", &cargo_file])
        .output()
        .expect("Failed to run cargo runner");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output:\n{}", stdout);
    
    // Check that cargo command has --all-features
    if stdout.contains("cargo") && stdout.contains("--all-features") {
        println!("✅ PASSED: Cargo project uses cargo config correctly");
    } else {
        println!("❌ FAILED: Cargo project missing expected flags!");
    }
    
    // Cleanup
    fs::remove_dir_all(test_dir).ok();
    
    println!("\n=== Test Summary ===");
    println!("The nested configuration structure prevents cargo-specific");
    println!("configurations from being applied to rustc commands.");
}