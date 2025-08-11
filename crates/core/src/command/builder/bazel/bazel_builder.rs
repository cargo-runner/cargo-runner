//! Bazel command builder with placeholder support

use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::{BazelConfig, BazelFramework, Config},
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};
use std::path::Path;

/// Bazel command builder with rich placeholder support
pub struct BazelCommandBuilder;

impl ConfigAccess for BazelCommandBuilder {}

impl CommandBuilderImpl for BazelCommandBuilder {
    fn build(
        runnable: &Runnable,
        _package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("BazelCommandBuilder::build called for {:?}", runnable.kind);
        let builder = BazelCommandBuilder;
        let bazel_config = config.bazel.as_ref();

        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                builder.build_test_command(runnable, test_name, bazel_config, config, file_type)
            }
            RunnableKind::ModuleTests { .. } => {
                builder.build_module_tests_command(runnable, bazel_config, config, file_type)
            }
            RunnableKind::Binary { bin_name } => builder.build_binary_command(
                runnable,
                bin_name.as_deref(),
                bazel_config,
                config,
                file_type,
            ),
            RunnableKind::Benchmark { bench_name } => builder.build_benchmark_command(
                runnable,
                bench_name,
                bazel_config,
                config,
                file_type,
            ),
            RunnableKind::DocTest { .. } => {
                builder.build_doc_test_command(runnable, bazel_config, config, file_type)
            }
            _ => Err(crate::error::Error::ParseError(
                "Unsupported runnable type for bazel".to_string(),
            )),
        }
    }
}

impl BazelCommandBuilder {
    fn build_test_command(
        &self,
        runnable: &Runnable,
        test_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_test_command called for test: {}", test_name);

        // Get the test framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.test_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_test_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, true);

        // Build the test filter
        let test_filter = if runnable.module_path.is_empty() {
            test_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, test_name)
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            Some(&test_filter),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_module_tests_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_module_tests_command called");

        // Get the test framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.test_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_test_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, true);

        // Build module filter (no exact matching for module tests)
        let test_filter = if !runnable.module_path.is_empty() {
            Some(runnable.module_path.clone())
        } else {
            None
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            test_filter.as_deref(),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_binary_command(
        &self,
        runnable: &Runnable,
        bin_name: Option<&str>,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_binary_command called for binary: {:?}", bin_name);

        // Get the binary framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.binary_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_binary_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, false);

        // Build the command
        let mut command =
            self.build_command_from_framework(&framework, runnable, Some(&target), None, bin_name);

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_benchmark_command(
        &self,
        runnable: &Runnable,
        bench_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!(
            "build_benchmark_command called for benchmark: {}",
            bench_name
        );

        // Get the benchmark framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.benchmark_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_benchmark_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, true);

        // Build the benchmark filter
        let bench_filter = if runnable.module_path.is_empty() {
            bench_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, bench_name)
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            Some(&bench_filter),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_doc_test_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_doc_test_command called");

        // Get the doc test framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.doc_test_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_doc_test_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, true);

        // Build the command
        let mut command =
            self.build_command_from_framework(&framework, runnable, Some(&target), None, None);

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    /// Build a command from a framework configuration
    fn build_command_from_framework(
        &self,
        framework: &BazelFramework,
        runnable: &Runnable,
        target: Option<&str>,
        test_filter: Option<&str>,
        binary_name: Option<&str>,
    ) -> CargoCommand {
        let command_name = framework.command.as_deref().unwrap_or("bazel");
        let subcommand = framework.subcommand.as_deref().unwrap_or("test");

        let mut args = vec![subcommand.to_string()];

        // Add the target
        if let Some(target_template) = &framework.target {
            let expanded_target = self.expand_template(
                target_template,
                &runnable.file_path,
                target.unwrap_or(":test"),
                test_filter,
                binary_name,
                &runnable.module_path,
            );
            args.push(expanded_target);
        } else if let Some(target) = target {
            args.push(target.to_string());
        }

        // Add base args with placeholder expansion
        if let Some(base_args) = &framework.args {
            for arg in base_args {
                let expanded = self.expand_template(
                    arg,
                    &runnable.file_path,
                    target.unwrap_or(":test"),
                    test_filter,
                    binary_name,
                    &runnable.module_path,
                );
                args.push(expanded);
            }
        }

        // Add extra args (no expansion needed)
        if let Some(extra_args) = &framework.extra_args {
            args.extend(extra_args.clone());
        }

        // Add test args (for test subcommand)
        if subcommand == "test" && test_filter.is_some() {
            if let Some(test_args) = &framework.test_args {
                for arg in test_args {
                    let expanded = self.expand_template(
                        arg,
                        &runnable.file_path,
                        target.unwrap_or(":test"),
                        test_filter,
                        binary_name,
                        &runnable.module_path,
                    );
                    if !expanded.is_empty() {
                        // Add --test_arg before each test argument
                        args.push("--test_arg".to_string());
                        args.push(expanded);
                    }
                }
            }
        }

        // Add exec args (for run subcommand)
        if subcommand == "run" {
            if let Some(exec_args) = &framework.exec_args {
                if !exec_args.is_empty() {
                    args.push("--".to_string());
                    for arg in exec_args {
                        let expanded = self.expand_template(
                            arg,
                            &runnable.file_path,
                            target.unwrap_or("//:server"),
                            test_filter,
                            binary_name,
                            &runnable.module_path,
                        );
                        args.push(expanded);
                    }
                }
            }
        }

        let mut command = if command_name == "bazel" {
            CargoCommand::new_bazel(args)
        } else {
            // Support custom commands (like bazelisk)
            CargoCommand::new_shell(command_name.to_string(), args)
        };

        // Apply environment variables
        if let Some(env) = &framework.extra_env {
            for (key, value) in env {
                command.env.push((key.clone(), value.clone()));
            }
        }

        command
    }

    /// Determine the Bazel target based on the runnable and configuration
    fn determine_target(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        is_test: bool,
    ) -> String {
        // Check for legacy configuration first
        if let Some(config) = bazel_config {
            if is_test && config.test_target.is_some() {
                return config.test_target.clone().unwrap();
            } else if !is_test && config.binary_target.is_some() {
                return config.binary_target.clone().unwrap();
            }
        }

        // Use configured defaults if available
        if let Some(config) = bazel_config {
            if is_test && config.default_test_target.is_some() {
                return config.default_test_target.clone().unwrap();
            } else if !is_test && config.default_binary_target.is_some() {
                return config.default_binary_target.clone().unwrap();
            }
        }

        // Try to find the actual Bazel target from BUILD files
        // First, we need to find the workspace root
        let workspace_root = runnable
            .file_path
            .ancestors()
            .find(|p| p.join("WORKSPACE").exists() || p.join("MODULE.bazel").exists());

        if let Some(workspace_root) = workspace_root {
            tracing::debug!(
                "Looking for Bazel target for file: {:?} in workspace: {:?}",
                runnable.file_path,
                workspace_root
            );
            if let Some(target) = super::target_detection::find_bazel_target_for_file(
                &runnable.file_path,
                workspace_root,
                is_test,
            ) {
                tracing::debug!("Found Bazel target from BUILD file: {}", target);
                return target;
            } else {
                tracing::debug!("No Bazel target found in BUILD file");
            }
        } else {
            tracing::debug!("No workspace root found, using default target");
        }

        // Fall back to simple inference
        let file_path = &runnable.file_path;
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Check common patterns
        if file_path.to_string_lossy().contains("src/bin/") {
            // Binary in src/bin
            format!("//src/bin:{}", file_name)
        } else if file_name == "main" && !is_test {
            // Main binary
            "//:main".to_string()
        } else if is_test {
            // Default test target
            ":test".to_string()
        } else {
            // Default binary target
            "//:server".to_string()
        }
    }

    /// Expand template placeholders
    fn expand_template(
        &self,
        template: &str,
        file_path: &Path,
        target: &str,
        test_filter: Option<&str>,
        binary_name: Option<&str>,
        module_path: &str,
    ) -> String {
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let parent_dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or(".");

        // Extract target components
        let (package, target_name) = if target.contains(':') {
            let parts: Vec<&str> = target.splitn(2, ':').collect();
            (parts[0], parts[1])
        } else {
            ("", target)
        };

        template
            // Bazel-specific placeholders
            .replace("{target}", target)
            .replace("{target_name}", target_name)
            .replace("{package}", package)
            // File-related placeholders
            .replace("{file_path}", file_path.to_str().unwrap_or(""))
            .replace("{file_name}", file_name)
            .replace("{parent_dir}", parent_dir)
            // Test/benchmark placeholders
            .replace("{test_filter}", test_filter.unwrap_or(""))
            .replace("{bench_filter}", test_filter.unwrap_or(""))
            .replace("{test_name}", test_filter.unwrap_or(""))
            .replace("{module_path}", module_path)
            // Binary placeholders
            .replace("{binary_name}", binary_name.unwrap_or(file_name))
    }

    /// Get override configuration for a runnable
    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = crate::types::FunctionIdentity {
            package: config
                .bazel
                .as_ref()
                .and_then(|b| b.workspace.clone())
                .or_else(|| config.cargo.as_ref().and_then(|c| c.package.clone())),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        };

        tracing::debug!("Looking for override for identity: {:?}", identity);
        let result = config.get_override_for(&identity);
        if result.is_some() {
            tracing::debug!("Found matching override!");
        } else {
            tracing::debug!("No matching override found");
        }
        result
    }

    /// Apply overrides to the command
    fn apply_overrides(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        let override_config = self.get_override(runnable, config, file_type);

        if let Some(override_) = override_config {
            if let Some(bazel_override) = &override_.bazel {
                // Apply legacy overrides for backward compatibility
                if let Some(env) = &bazel_override.extra_env {
                    for (key, value) in env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }

                // Apply extra test args
                if let Some(extra_args) = &bazel_override.extra_test_args {
                    for arg in extra_args {
                        command.args.push("--test_arg".to_string());
                        command.args.push(arg.clone());
                    }
                }
            }
        }
    }
}
