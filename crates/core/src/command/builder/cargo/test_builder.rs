//! Test command builder with test framework support

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};
use std::path::Path;

/// Test command builder with test framework support
pub struct TestCommandBuilder;

impl ConfigAccess for TestCommandBuilder {}
impl CargoBuilderHelper for TestCommandBuilder {}

impl CommandBuilderImpl for TestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::warn!(
            "TestCommandBuilder::build called for {:?}, package={:?}",
            runnable.file_path,
            package
        );
        let builder = TestCommandBuilder;
        let mut args = vec![];

        // NUKE-CONFIG: Replaced complex test_framework handling with simple hardcoded command
        // TODO: Add back tool selection (nextest, miri) when we have simple config
        args.push("test".to_string());
        let _ = (config, file_type); // Suppress warnings

        // Add package
        if let Some(pkg) = package {
            if !pkg.is_empty() {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }
        }

        // NUKE-CONFIG: Removed test_framework check for target detection
        // Add target/bin/lib (for tests in specific files)
        tracing::debug!("Calling add_target for file: {:?}", runnable.file_path);
        builder.add_target(&mut args, &runnable.file_path, package, None)?;

        // NUKE-CONFIG: Apply configuration from complex config structure
        // TODO: Remove this once config is nuked
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add test filter
        builder.add_test_filter(&mut args, runnable, config, file_type);

        let mut command = CargoCommand::new(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            tracing::debug!(
                "Setting working directory for test command to: {:?}",
                cargo_root
            );
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        } else {
            tracing::debug!("No cargo root found for: {:?}", runnable.file_path);
        }

        // NUKE-CONFIG: Removed all complex env configuration
        // TODO: Add back simple env vars when needed
        // Just add basic RUST_BACKTRACE for debugging
        command.env.push(("RUST_BACKTRACE".to_string(), "1".to_string()));

        Ok(command)
    }
}

impl TestCommandBuilder {
    fn add_target(
        &self,
        args: &mut Vec<String>,
        file_path: &Path,
        package: Option<&str>,
        test_framework: Option<&()>, // NUKE-CONFIG: TestFramework replaced with ()
    ) -> Result<()> {
        let path_str = file_path.to_str().unwrap_or("");
        tracing::debug!(
            "add_target called with path: {}, args before: {:?}",
            path_str,
            args
        );

        // NUKE-CONFIG: Simplified - always using default cargo test now
        let is_default_test = args.contains(&"test".to_string());
        let _ = test_framework; // Suppress warning

        // For integration tests in tests/ directory, add --test flag
        if let Some(parent) = file_path.parent() {
            if parent.ends_with("tests") || parent.to_str().map_or(false, |s| s.contains("/tests/"))
            {
                if let Some(stem) = file_path.file_stem() {
                    args.push("--test".to_string());
                    args.push(stem.to_string_lossy().to_string());
                }
                return Ok(());
            }
        }

        // For tests in library source files (src/**/*.rs, excluding main.rs and bin/), add --lib flag
        // Only if using default cargo test command
        tracing::debug!(
            "Checking lib condition: is_default_test={}, path_str={}, contains_src={}, starts_with_src={}, is_lib_rs={}, ends_with_main={}, contains_bin={}",
            is_default_test,
            path_str,
            path_str.contains("/src/"),
            path_str.starts_with("src/"),
            path_str == "lib.rs",
            path_str.ends_with("/main.rs") || path_str.ends_with("main.rs"),
            path_str.contains("/bin/")
        );
        if is_default_test
            && ((path_str.contains("/src/")
                || path_str.starts_with("src/")
                || path_str == "lib.rs")
                && !path_str.ends_with("/main.rs")
                && !path_str.ends_with("main.rs")
                && !path_str.contains("/bin/"))
        {
            tracing::debug!(
                "Adding --lib for tests in library source file: {}",
                path_str
            );
            args.push("--lib".to_string());
            return Ok(());
        }

        // For tests in example files (examples/*.rs), add --example flag
        // Only if using default cargo test command
        tracing::debug!(
            "Checking example: is_default_test={}, path_str={}, contains_examples={}",
            is_default_test,
            path_str,
            path_str.contains("/examples/") || path_str.starts_with("examples/")
        );
        if is_default_test && (path_str.contains("/examples/") || path_str.starts_with("examples/"))
        {
            if let Some(stem) = file_path.file_stem() {
                let example_name = stem.to_string_lossy();
                tracing::debug!(
                    "Adding --example {} for tests in examples/{}.rs",
                    example_name,
                    example_name
                );
                args.push("--example".to_string());
                args.push(example_name.to_string());
            }
            return Ok(());
        }

        // For tests in binary files (src/main.rs, src/bin/*.rs), add --bin flag
        // Only if using default cargo test command
        let is_main_rs = path_str.ends_with("/src/main.rs") 
            || path_str == "src/main.rs" 
            || path_str == "main.rs";
            
        if is_default_test && (is_main_rs || path_str.contains("/src/bin/"))
        {
            if is_main_rs {
                // For src/main.rs, use package name as binary name
                if let Some(pkg) = package {
                    tracing::debug!("Adding --bin {} for tests in src/main.rs", pkg);
                    args.push("--bin".to_string());
                    args.push(pkg.to_string());
                }
            } else if let Some(stem) = file_path.file_stem() {
                // For src/bin/*.rs, use file stem as binary name
                let bin_name = stem.to_string_lossy();
                tracing::debug!(
                    "Adding --bin {} for tests in src/bin/{}.rs",
                    bin_name,
                    bin_name
                );
                args.push("--bin".to_string());
                args.push(bin_name.to_string());
            }
        }

        Ok(())
    }

    fn add_test_filter(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        if let RunnableKind::Test { test_name, .. } = &runnable.kind {
            args.push("--".to_string());

            // Build the full test path including module
            let full_test_path = if !runnable.module_path.is_empty() {
                format!("{}::{}", runnable.module_path, test_name)
            } else {
                test_name.clone()
            };

            args.push(full_test_path);

            // Add --exact flag for individual test functions
            args.push("--exact".to_string());

            // Apply test binary args
            self.apply_test_binary_args(args, runnable, config, file_type);
        }
    }

    fn apply_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply features first
        self.apply_features(
            args,
            runnable,
            config,
            file_type,
            self.get_features(config, file_type),
        );

        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_test_binary_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override test binary args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global test binary args
        if let Some(extra_args) = self.get_extra_test_binary_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    // NUKE-CONFIG: Removed apply_env method
    // TODO: Add simple env configuration when new config is ready
}
