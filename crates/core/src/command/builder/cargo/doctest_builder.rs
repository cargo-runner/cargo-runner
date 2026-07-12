//! DocTest command builder

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        Command,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

/// DocTest command builder
pub struct DocTestCommandBuilder;

impl ConfigAccess for DocTestCommandBuilder {}
impl CargoBuilderHelper for DocTestCommandBuilder {}

impl CommandBuilderImpl for DocTestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
        let builder = DocTestCommandBuilder;

        // Rustdoc test names look like `src/lib.rs - nested::Item (line N)`.
        // Filters must be crate-relative (e.g. `Item`, `nested::Item`, `Type::method`)
        // and must NOT include the Cargo package name (that matches zero tests).
        let test_id = match &runnable.kind {
            RunnableKind::DocTest {
                struct_or_module_name,
                method_name,
            } => {
                // Strip "impl " prefix if present (used to differentiate impl blocks from structs)
                let clean_name = struct_or_module_name
                    .strip_prefix("impl ")
                    .unwrap_or(struct_or_module_name);

                let item = if let Some(method) = method_name {
                    format!("{clean_name}::{method}")
                } else {
                    clean_name.to_string()
                };

                let module_prefix = rustdoc_module_prefix(&runnable.module_path, package, &item);
                if module_prefix.is_empty() {
                    item
                } else if module_prefix == item || module_prefix.ends_with(&format!("::{item}")) {
                    module_prefix
                } else {
                    format!("{module_prefix}::{item}")
                }
            }
            _ => {
                return Err(crate::error::Error::UnsupportedRunnable {
                    context: "Expected DocTest runnable",
                });
            }
        };

        let mut args = vec![];
        let mut strategy = crate::command::CommandStrategy::Cargo;

        let override_cmd =
            builder.apply_cargo_override_command(&mut args, runnable, config, file_type, "test");

        if let Some((strat, _)) = override_cmd {
            strategy = strat;
            // Ensure --doc is present for cargo doctests unless a fully custom command
            if strategy == crate::command::CommandStrategy::Cargo
                && !args.iter().any(|a| a == "--doc")
            {
                args.push("--doc".to_string());
            }
        } else {
            // Add channel
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{channel}"));
            }
            args.push("test".to_string());
            args.push("--doc".to_string());
        }

        // Add package
        if strategy == crate::command::CommandStrategy::Cargo
            && let Some(pkg) = package
            && !pkg.is_empty()
        {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add doc test filter
        if strategy == crate::command::CommandStrategy::Cargo {
            args.push("--".to_string());
            args.push(test_id.clone());
            // Apply test binary args
            builder.apply_test_binary_args(&mut args, runnable, config, file_type);
        }

        let mut command = match strategy {
            crate::command::CommandStrategy::Shell => {
                let program = args.first().cloned().unwrap_or_else(|| "cargo".into());
                let rest = if args.len() > 1 {
                    args[1..].to_vec()
                } else {
                    vec![]
                };
                Command::shell(program, rest)
            }
            _ => Command::cargo(args),
        };

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        builder.apply_common_config(
            &mut command,
            config,
            file_type,
            builder.get_extra_env(config, file_type),
        );
        builder.apply_env(&mut command, runnable, config, file_type);

        Ok(command)
    }
}

/// Strip Cargo package / rustc crate name from a resolved module path so the
/// remaining path matches rustdoc filter names (`nested::Item`, not `pkg::nested::Item`).
fn rustdoc_module_prefix(module_path: &str, package: Option<&str>, item: &str) -> String {
    if module_path.is_empty() {
        return String::new();
    }

    let mut path = module_path.to_string();

    let strip_prefixes: Vec<String> = {
        let mut v = Vec::new();
        if let Some(pkg) = package {
            v.push(pkg.to_string());
            v.push(pkg.replace('-', "_"));
        }
        v
    };

    for prefix in &strip_prefixes {
        if path == *prefix {
            return String::new();
        }
        let with_sep = format!("{prefix}::");
        if let Some(rest) = path.strip_prefix(&with_sep) {
            path = rest.to_string();
            break;
        }
    }

    // If path is only the item (or ends with it), still useful as prefix empty case
    if path == item {
        return String::new();
    }

    path
}

impl DocTestCommandBuilder {
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
        if let Some(override_config) = self.get_override(runnable, config, file_type)
            && let Some(override_cargo) = &override_config.cargo
            && let Some(extra_args) = &override_cargo.extra_args
        {
            args.extend(extra_args.clone());
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
        if let Some(override_config) = self.get_override(runnable, config, file_type)
            && let Some(override_cargo) = &override_config.cargo
            && let Some(extra_args) = &override_cargo.extra_test_binary_args
        {
            args.extend(extra_args.clone());
        }

        // Apply global test binary args
        if let Some(extra_args) = self.get_extra_test_binary_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_env(
        &self,
        command: &mut Command,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type)
            && let Some(override_cargo) = &override_config.cargo
            && let Some(extra_env) = &override_cargo.extra_env
        {
            for (key, value) in extra_env {
                command.env.insert(key.clone(), value.clone());
            }
        }
    }
}
