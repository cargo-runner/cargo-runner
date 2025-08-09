//! Rustc command builder for standalone files with typestate pattern

use crate::{
    command::{builder::{CommandBuilderImpl, ConfigAccess}, CargoCommand},
    config::{Config, RustcFramework, RustcPhaseConfig},
    error::Result,
    types::{FileType, FunctionIdentity, Runnable, RunnableKind},
};

/// Rustc command builder for standalone files
pub struct RustcCommandBuilder;

impl ConfigAccess for RustcCommandBuilder {}

impl CommandBuilderImpl for RustcCommandBuilder {
    fn build(
        runnable: &Runnable,
        _package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        eprintln!("DEBUG: RustcCommandBuilder::build called for {:?}", runnable.kind);
        let builder = RustcCommandBuilder;
        
        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                builder.build_test_command(runnable, test_name, config, file_type)
            }
            RunnableKind::ModuleTests { .. } => {
                builder.build_module_tests_command(runnable, config, file_type)
            }
            RunnableKind::Binary { bin_name } => {
                builder.build_binary_command(runnable, bin_name.as_deref(), config, file_type)
            }
            RunnableKind::Standalone { .. } => {
                // Standalone is just a binary without explicit name
                builder.build_binary_command(runnable, None, config, file_type)
            }
            RunnableKind::Benchmark { bench_name } => {
                builder.build_benchmark_command(runnable, bench_name, config, file_type)
            }
            _ => Err(crate::error::Error::ParseError(
                "Unsupported runnable type for rustc".to_string(),
            )),
        }
    }
}

impl RustcCommandBuilder {
    fn build_test_command(
        &self,
        runnable: &Runnable,
        test_name: &str,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        eprintln!("DEBUG: build_test_command called for test: {}", test_name);
        let framework = self.get_test_framework(config);
        let file_name = self.get_file_name(runnable)?;
        let output_name = format!("{}_test", file_name);
        
        // Build phase
        let mut build_args = self.create_build_args(
            &framework,
            &runnable.file_path,
            &output_name,
            true, // is_test
        );
        
        // Apply configuration
        self.apply_build_config(&mut build_args, runnable, config, file_type, &framework);
        
        // Create the command
        let mut command = CargoCommand::new_rustc(build_args);
        
        // Build the full test path with module
        let test_path = if runnable.module_path.is_empty() {
            test_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, test_name)
        };
        command = command.with_test_filter(test_path);
        
        // Store exec phase args  
        let mut exec_args = Vec::new();
        if let Some(exec) = &framework.exec {
            if let Some(args) = &exec.args {
                exec_args = args.iter()
                    .map(|arg| self.expand_template(arg, &runnable.file_path, "", "", test_name))
                    .collect();
            }
        }
        
        // For individual tests, add --exact flag
        exec_args.push("--exact".to_string());
        
        if !exec_args.is_empty() {
            command.env.push((
                "_RUSTC_EXEC_ARGS".to_string(),
                exec_args.join(" "),
            ));
        }
        
        // Apply exec configuration (stored in env for later use)
        self.apply_exec_config(&mut command, runnable, config, file_type, &framework);
        
        // Apply environment variables
        self.apply_env(&mut command, runnable, config, file_type);
        
        Ok(command)
    }
    
    fn build_module_tests_command(
        &self,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let framework = self.get_test_framework(config);
        let file_name = self.get_file_name(runnable)?;
        let output_name = format!("{}_test", file_name);
        
        // Build phase
        let mut build_args = self.create_build_args(
            &framework,
            &runnable.file_path,
            &output_name,
            true, // is_test
        );
        
        // Apply configuration
        self.apply_build_config(&mut build_args, runnable, config, file_type, &framework);
        
        // Create the command (no test filter for module tests)
        let mut command = CargoCommand::new_rustc(build_args);
        
        // Apply exec configuration
        self.apply_exec_config(&mut command, runnable, config, file_type, &framework);
        
        // Apply environment variables
        self.apply_env(&mut command, runnable, config, file_type);
        
        Ok(command)
    }
    
    fn build_binary_command(
        &self,
        runnable: &Runnable,
        bin_name: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let framework = self.get_binary_framework(config);
        let file_name = self.get_file_name(runnable)?;
        let crate_name = bin_name.unwrap_or(&file_name);
        let output_name = crate_name;
        
        // Build phase
        let mut build_args = self.create_binary_build_args(
            &framework,
            &runnable.file_path,
            crate_name,
            output_name,
        );
        
        // Apply configuration
        self.apply_build_config(&mut build_args, runnable, config, file_type, &framework);
        
        // Create the command
        let mut command = CargoCommand::new_rustc(build_args);
        
        // Apply exec configuration
        self.apply_exec_config(&mut command, runnable, config, file_type, &framework);
        
        // Apply environment variables
        self.apply_env(&mut command, runnable, config, file_type);
        
        Ok(command)
    }
    
    fn build_benchmark_command(
        &self,
        runnable: &Runnable,
        bench_name: &str,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let framework = self.get_benchmark_framework(config);
        let file_name = self.get_file_name(runnable)?;
        let output_name = format!("{}_bench", file_name);
        
        // Build phase
        let mut build_args = self.create_benchmark_build_args(
            &framework,
            &runnable.file_path,
            &output_name,
        );
        
        // Apply configuration
        self.apply_build_config(&mut build_args, runnable, config, file_type, &framework);
        
        // Create the command
        let mut command = CargoCommand::new_rustc(build_args);
        
        // Build the full benchmark path with module
        let bench_path = if runnable.module_path.is_empty() {
            bench_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, bench_name)
        };
        command = command.with_test_filter(bench_path);
        
        // Store exec phase args (like --bench) 
        let mut exec_args = Vec::new();
        if let Some(exec) = &framework.exec {
            if let Some(args) = &exec.args {
                exec_args = args.iter()
                    .map(|arg| self.expand_template(arg, &runnable.file_path, "", "", bench_name))
                    .collect();
            }
        }
        
        // For individual benchmarks, add --exact flag after the benchmark name
        exec_args.push("--exact".to_string());
        
        if !exec_args.is_empty() {
            command.env.push((
                "_RUSTC_EXEC_ARGS".to_string(),
                exec_args.join(" "),
            ));
        }
        
        // Apply exec configuration (stored in env for later use)
        self.apply_exec_config(&mut command, runnable, config, file_type, &framework);
        
        // Apply environment variables
        self.apply_env(&mut command, runnable, config, file_type);
        
        Ok(command)
    }
    
    fn get_test_framework(&self, config: &Config) -> RustcFramework {
        config.rustc.as_ref()
            .and_then(|r| r.test_framework.clone())
            .unwrap_or_else(|| self.default_test_framework())
    }
    
    fn get_binary_framework(&self, config: &Config) -> RustcFramework {
        config.rustc.as_ref()
            .and_then(|r| r.binary_framework.clone())
            .unwrap_or_else(|| self.default_binary_framework())
    }
    
    fn get_benchmark_framework(&self, config: &Config) -> RustcFramework {
        config.rustc.as_ref()
            .and_then(|r| r.benchmark_framework.clone())
            .unwrap_or_else(|| self.default_benchmark_framework())
    }
    
    fn default_test_framework(&self) -> RustcFramework {
        RustcFramework {
            build: Some(RustcPhaseConfig {
                command: Some("rustc".to_string()),
                args: Some(vec![
                    "--test".to_string(),
                    "{source_file}".to_string(),
                    "-o".to_string(),
                    "{output_name}".to_string(),
                ]),
                extra_args: None,
                extra_test_binary_args: None,
            }),
            exec: Some(RustcPhaseConfig {
                command: Some("./{output_name}".to_string()),
                args: None,  // Test name is added separately with module path
                extra_args: None,
                extra_test_binary_args: None,
            }),
        }
    }
    
    fn default_binary_framework(&self) -> RustcFramework {
        RustcFramework {
            build: Some(RustcPhaseConfig {
                command: Some("rustc".to_string()),
                args: Some(vec![
                    "--crate-type".to_string(),
                    "bin".to_string(),
                    "--crate-name".to_string(),
                    "{crate_name}".to_string(),
                    "{source_file}".to_string(),
                    "-o".to_string(),
                    "{output_name}".to_string(),
                ]),
                extra_args: None,
                extra_test_binary_args: None,
            }),
            exec: Some(RustcPhaseConfig {
                command: Some("./{output_name}".to_string()),
                args: None,
                extra_args: None,
                extra_test_binary_args: None,
            }),
        }
    }
    
    fn default_benchmark_framework(&self) -> RustcFramework {
        RustcFramework {
            build: Some(RustcPhaseConfig {
                command: Some("rustc".to_string()),
                args: Some(vec![
                    "--test".to_string(),
                    "{source_file}".to_string(),
                    "-o".to_string(),
                    "{output_name}".to_string(),
                ]),
                extra_args: None,  // No extra build args by default
                extra_test_binary_args: None,
            }),
            exec: Some(RustcPhaseConfig {
                command: Some("./{output_name}".to_string()),
                args: Some(vec![
                    "--bench".to_string(),
                ]),
                extra_args: None,
                extra_test_binary_args: None,
            }),
        }
    }
    
    fn create_build_args(
        &self,
        framework: &RustcFramework,
        source_file: &std::path::Path,
        output_name: &str,
        is_test: bool,
    ) -> Vec<String> {
        if let Some(build) = &framework.build {
            if let Some(args) = &build.args {
                let mut result = Vec::new();
                for arg in args {
                    let expanded = self.expand_template(arg, source_file, output_name, "", "");
                    result.push(expanded);
                }
                return result;
            }
        }
        
        // Fallback to defaults
        if is_test {
            vec![
                "--test".to_string(),
                source_file.to_str().unwrap_or("").to_string(),
                "-o".to_string(),
                output_name.to_string(),
            ]
        } else {
            vec![
                source_file.to_str().unwrap_or("").to_string(),
                "-o".to_string(),
                output_name.to_string(),
            ]
        }
    }
    
    fn create_binary_build_args(
        &self,
        framework: &RustcFramework,
        source_file: &std::path::Path,
        crate_name: &str,
        output_name: &str,
    ) -> Vec<String> {
        if let Some(build) = &framework.build {
            if let Some(args) = &build.args {
                let mut result = Vec::new();
                for arg in args {
                    let expanded = self.expand_template(arg, source_file, output_name, crate_name, "");
                    result.push(expanded);
                }
                return result;
            }
        }
        
        // Fallback to defaults
        vec![
            "--crate-type".to_string(),
            "bin".to_string(),
            "--crate-name".to_string(),
            crate_name.to_string(),
            source_file.to_str().unwrap_or("").to_string(),
            "-o".to_string(),
            output_name.to_string(),
        ]
    }
    
    fn create_benchmark_build_args(
        &self,
        framework: &RustcFramework,
        source_file: &std::path::Path,
        output_name: &str,
    ) -> Vec<String> {
        if let Some(build) = &framework.build {
            if let Some(args) = &build.args {
                let mut result = Vec::new();
                for arg in args {
                    let expanded = self.expand_template(arg, source_file, output_name, "", "");
                    result.push(expanded);
                }
                return result;
            }
        }
        
        // Fallback to defaults
        vec![
            "--test".to_string(),
            source_file.to_str().unwrap_or("").to_string(),
            "-o".to_string(),
            output_name.to_string(),
        ]
    }
    
    fn expand_template(
        &self,
        template: &str,
        source_file: &std::path::Path,
        output_name: &str,
        crate_name: &str,
        test_name: &str,
    ) -> String {
        template
            .replace("{source_file}", source_file.to_str().unwrap_or(""))
            .replace("{output_name}", output_name)
            .replace("{crate_name}", crate_name)
            .replace("{test_name}", test_name)
            .replace("{bench_name}", test_name)  // bench_name uses same param as test_name
    }
    
    fn apply_build_config(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
        framework: &RustcFramework,
    ) {
        // Find the position of -o flag
        let output_flag_pos = args.iter().position(|arg| arg == "-o");
        
        // Collect all extra args
        let mut extra_args = Vec::new();
        
        // Apply framework extra_args
        if let Some(build) = &framework.build {
            if let Some(framework_args) = &build.extra_args {
                extra_args.extend(framework_args.clone());
            }
        }
        
        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_rustc) = &override_config.rustc {
                if let Some(override_args) = &override_rustc.extra_args {
                    extra_args.extend(override_args.clone());
                }
            }
        }
        
        // Apply global rustc args
        if let Some(rustc_config) = &config.rustc {
            if let Some(global_args) = &rustc_config.extra_args {
                extra_args.extend(global_args.clone());
            }
        }
        
        // Insert extra args before -o flag if it exists
        if let Some(pos) = output_flag_pos {
            // Insert all extra args before the -o flag
            for (i, arg) in extra_args.into_iter().enumerate() {
                args.insert(pos + i, arg);
            }
        } else {
            // No -o flag, just append
            args.extend(extra_args);
        }
    }
    
    fn apply_exec_config(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
        framework: &RustcFramework,
    ) {
        let mut exec_args = Vec::new();
        
        // Collect exec phase extra_test_binary_args
        if let Some(exec) = &framework.exec {
            if let Some(extra_test_binary_args) = &exec.extra_test_binary_args {
                exec_args.extend(extra_test_binary_args.clone());
            }
        }
        
        tracing::debug!("apply_exec_config: framework exec_args = {:?}", exec_args);
        
        // Apply override test binary args
        let override_config = self.get_override(runnable, config, file_type);
        tracing::debug!("get_override returned: {:?}", override_config.is_some());
        if let Some(override_config) = override_config {
            tracing::debug!("Found override for runnable: {:?}", runnable.get_function_name());
            // First check if there's a framework-specific override
            if let Some(override_rustc) = &override_config.rustc {
                // Get the appropriate framework based on runnable type
                let override_framework = match &runnable.kind {
                    RunnableKind::Test { .. } | RunnableKind::ModuleTests { .. } => {
                        override_rustc.test_framework.as_ref()
                    }
                    RunnableKind::Benchmark { .. } => {
                        override_rustc.benchmark_framework.as_ref()
                    }
                    _ => {
                        override_rustc.binary_framework.as_ref()
                    }
                };
                
                // Apply framework-specific exec args
                if let Some(framework) = override_framework {
                    if let Some(exec) = &framework.exec {
                        if let Some(extra_test_binary_args) = &exec.extra_test_binary_args {
                            tracing::debug!("Adding override exec args: {:?}", extra_test_binary_args);
                            exec_args.extend(extra_test_binary_args.clone());
                        }
                    }
                }
            }
            
            // Also check cargo config for backwards compatibility
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_test_binary_args) = &override_cargo.extra_test_binary_args {
                    exec_args.extend(extra_test_binary_args.clone());
                }
            }
        }
        
        // Apply global test binary args from cargo config
        if let Some(cargo_config) = &config.cargo {
            if let Some(extra_test_binary_args) = &cargo_config.extra_test_binary_args {
                exec_args.extend(extra_test_binary_args.clone());
            }
        }
        
        // Store exec args in env for later use by execute()
        tracing::debug!("Final exec_args: {:?}", exec_args);
        if !exec_args.is_empty() {
            command.env.push((
                "_RUSTC_TEST_EXTRA_ARGS".to_string(),
                exec_args.join(" "),
            ));
        }
    }
    
    fn apply_env(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars (highest priority)
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_rustc) = &override_config.rustc {
                if let Some(extra_env) = &override_rustc.extra_env {
                    for (key, value) in extra_env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
        
        // Apply global rustc env vars
        if let Some(rustc_config) = &config.rustc {
            if let Some(extra_env) = &rustc_config.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }
    }
    
    fn get_file_name(&self, runnable: &Runnable) -> Result<String> {
        runnable
            .file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| crate::error::Error::ParseError("Invalid file name".to_string()))
    }
    
    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config, file_type);
        config.get_override_for(&identity)
    }
    
    fn create_identity(
        &self,
        runnable: &Runnable,
        _config: &Config,
        file_type: FileType,
    ) -> FunctionIdentity {
        let identity = FunctionIdentity {
            package: None, // Standalone files don't have packages
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        };
        tracing::debug!("Created identity: {:?}", identity);
        identity
    }
}