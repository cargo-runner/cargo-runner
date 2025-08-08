//! Example of using cargo-runner as a library

use cargo_runner_core::{
    command::CommandBuilder,
    types::{Runnable, RunnableKind},
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Simple usage - build command for a test
    let test_runnable = Runnable {
        label: "Run test 'test_addition'".to_string(),
        scope: /* ... */,
        kind: RunnableKind::Test {
            test_name: "test_addition".to_string(),
            is_async: false,
        },
        module_path: "tests::math".to_string(),
        file_path: PathBuf::from("src/lib.rs"),
        extended_scope: None,
    };
    
    // Clean API - just chain what you need
    let command = CommandBuilder::for_runnable(&test_runnable)
        .with_package("my-crate")
        .build()?;
    
    println!("Command: {}", command.to_shell_command());
    
    // Example 2: Doc test with custom config
    let doc_test = Runnable {
        label: "Run doc test for 'User::new'".to_string(),
        scope: /* ... */,
        kind: RunnableKind::DocTest {
            struct_or_module_name: "User".to_string(),
            method_name: Some("new".to_string()),
        },
        module_path: String::new(),
        file_path: PathBuf::from("src/user.rs"),
        extended_scope: None,
    };
    
    let command = CommandBuilder::for_runnable(&doc_test)
        .with_package("my-crate")
        .with_project_root(Path::new("/path/to/project"))
        .build()?;
    
    println!("Doc test command: {}", command.to_shell_command());
    
    // Example 3: Using custom configuration
    let config = Config {
        test_frameworks: Some(TestFramework {
            command: Some("cargo".to_string()),
            subcommand: Some("nextest run".to_string()),
            channel: Some("nightly".to_string()),
            args: Some(vec!["-j10".to_string()]),
            extra_env: Some(HashMap::from([
                ("RUST_LOG".to_string(), "debug".to_string()),
            ])),
        }),
        ..Default::default()
    };
    
    let command = CommandBuilder::for_runnable(&test_runnable)
        .with_package("my-crate")
        .with_config(config)
        .build()?;
    
    println!("Test with framework: {}", command.to_shell_command());
    // Output: cargo +nightly nextest run -j10 --package my-crate --lib -- tests::math::test_addition --exact
    
    Ok(())
}

// The API is designed to be:
// 1. Discoverable - start with CommandBuilder::for_runnable()
// 2. Chainable - add only what you need with .with_*() methods
// 3. Safe - returns Result for error handling
// 4. Clean - hides complexity of config merging and builder selection