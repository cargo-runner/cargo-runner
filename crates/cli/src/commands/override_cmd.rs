use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::utils::parser::parse_filepath_with_line;

pub fn override_command(
    filepath_arg: &str,
    root: bool,
    flag_command: Option<String>,
    flag_subcommand: Option<String>,
    flag_channel: Option<String>,
    override_args: Vec<String>,
) -> Result<()> {
    // Parse filepath and line number
    let (filepath, line) = parse_filepath_with_line(filepath_arg);

    println!("🔧 Creating override configuration...");
    println!("   📍 File: {filepath}");
    if let Some(line_num) = line {
        println!("   📍 Line: {}", line_num + 1); // Convert back to 1-based
    }

    // Create a runner to detect the runnable at the given location
    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    // Resolve the file path
    let resolved_path = runner.resolve_file_path(&filepath)?;

    // Get the runnable at the specified line
    let runnable = if let Some(line_num) = line {
        runner.get_best_runnable_at_line(&resolved_path, line_num as u32)?
    } else {
        // If no line specified, try to get any runnable from the file
        let all_runnables = runner.detect_all_runnables(&resolved_path)?;
        all_runnables.into_iter().next()
    };

    // If no runnable found, create a file-level override
    if runnable.is_none() {
        println!("   📄 No specific runnable found, creating file-level override");

        // For file-level overrides, we'll use the file path as the match criteria
        return create_file_level_override(
            &filepath,
            root,
            flag_command,
            flag_subcommand,
            flag_channel,
            override_args,
        );
    }

    let runnable = runnable.ok_or_else(|| anyhow::anyhow!("Runnable is expected here"))?;

    // Detect file type based on the runnable
    let file_type = runner.detect_file_type(&resolved_path)?;

    // Determine which framework we're targeting
    let framework_type = match &runnable.kind {
        cargo_runner_core::RunnableKind::Test { .. }
        | cargo_runner_core::RunnableKind::ModuleTests { .. } => "test_framework",
        cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark_framework",
        cargo_runner_core::RunnableKind::Binary { .. }
        | cargo_runner_core::RunnableKind::Standalone { .. } => "binary_framework",
        _ => "unknown",
    };

    println!("   🎯 Found: {:?}", runnable.kind);
    println!("   📝 File type: {file_type:?}");

    // Load the merged config to detect build system
    println!(
        "   📂 Loading configs for path: {}",
        resolved_path.display()
    );
    let mut merger = cargo_runner_core::config::ConfigMerger::new();
    merger.load_configs_for_path(&resolved_path)?;
    let merged_config = merger.get_merged_config();

    // Determine configuration section based on file type
    let config_section = match file_type {
        cargo_runner_core::FileType::CargoProject => {
            // Check if this is a Bazel project
            let command = merged_config
                .cargo
                .as_ref()
                .and_then(|c| c.command.as_ref());

            if let Some(cmd) = command {
                println!("   🔍 Detected command: {cmd}");
            }

            let is_bazel = command.map(|cmd| cmd == "bazel").unwrap_or(false);

            if is_bazel { "bazel" } else { "cargo" }
        }
        cargo_runner_core::FileType::Standalone => "rustc",
        cargo_runner_core::FileType::SingleFileScript => "single_file_script",
    };

    println!("   🎨 Config section: {config_section}");
    if config_section == "rustc" {
        println!("      └─ Framework: {framework_type}");
    }

    // Create function identity for the override
    let identity = cargo_runner_core::FunctionIdentity {
        package: runner.get_package_name_str(&resolved_path).ok(),
        module_path: if runnable.module_path.is_empty() {
            None
        } else {
            Some(runnable.module_path.clone())
        },
        file_path: Some(resolved_path.clone()),
        function_name: runnable.get_function_name(),
        file_type: Some(file_type),
    };

    println!("   🔑 Function identity:");
    if let Some(pkg) = &identity.package {
        println!("      - package: {pkg}");
    }
    if let Some(module) = &identity.module_path {
        println!("      - module_path: {module}");
    }
    if let Some(func) = &identity.function_name {
        println!("      - function_name: {func}");
    }
    println!("      - file_type: {file_type:?}");

    // Create the override configuration based on file type
    let mut override_config = Map::new();

    // Add matcher
    let mut matcher = Map::new();
    if let Some(pkg) = &identity.package {
        matcher.insert("package".to_string(), json!(pkg));
    }
    if let Some(module) = &identity.module_path {
        matcher.insert("module_path".to_string(), json!(module));
    }
    if let Some(func) = &identity.function_name {
        matcher.insert("function_name".to_string(), json!(func));
    }
    // Always add file_path for precise matching
    matcher.insert(
        "file_path".to_string(),
        json!(
            resolved_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid file path string"))?
        ),
    );

    override_config.insert("match".to_string(), Value::Object(matcher));

    // Parse override arguments (token-based: @dx.serve, +nightly, etc.)
    let mut parsed_args =
        cargo_runner_core::config::override_manager::OverrideManager::parse_override_args(
            &override_args,
        );

    // Named flags (--command, --subcommand, --channel) take precedence
    if let Some(cmd) = &flag_command {
        parsed_args.insert("command".to_string(), json!(cmd));
    }
    if let Some(sub) = &flag_subcommand {
        parsed_args.insert("subcommand".to_string(), json!(sub));
    }
    if let Some(ch) = &flag_channel {
        parsed_args.insert("channel".to_string(), json!(ch));
    }

    // Check if we should remove the entire override (`-` or legacy `!!`)
    if parsed_args
        .get("remove_override")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || (override_args.len() == 1 && (override_args[0] == "-" || override_args[0] == "!!"))
    {
        // Load config and remove matching override
        let config_path = if root {
            let root_path = env::var("PROJECT_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            root_path.join(".cargo-runner.json")
        } else {
            runner
                .find_config_path(&resolved_path)?
                .ok_or_else(|| anyhow::anyhow!("No config file found"))?
        };

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Map<String, Value> = serde_json::from_str(&content)?;

            if let Some(overrides) = config.get_mut("overrides").and_then(|v| v.as_array_mut()) {
                // Find and remove matching override
                overrides.retain(|o| {
                    if let Some(obj) = o.as_object()
                        && let Some(match_obj) = obj.get("match").and_then(|m| m.as_object())
                    {
                        // Check if this override matches our identity
                        let matches = match_obj
                            .get("file_path")
                            .and_then(|v| v.as_str())
                            .map(|p| Some(p) == resolved_path.to_str())
                            .unwrap_or(false);

                        if matches && identity.function_name.is_some() {
                            let func_matches = match_obj
                                .get("function_name")
                                .and_then(|v| v.as_str())
                                .map(|f| Some(f.to_string()) == identity.function_name)
                                .unwrap_or(false);
                            return !func_matches;
                        }

                        return !matches;
                    }
                    true
                });

                let json = serde_json::to_string_pretty(&config)?;
                fs::write(&config_path, json)?;

                println!("✅ Override removed successfully!");
                return Ok(());
            }
        }

        println!("❌ No matching override found to remove");
        return Ok(());
    }

    // Drop internal control flags before writing config sections
    parsed_args.remove("remove_override");
    let append_mode = parsed_args
        .remove("append")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Add configuration based on file type
    match file_type {
        cargo_runner_core::FileType::CargoProject => {
            // Check if this is a Bazel project
            let is_bazel = merged_config
                .cargo
                .as_ref()
                .and_then(|c| c.command.as_ref())
                .map(|cmd| cmd == "bazel")
                .unwrap_or(false);

            if is_bazel {
                // Create bazel override section — use flat BazelOverride field names
                // (`test_args`, `extra_args`, `exec_args`) so core can deserialize them.
                let mut bazel_config = Map::new();

                if let Some(extra_args) = parsed_args.get("extra_args") {
                    if matches!(
                        &runnable.kind,
                        cargo_runner_core::RunnableKind::Test { .. }
                            | cargo_runner_core::RunnableKind::ModuleTests { .. }
                    ) {
                        // Cargo-style flags on tests map to bazel --test_arg values
                        // only when they look like test binary args; otherwise extra_args.
                        bazel_config.insert("extra_args".to_string(), extra_args.clone());
                    } else {
                        // Binary/run: append as exec args after `--`
                        bazel_config.insert("exec_args".to_string(), extra_args.clone());
                    }
                }

                // Test binary args → Bazel test_args (--test_arg)
                if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                    bazel_config.insert("test_args".to_string(), extra_test_binary_args.clone());
                }

                if let Some(command) = parsed_args.get("command") {
                    bazel_config.insert("command".to_string(), command.clone());
                }
                if let Some(subcommand) = parsed_args.get("subcommand") {
                    bazel_config.insert("subcommand".to_string(), subcommand.clone());
                }

                if let Some(extra_env) = parsed_args.get("extra_env") {
                    bazel_config.insert("extra_env".to_string(), extra_env.clone());
                }

                override_config.insert("bazel".to_string(), Value::Object(bazel_config));
            } else {
                // Create cargo override section
                let mut cargo_config = Map::new();

                // Handle command and subcommand
                if let Some(command) = parsed_args.get("command") {
                    cargo_config.insert("command".to_string(), command.clone());
                }
                if let Some(subcommand) = parsed_args.get("subcommand") {
                    cargo_config.insert("subcommand".to_string(), subcommand.clone());
                }
                if let Some(channel) = parsed_args.get("channel") {
                    cargo_config.insert("channel".to_string(), channel.clone());
                }

                if let Some(extra_args) = parsed_args.get("extra_args") {
                    cargo_config.insert("extra_args".to_string(), extra_args.clone());
                }
                if let Some(extra_env) = parsed_args.get("extra_env") {
                    cargo_config.insert("extra_env".to_string(), extra_env.clone());
                }
                if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                    cargo_config.insert(
                        "extra_test_binary_args".to_string(),
                        extra_test_binary_args.clone(),
                    );
                }
                override_config.insert("cargo".to_string(), Value::Object(cargo_config));
            }
        }
        cargo_runner_core::FileType::Standalone => {
            // For rustc
            let mut rustc_config = Map::new();

            // Determine which framework to configure based on runnable kind
            let framework_key = match &runnable.kind {
                cargo_runner_core::RunnableKind::Test { .. }
                | cargo_runner_core::RunnableKind::ModuleTests { .. } => "test_framework",
                cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark_framework",
                _ => "binary_framework",
            };

            let mut framework = Map::new();
            let mut build = Map::new();
            let mut exec = Map::new();

            // Handle command for build phase (rustc can have custom command)
            if let Some(command) = parsed_args.get("command") {
                build.insert("command".to_string(), command.clone());
            }

            // Add parsed arguments to appropriate sections
            if let Some(extra_args) = parsed_args.get("extra_args") {
                build.insert("extra_args".to_string(), extra_args.clone());
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                exec.insert("extra_env".to_string(), extra_env.clone());
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                exec.insert(
                    "extra_test_binary_args".to_string(),
                    extra_test_binary_args.clone(),
                );
            }

            if !build.is_empty() {
                framework.insert("build".to_string(), Value::Object(build));
            }
            if !exec.is_empty() {
                framework.insert("exec".to_string(), Value::Object(exec));
            }

            rustc_config.insert(framework_key.to_string(), Value::Object(framework));
            // Channel lives at the rustc root (applies via rustup run <channel> rustc)
            if let Some(channel) = parsed_args.get("channel") {
                rustc_config.insert("channel".to_string(), channel.clone());
            }
            override_config.insert("rustc".to_string(), Value::Object(rustc_config));
        }
        cargo_runner_core::FileType::SingleFileScript => {
            let mut script_config = Map::new();

            // Handle command and subcommand
            if let Some(command) = parsed_args.get("command") {
                script_config.insert("command".to_string(), command.clone());
            }
            if let Some(subcommand) = parsed_args.get("subcommand") {
                script_config.insert("subcommand".to_string(), subcommand.clone());
            }
            if let Some(channel) = parsed_args.get("channel") {
                script_config.insert("channel".to_string(), channel.clone());
            }

            if let Some(extra_args) = parsed_args.get("extra_args") {
                script_config.insert("extra_args".to_string(), extra_args.clone());
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                script_config.insert("extra_env".to_string(), extra_env.clone());
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                script_config.insert(
                    "extra_test_binary_args".to_string(),
                    extra_test_binary_args.clone(),
                );
            }
            override_config.insert(
                "single_file_script".to_string(),
                Value::Object(script_config),
            );
        }
    }

    // Load existing config
    let config_path = if root {
        // Use PROJECT_ROOT or current directory for root config
        let root_path = env::var("PROJECT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        root_path.join(".cargo-runner.json")
    } else {
        // Find the closest project config
        runner
            .find_config_path(&resolved_path)?
            .ok_or_else(|| anyhow::anyhow!("No config file found. Run 'cargo runner init' first"))?
    };

    println!("   📄 Config file: {}", config_path.display());

    // Read existing config or create new one
    let mut config: Map<String, Value> = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        serde_json::from_str(&content)?
    } else {
        let mut new_config = Map::new();
        new_config.insert("overrides".to_string(), json!([]));
        new_config
    };

    // Get or create the overrides array
    let overrides = config
        .get_mut("overrides")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            anyhow::anyhow!("Invalid config format: missing or invalid 'overrides' array")
        })?;

    // Check if an override already exists for this identity
    let existing_index = overrides.iter().position(|o| {
        if let Some(obj) = o.as_object()
            && let Some(match_obj) = obj.get("match").and_then(|m| m.as_object())
        {
            // Check if this override matches our identity
            let file_matches = match_obj
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(|p| Some(p) == resolved_path.to_str())
                .unwrap_or(false);

            if file_matches && identity.function_name.is_some() {
                let func_matches = match_obj
                    .get("function_name")
                    .and_then(|v| v.as_str())
                    .map(|f| Some(f.to_string()) == identity.function_name)
                    .unwrap_or(false);
                return func_matches;
            }

            return file_matches;
        }
        false
    });

    // Handle removal operations on existing override
    if let Some(idx) = existing_index {
        let existing = &mut overrides[idx];
        if let Some(existing_obj) = existing.as_object_mut() {
            // Get the appropriate config section
            let config_section = match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel {
                        existing_obj.get_mut("bazel")
                    } else {
                        existing_obj.get_mut("cargo")
                    }
                }
                cargo_runner_core::FileType::Standalone => existing_obj.get_mut("rustc"),
                cargo_runner_core::FileType::SingleFileScript => {
                    existing_obj.get_mut("single_file_script")
                }
            };

            if let Some(section) = config_section.and_then(|v| v.as_object_mut()) {
                // Apply removals
                if parsed_args.get("remove_command").is_some() {
                    section.remove("command");
                }
                if parsed_args.get("remove_subcommand").is_some() {
                    section.remove("subcommand");
                }
                if parsed_args.get("remove_channel").is_some() {
                    section.remove("channel");
                }
                if parsed_args.get("remove_args").is_some() {
                    section.remove("extra_args");
                }
                if parsed_args.get("remove_env").is_some() {
                    section.remove("extra_env");
                }
                if parsed_args.get("remove_test_args").is_some() {
                    section.remove("extra_test_binary_args");
                }

                // Remove specific env vars
                if let Some(env_keys) = parsed_args
                    .get("remove_env_keys")
                    .and_then(|v| v.as_array())
                    && let Some(extra_env) =
                        section.get_mut("extra_env").and_then(|v| v.as_object_mut())
                {
                    for key in env_keys {
                        if let Some(key_str) = key.as_str() {
                            extra_env.remove(key_str);
                        }
                    }
                }
            }

            // Merge in the new override config
            let section_key = match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel { "bazel" } else { "cargo" }
                }
                cargo_runner_core::FileType::Standalone => "rustc",
                cargo_runner_core::FileType::SingleFileScript => "single_file_script",
            };

            if let Some(new_section) = override_config.get(section_key) {
                if append_mode {
                    // Merge fields into the existing section (append mode: bare `@` first)
                    let existing_section = existing_obj
                        .entry(section_key.to_string())
                        .or_insert_with(|| Value::Object(Map::new()));
                    if let (Some(dst), Some(src)) = (
                        existing_section.as_object_mut(),
                        new_section.as_object(),
                    ) {
                        merge_json_section(dst, src);
                    }
                    println!("✅ Override appended (merged) successfully!");
                } else {
                    existing_obj.insert(section_key.to_string(), new_section.clone());
                    println!("✅ Override updated successfully!");
                }
            } else {
                println!("✅ Override updated successfully!");
            }
        }
    } else {
        // Add new override
        overrides.push(Value::Object(override_config));
        println!("✅ Override added successfully!");
    }

    // Write the updated config
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, json)?;

    println!("   • Arguments: {override_args:?}");

    // Show what was applied
    if !parsed_args.is_empty() {
        println!(
            "\n   📋 Applied changes to {}:",
            match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel {
                        "bazel configuration"
                    } else {
                        "cargo configuration"
                    }
                }
                cargo_runner_core::FileType::Standalone => {
                    match &runnable.kind {
                        cargo_runner_core::RunnableKind::Test { .. }
                        | cargo_runner_core::RunnableKind::ModuleTests { .. } => {
                            "rustc.test_framework configuration"
                        }
                        cargo_runner_core::RunnableKind::Benchmark { .. } => {
                            "rustc.benchmark_framework configuration"
                        }
                        _ => "rustc.binary_framework configuration",
                    }
                }
                cargo_runner_core::FileType::SingleFileScript => "single_file_script configuration",
            }
        );
        if parsed_args.contains_key("command") {
            println!("      • command: {}", parsed_args["command"]);
        }
        if parsed_args.contains_key("subcommand") {
            println!("      • subcommand: {}", parsed_args["subcommand"]);
        }
        if parsed_args.contains_key("channel") {
            println!("      • channel: {}", parsed_args["channel"]);
        }
        if parsed_args.contains_key("extra_args")
            && let Some(args) = parsed_args["extra_args"].as_array()
        {
            let args_str: Vec<String> = args
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if file_type == cargo_runner_core::FileType::Standalone {
                println!("      • build.extra_args: {args_str:?}");
            } else {
                println!("      • extra_args: {args_str:?}");
            }
        }
        if parsed_args.contains_key("extra_env")
            && let Some(env) = parsed_args["extra_env"].as_object()
        {
            for (k, v) in env {
                if file_type == cargo_runner_core::FileType::Standalone {
                    println!("      • exec.extra_env: {k}={v}");
                } else {
                    println!("      • env: {k}={v}");
                }
            }
        }
        if parsed_args.contains_key("extra_test_binary_args")
            && let Some(args) = parsed_args["extra_test_binary_args"].as_array()
        {
            let args_str: Vec<String> = args
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if file_type == cargo_runner_core::FileType::Standalone {
                println!("      • exec.extra_test_binary_args: {args_str:?}");
            } else {
                println!("      • extra_test_binary_args: {args_str:?}");
            }
        }

        // Show removals
        if parsed_args.contains_key("remove_command") {
            println!("      • ❌ removed: command");
        }
        if parsed_args.contains_key("remove_subcommand") {
            println!("      • ❌ removed: subcommand");
        }
        if parsed_args.contains_key("remove_channel") {
            println!("      • ❌ removed: channel");
        }
        if parsed_args.contains_key("remove_args") {
            println!("      • ❌ removed: extra_args");
        }
        if parsed_args.contains_key("remove_env") {
            println!("      • ❌ removed: extra_env");
        }
        if parsed_args.contains_key("remove_test_args") {
            println!("      • ❌ removed: extra_test_binary_args");
        }
        if let Some(env_keys) = parsed_args
            .get("remove_env_keys")
            .and_then(|v| v.as_array())
        {
            for key in env_keys {
                if let Some(key_str) = key.as_str() {
                    println!("      • ❌ removed env: {key_str}");
                }
            }
        }
    }

    Ok(())
}

fn create_file_level_override(
    filepath: &str,
    root: bool,
    flag_command: Option<String>,
    flag_subcommand: Option<String>,
    flag_channel: Option<String>,
    override_args: Vec<String>,
) -> Result<()> {
    // Parse the override arguments - this returns a Map with the parsed configuration
    let mut override_config =
        cargo_runner_core::config::override_manager::OverrideManager::parse_override_args(
            &override_args,
        );

    // Named flags take precedence
    if let Some(cmd) = &flag_command {
        override_config.insert("command".to_string(), json!(cmd));
    }
    if let Some(sub) = &flag_subcommand {
        override_config.insert("subcommand".to_string(), json!(sub));
    }
    if let Some(ch) = &flag_channel {
        override_config.insert("channel".to_string(), json!(ch));
    }

    // Create the match criteria for file-level override
    let mut match_criteria = Map::new();
    match_criteria.insert("file_path".to_string(), json!(filepath));

    // Create the final override entry
    let mut override_entry = Map::new();
    override_entry.insert("match".to_string(), Value::Object(match_criteria));

    // Separate cargo-specific fields from other fields
    let cargo_fields = vec![
        "command",
        "subcommand",
        "channel",
        "extra_args",
        "extra_env",
        "extra_test_binary_args",
        "package",
        "remove_command",
        "remove_subcommand",
        "remove_channel",
        "remove_args",
        "remove_env",
        "remove_test_args",
        "remove_env_keys",
    ];

    let mut cargo_config = Map::new();
    let mut has_cargo_fields = false;

    // Add all the override configurations, separating cargo fields
    for (key, value) in override_config {
        if cargo_fields.contains(&key.as_str()) {
            cargo_config.insert(key, value);
            has_cargo_fields = true;
        } else {
            // Non-cargo fields go at the root level
            override_entry.insert(key, value);
        }
    }

    // Only add cargo section if we have cargo fields
    if has_cargo_fields {
        override_entry.insert("cargo".to_string(), Value::Object(cargo_config));
    }

    // Debug: verify structure
    if has_cargo_fields {
        println!("   📝 Cargo fields moved to cargo section");
    }

    // Determine config file location
    let config_path = if root {
        // Get PROJECT_ROOT from environment
        let project_root = env::var("PROJECT_ROOT")
            .map(PathBuf::from)
            .or_else(|_| env::current_dir())
            .context("Failed to determine project root")?;
        project_root.join(".cargo-runner.json")
    } else {
        // Find the nearest .cargo-runner.json file
        let path = Path::new(filepath);
        let mut current = path.parent();

        while let Some(dir) = current {
            let config_path = dir.join(".cargo-runner.json");
            if config_path.exists() {
                println!("   📂 Found config at: {}", config_path.display());
                cargo_runner_core::config::override_manager::OverrideManager::add_override_to_existing_config(&config_path, override_entry)?;

                println!("\n✅ File-level override created successfully!");
                println!("   📍 Config: {}", config_path.display());
                println!("   📄 File: {filepath}");

                return Ok(());
            }
            current = dir.parent();
        }

        // If no config found, create one in the file's directory
        let parent_dir = path.parent().unwrap_or(Path::new("."));
        parent_dir.join(".cargo-runner.json")
    };

    // Add the override to the config
    cargo_runner_core::config::override_manager::OverrideManager::add_override_to_existing_config(
        &config_path,
        override_entry.clone(),
    )?;

    println!("\n✅ File-level override created successfully!");
    println!("   📍 Config: {}", config_path.display());
    println!("   📄 File: {filepath}");

    // Show what was configured
    if let Some(cargo_config) = override_entry.get("cargo")
        && let Some(obj) = cargo_config.as_object()
    {
        if let Some(subcommand) = obj.get("subcommand") {
            println!(
                "   🚀 Subcommand: cargo {}",
                subcommand.as_str().unwrap_or("")
            );
        }
        if let Some(extra_args) = obj.get("extra_args")
            && let Some(args) = extra_args.as_array()
        {
            let args_str: Vec<String> = args
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !args_str.is_empty() {
                println!("   📝 Extra args: {}", args_str.join(" "));
            }
        }
    }

    if let Some(bazel_config) = override_entry.get("bazel")
        && let Some(obj) = bazel_config.as_object()
    {
        if let Some(test_args) = obj
            .get("test_args")
            .or_else(|| obj.get("extra_test_args"))
            && let Some(args) = test_args.as_array()
        {
            let args_str: Vec<String> = args
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !args_str.is_empty() {
                println!("   📝 test_args (Bazel): {}", args_str.join(" "));
            }
        }
        if let Some(exec_args) = obj
            .get("exec_args")
            .or_else(|| obj.get("extra_run_args"))
            && let Some(args) = exec_args.as_array()
        {
            let args_str: Vec<String> = args
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !args_str.is_empty() {
                println!("   📝 exec_args (Bazel): {}", args_str.join(" "));
            }
        }
    }

    Ok(())
}

/// List overrides from `.cargo-runner.json` files under the workspace.
///
/// When `file_filter` is set, only overrides whose `match.file_path` equals
/// that path (after resolution) are returned.
pub fn list_overrides_command(file_filter: Option<&str>, json: bool) -> Result<()> {
    let cwd = env::current_dir().context("failed to get current directory")?;
    let filter_path = file_filter.map(|f| resolve_path(&cwd, f));

    let configs = collect_cargo_runner_configs(&cwd)?;
    let mut entries = Vec::new();

    for config_path in configs {
        let content = fs::read_to_string(&config_path).with_context(|| {
            format!("failed to read config {}", config_path.display())
        })?;
        let config: Value = serde_json::from_str(&content).with_context(|| {
            format!("failed to parse config {}", config_path.display())
        })?;
        let Some(overrides) = config.get("overrides").and_then(|v| v.as_array()) else {
            continue;
        };

        for ov in overrides {
            if let Some(ref filter) = filter_path {
                let matched = ov
                    .get("match")
                    .and_then(|m| m.get("file_path"))
                    .and_then(|p| p.as_str())
                    .map(|p| Path::new(p) == filter.as_path() || p == filter.to_string_lossy())
                    .unwrap_or(false);
                if !matched {
                    continue;
                }
            }

            let mut entry = Map::new();
            entry.insert(
                "config_path".to_string(),
                json!(config_path.to_string_lossy()),
            );
            entry.insert("override".to_string(), ov.clone());
            entries.push(Value::Object(entry));
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if entries.is_empty() {
        println!("No overrides found.");
    } else {
        println!("Found {} override(s):\n", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            let config_path = entry
                .get("config_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let ov = entry.get("override");
            let match_obj = ov.and_then(|o| o.get("match"));
            let func = match_obj
                .and_then(|m| m.get("function_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("(file-level)");
            let file = match_obj
                .and_then(|m| m.get("file_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            println!("{}. {func}", i + 1);
            println!("   file: {file}");
            println!("   config: {config_path}");
            if let Some(cargo) = ov.and_then(|o| o.get("cargo")) {
                println!("   cargo: {cargo}");
            }
            if let Some(bazel) = ov.and_then(|o| o.get("bazel")) {
                println!("   bazel: {bazel}");
            }
            println!();
        }
    }

    Ok(())
}

/// Show the override matching a filepath (and optional line-selected function).
pub fn show_override_command(filepath_arg: &str, json: bool) -> Result<()> {
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    let cwd = env::current_dir().context("failed to get current directory")?;
    let absolute = resolve_path(&cwd, &filepath);

    let runner = cargo_runner_core::UnifiedRunner::new()?;
    let function_name = if absolute.exists() {
        if let Some(line_num) = line {
            runner
                .get_best_runnable_at_line(&absolute, line_num as u32)?
                .and_then(|r| r.get_function_name())
        } else {
            None
        }
    } else {
        None
    };

    let configs = collect_cargo_runner_configs(&cwd)?;
    let mut matches = Vec::new();

    for config_path in configs {
        let content = fs::read_to_string(&config_path)?;
        let Ok(config) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        let Some(overrides) = config.get("overrides").and_then(|v| v.as_array()) else {
            continue;
        };

        for ov in overrides {
            let Some(match_obj) = ov.get("match").and_then(|m| m.as_object()) else {
                continue;
            };
            let file_ok = match_obj
                .get("file_path")
                .and_then(|p| p.as_str())
                .map(|p| Path::new(p) == absolute.as_path() || p == absolute.to_string_lossy())
                .unwrap_or(false);
            if !file_ok {
                continue;
            }
            if let Some(ref want_fn) = function_name {
                let fn_ok = match_obj
                    .get("function_name")
                    .and_then(|f| f.as_str())
                    .map(|f| f == want_fn.as_str())
                    .unwrap_or(false);
                if !fn_ok {
                    continue;
                }
            }

            let mut entry = Map::new();
            entry.insert(
                "config_path".to_string(),
                json!(config_path.to_string_lossy()),
            );
            entry.insert("override".to_string(), ov.clone());
            matches.push(Value::Object(entry));
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&matches)?);
    } else if matches.is_empty() {
        println!("No matching override found for {filepath_arg}");
    } else {
        for entry in &matches {
            println!("{}", serde_json::to_string_pretty(entry)?);
        }
    }

    Ok(())
}

fn resolve_path(cwd: &Path, filepath: &str) -> PathBuf {
    let path = Path::new(filepath);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

/// Merge override section objects for append mode (`@` first token).
/// - Scalars / objects: incoming wins for same keys
/// - Arrays: append unique items
/// - `extra_env` objects: merge key-by-key (incoming wins)
fn merge_json_section(dst: &mut Map<String, Value>, src: &Map<String, Value>) {
    for (key, value) in src {
        match (dst.get_mut(key), value) {
            (Some(Value::Array(existing)), Value::Array(incoming)) => {
                for item in incoming {
                    if !existing.contains(item) {
                        existing.push(item.clone());
                    }
                }
            }
            (Some(Value::Object(existing)), Value::Object(incoming)) => {
                for (k, v) in incoming {
                    existing.insert(k.clone(), v.clone());
                }
            }
            _ => {
                dst.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Walk from cwd upward and into children to find `.cargo-runner.json` files.
fn collect_cargo_runner_configs(cwd: &Path) -> Result<Vec<PathBuf>> {
    let mut configs = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Prefer nearest configs walking up from cwd.
    let mut current = Some(cwd);
    while let Some(dir) = current {
        let candidate = dir.join(".cargo-runner.json");
        if candidate.is_file() && seen.insert(candidate.clone()) {
            configs.push(candidate);
        }
        current = dir.parent();
    }

    // Also scan one level of children (workspace members often have local configs).
    if let Ok(entries) = fs::read_dir(cwd) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let candidate = path.join(".cargo-runner.json");
                if candidate.is_file() && seen.insert(candidate.clone()) {
                    configs.push(candidate);
                }
            }
        }
    }

    // PROJECT_ROOT if set
    if let Ok(root) = env::var("PROJECT_ROOT") {
        let candidate = PathBuf::from(root).join(".cargo-runner.json");
        if candidate.is_file() && seen.insert(candidate.clone()) {
            configs.push(candidate);
        }
    }

    Ok(configs)
}
