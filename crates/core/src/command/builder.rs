use crate::{
    command::{CargoCommand, Target},
    config::Config,
    error::Result,
    types::{FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;
use tracing::debug;

pub struct CommandBuilder {
    config: Config,
}

impl CommandBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn build_command(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
        _project_root: &Path,
    ) -> Result<CargoCommand> {
        let mut args = vec![];

        // Determine base command
        match &runnable.kind {
            RunnableKind::Test { .. } => args.push("test".to_string()),
            RunnableKind::DocTest { .. } => {
                args.push("test".to_string());
                args.push("--doc".to_string());
            }
            RunnableKind::Benchmark { .. } => args.push("bench".to_string()),
            RunnableKind::Binary { .. } => args.push("run".to_string()),
            RunnableKind::ModuleTests { .. } => args.push("test".to_string()),
        }

        // Add package if specified
        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        // Determine and add target
        debug!("Determining target for file path: {:?}", runnable.file_path);
        if let Some(target) = Target::from_file_path(&runnable.file_path) {
            debug!("Target determined: {:?}", target);
            match target {
                Target::Lib => args.push("--lib".to_string()),
                Target::Bin(name) => {
                    if name != "main" {
                        args.push("--bin".to_string());
                        args.push(name);
                    }
                }
                Target::Example(name) => {
                    args.push("--example".to_string());
                    args.push(name);
                }
                Target::Test(name) => {
                    args.push("--test".to_string());
                    args.push(name);
                }
                Target::Bench(name) => {
                    args.push("--bench".to_string());
                    args.push(name);
                }
            }
        }

        // Add test filter for specific tests
        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                args.push("--".to_string());
                let test_path = if runnable.module_path.is_empty() {
                    test_name.clone()
                } else {
                    format!("{}::{}", runnable.module_path, test_name)
                };
                args.push(test_path);
                args.push("--exact".to_string());
            }
            RunnableKind::DocTest {
                struct_or_module_name,
                method_name,
            } => {
                args.push("--".to_string());
                if let Some(method) = method_name {
                    args.push(format!("{struct_or_module_name}::{method}"));
                } else {
                    args.push(struct_or_module_name.clone());
                }
            }
            RunnableKind::ModuleTests { module_name } => {
                args.push("--".to_string());
                // Build the full path including the module name
                if !runnable.module_path.is_empty() {
                    args.push(format!("{}::{}", runnable.module_path, module_name));
                } else {
                    args.push(module_name.clone());
                }
            }
            _ => {}
        }

        // Apply overrides from config
        let identity = self.create_function_identity(runnable, package_name);
        let command = self.apply_overrides(args, &identity);

        Ok(command)
    }

    fn create_function_identity(
        &self,
        runnable: &Runnable,
        package_name: Option<&str>,
    ) -> FunctionIdentity {
        let function_name = match &runnable.kind {
            RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
            RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
            _ => runnable.scope.name.clone(),
        };

        FunctionIdentity {
            package: package_name.map(|s| s.to_string()),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name,
        }
    }

    fn apply_overrides(&self, mut args: Vec<String>, identity: &FunctionIdentity) -> CargoCommand {
        // First apply global config
        let mut channel = self.config.channel.clone();
        let mut extra_env = self.config.extra_env.clone().unwrap_or_default();

        // Check for function-specific overrides
        if let Some(override_) = self.config.get_override_for(identity) {
            // Override channel if specified
            if let Some(override_channel) = &override_.channel {
                channel = Some(override_channel.clone());
            }

            // Handle command/subcommand overrides
            if let Some(cmd) = &override_.command {
                // If command is overridden, replace the base command
                if !args.is_empty() {
                    args[0] = cmd.clone();
                }
            }

            if let Some(subcmd) = &override_.subcommand {
                // Insert subcommand after the main command
                if !args.is_empty() {
                    args.insert(1, subcmd.clone());
                }
            }

            // Apply extra args
            if let Some(extra_args) = &override_.extra_args {
                if override_.force_replace_args.unwrap_or(false) {
                    // Replace all args after the command/subcommand
                    let cmd_count = if override_.subcommand.is_some() { 2 } else { 1 };
                    args.truncate(cmd_count);
                    args.extend(extra_args.clone());
                } else {
                    // Add extra args before the "--" separator
                    let sep_index = args.iter().position(|arg| arg == "--");
                    match sep_index {
                        Some(idx) => {
                            for arg in extra_args.iter().rev() {
                                args.insert(idx, arg.clone());
                            }
                        }
                        None => args.extend(extra_args.clone()),
                    }
                }
            }

            // Apply extra test binary args (after --)
            if let Some(test_args) = &override_.extra_test_binary_args {
                if !args.contains(&"--".to_string()) {
                    args.push("--".to_string());
                }
                args.extend(test_args.clone());
            }

            // Handle environment variables
            if let Some(override_env) = &override_.extra_env {
                if override_.force_replace_env.unwrap_or(false) {
                    extra_env = override_env.clone();
                } else {
                    extra_env.extend(override_env.clone());
                }
            }
        } else {
            // Apply global extra args if no override
            if let Some(global_args) = &self.config.extra_args {
                let sep_index = args.iter().position(|arg| arg == "--");
                match sep_index {
                    Some(idx) => {
                        for arg in global_args.iter().rev() {
                            args.insert(idx, arg.clone());
                        }
                    }
                    None => args.extend(global_args.clone()),
                }
            }

            // Apply global test binary args
            if let Some(test_args) = &self.config.extra_test_binary_args {
                if !args.contains(&"--".to_string()) {
                    args.push("--".to_string());
                }
                args.extend(test_args.clone());
            }
        }

        // Add channel to the beginning if specified
        if let Some(ch) = channel {
            args.insert(0, format!("+{ch}"));
        }

        // Create command with environment variables
        let mut command = CargoCommand::new(args);
        for (key, value) in extra_env {
            command = command.with_env(key, value);
        }

        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Position, Scope, ScopeKind};
    use std::path::PathBuf;

    fn create_test_runnable(name: &str, module_path: &str) -> Runnable {
        Runnable {
            label: format!("Run test '{name}'"),
            scope: Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
                kind: ScopeKind::Test,
                name: Some(name.to_string()),
            },
            kind: RunnableKind::Test {
                test_name: name.to_string(),
                is_async: false,
            },
            module_path: module_path.to_string(),
            file_path: PathBuf::from("/project/src/lib.rs"),
            extended_scope: None,
        }
    }

    #[test]
    fn test_build_test_command() {
        let config = Config::default();
        let builder = CommandBuilder::new(config);

        let runnable = create_test_runnable("test_addition", "my_crate::tests");
        let command = builder
            .build_command(&runnable, Some("my_crate"), Path::new("/project"))
            .unwrap();

        assert_eq!(
            command.args,
            vec![
                "test",
                "--package",
                "my_crate",
                "--lib",
                "--",
                "my_crate::tests::test_addition",
                "--exact"
            ]
        );
    }

    #[test]
    fn test_build_doc_test_command() {
        let config = Config::default();
        let builder = CommandBuilder::new(config);

        let runnable = Runnable {
            label: "Run doc test".to_string(),
            scope: Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
                kind: ScopeKind::DocTest,
                name: Some("doc test".to_string()),
            },
            kind: RunnableKind::DocTest {
                struct_or_module_name: "User".to_string(),
                method_name: Some("new".to_string()),
            },
            module_path: String::new(),
            file_path: PathBuf::from("/project/src/user.rs"),
            extended_scope: None,
        };

        let command = builder
            .build_command(&runnable, Some("my_crate"), Path::new("/project"))
            .unwrap();

        assert_eq!(
            command.args,
            vec!["test", "--doc", "--package", "my_crate", "--", "User::new"]
        );
    }

    #[test]
    fn test_build_binary_command() {
        let config = Config::default();
        let builder = CommandBuilder::new(config);

        let runnable = Runnable {
            label: "Run main()".to_string(),
            scope: Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("main".to_string()),
            },
            kind: RunnableKind::Binary { bin_name: None },
            module_path: String::new(),
            file_path: PathBuf::from("/project/src/main.rs"),
            extended_scope: None,
        };

        let command = builder
            .build_command(&runnable, Some("my_crate"), Path::new("/project"))
            .unwrap();

        assert_eq!(command.args, vec!["run", "--package", "my_crate"]);
    }
}
