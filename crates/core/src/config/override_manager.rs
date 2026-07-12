use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

pub struct OverrideManager;

impl OverrideManager {
    /// Parse override tokens into a configuration map.
    ///
    /// Supported tokens:
    /// - `@cmd.sub` — set command and subcommand (e.g. `@dx.serve`)
    /// - `@` alone as the first token — append mode (merge with existing override)
    /// - `+channel` — Rust toolchain channel
    /// - `KEY=value` — environment variable
    /// - `/args…` or `# args…` — test binary args (after `--` in cargo test)
    /// - `-command` / `-env` / `-arg` / `-test` / `-/` — remove fields
    /// - `!env` / `!#` / `!/` / `!features` / `!args` — legacy reset aliases
    /// - `!!` or `-` — remove entire override
    /// - other tokens — `extra_args`
    pub fn parse_override_args(args: &[String]) -> Map<String, Value> {
        let mut result = Map::new();
        let mut extra_args = Vec::new();
        let mut extra_env = Map::new();
        let mut extra_test_binary_args = Vec::new();
        let mut command = None;
        let mut subcommand = None;
        let mut channel = None;

        let mut remove_command = false;
        let mut remove_subcommand = false;
        let mut remove_channel = false;
        let mut remove_args = false;
        let mut remove_env = false;
        let mut remove_test_args = false;
        let mut remove_override = false;
        let mut append_mode = false;
        let mut env_to_remove = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];

            // Bare `@` as first token enables append/merge mode (legacy cargo-runner UX).
            if i == 0 && arg == "@" {
                append_mode = true;
                i += 1;
                continue;
            }

            // Full override removal (legacy `!!` or existing `-`).
            if arg == "!!" || arg == "-" {
                remove_override = true;
                i += 1;
                continue;
            }

            // Legacy `!field` resets (old VS Code tokenizer).
            if arg.starts_with('!') && arg.len() > 1 {
                match arg.as_str() {
                    "!env" => remove_env = true,
                    "!#" | "!/" | "!test" => remove_test_args = true,
                    "!args" | "!features" | "!" => remove_args = true,
                    "!command" | "!cmd" => remove_command = true,
                    "!subcommand" | "!sub" => remove_subcommand = true,
                    "!channel" | "!ch" => remove_channel = true,
                    _ => {
                        // Unknown `!token` falls through as extra_args for forward compat.
                        extra_args.push(arg.clone());
                    }
                }
                i += 1;
                continue;
            }

            if arg.starts_with('-') && !arg.starts_with("--") {
                match arg.as_str() {
                    "-command" | "-cmd" => remove_command = true,
                    "-subcommand" | "-sub" => remove_subcommand = true,
                    "-channel" | "-ch" => remove_channel = true,
                    "-arg" => remove_args = true,
                    "-env" => remove_env = true,
                    "-test" | "-/" => remove_test_args = true,
                    _ => {
                        let env_name = &arg[1..];
                        if env_name.chars().all(|c| c.is_uppercase() || c == '_')
                            && !env_name.is_empty()
                        {
                            env_to_remove.push(env_name.to_string());
                        }
                    }
                }
            } else if let Some(token) = arg.strip_prefix('@') {
                let parts: Vec<&str> = token.split('.').collect();
                if !parts.is_empty() && !parts[0].is_empty() {
                    let cmd = parts[0];
                    if cmd == "cargo" && parts.len() > 1 {
                        subcommand = Some(parts[1..].join(" "));
                    } else {
                        command = Some(cmd.to_string());
                        if parts.len() > 1 {
                            subcommand = Some(parts[1..].join(" "));
                        }
                    }
                }
            } else if arg.starts_with('+') && arg.len() > 1 {
                channel = Some(arg[1..].to_string());
            } else if arg == "/" || arg.starts_with('/') || arg == "#" || arg.starts_with('#') {
                // `/` (new) and `#` (legacy VS Code) both introduce test binary args.
                let rest = if arg == "/" || arg == "#" {
                    None
                } else if let Some(stripped) = arg.strip_prefix('/') {
                    Some(stripped.to_string())
                } else {
                    arg.strip_prefix('#').map(|s| s.to_string())
                };
                if let Some(first) = rest.filter(|s| !s.is_empty()) {
                    extra_test_binary_args.push(first);
                }
                while i + 1 < args.len() {
                    i += 1;
                    extra_test_binary_args.push(args[i].clone());
                }
            } else if arg
                .chars()
                .take_while(|&c| c != '=')
                .all(|c| c.is_uppercase() || c == '_')
                && arg.contains('=')
            {
                let parts: Vec<&str> = arg.splitn(2, '=').collect();
                if parts.len() == 2 && !parts[0].is_empty() {
                    extra_env.insert(parts[0].to_string(), json!(parts[1]));
                }
            } else {
                extra_args.push(arg.clone());
            }
            i += 1;
        }

        if remove_override {
            result.insert("remove_override".to_string(), json!(true));
            return result;
        }

        if append_mode {
            result.insert("append".to_string(), json!(true));
        }

        if let Some(cmd) = command {
            if !remove_command {
                result.insert("command".to_string(), json!(cmd));
            }
        } else if remove_command {
            result.insert("remove_command".to_string(), json!(true));
        }

        if let Some(sub) = subcommand {
            if !remove_subcommand {
                result.insert("subcommand".to_string(), json!(sub));
            }
        } else if remove_subcommand {
            result.insert("remove_subcommand".to_string(), json!(true));
        }

        if let Some(ch) = channel {
            if !remove_channel {
                result.insert("channel".to_string(), json!(ch));
            }
        } else if remove_channel {
            result.insert("remove_channel".to_string(), json!(true));
        }

        if !extra_args.is_empty() && !remove_args {
            result.insert("extra_args".to_string(), json!(extra_args));
        } else if remove_args {
            result.insert("remove_args".to_string(), json!(true));
        }

        if !extra_env.is_empty() && !remove_env {
            result.insert("extra_env".to_string(), Value::Object(extra_env));
        } else if remove_env {
            result.insert("remove_env".to_string(), json!(true));
        }

        if !env_to_remove.is_empty() {
            result.insert("remove_env_keys".to_string(), json!(env_to_remove));
        }

        if !extra_test_binary_args.is_empty() && !remove_test_args {
            result.insert(
                "extra_test_binary_args".to_string(),
                json!(extra_test_binary_args),
            );
        } else if remove_test_args {
            result.insert("remove_test_args".to_string(), json!(true));
        }

        result
    }

    pub fn add_override_to_existing_config(
        config_path: &Path,
        override_entry: Map<String, Value>,
    ) -> Result<()> {
        let mut config: Map<String, Value> = if config_path.exists() {
            let content = fs::read_to_string(config_path)
                .with_context(|| format!("Failed to read config from {}", config_path.display()))?;
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse config from {}", config_path.display()))?
        } else {
            let mut new_config = Map::new();
            new_config.insert(
                "cargo".to_string(),
                json!({
                    "extra_args": [],
                    "extra_env": {},
                    "extra_test_binary_args": []
                }),
            );
            new_config.insert("overrides".to_string(), json!([]));
            new_config
        };

        let overrides = config
            .entry("overrides".to_string())
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("overrides is not an array"))?;

        overrides.push(Value::Object(override_entry));

        let json_string = serde_json::to_string_pretty(&config)?;
        fs::write(config_path, json_string)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn parses_command_channel_env_and_extra_args() {
        let parsed = OverrideManager::parse_override_args(&args(&[
            "@dx.serve",
            "+nightly",
            "RUST_LOG=debug",
            "--release",
        ]));
        assert_eq!(parsed.get("command").and_then(|v| v.as_str()), Some("dx"));
        assert_eq!(
            parsed.get("subcommand").and_then(|v| v.as_str()),
            Some("serve")
        );
        assert_eq!(
            parsed.get("channel").and_then(|v| v.as_str()),
            Some("nightly")
        );
        assert_eq!(
            parsed
                .get("extra_env")
                .and_then(|v| v.get("RUST_LOG"))
                .and_then(|v| v.as_str()),
            Some("debug")
        );
        assert_eq!(
            parsed
                .get("extra_args")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(1)
        );
    }

    #[test]
    fn hash_is_legacy_alias_for_test_binary_args() {
        let slash = OverrideManager::parse_override_args(&args(&["/--nocapture", "--exact"]));
        let hash = OverrideManager::parse_override_args(&args(&["#--nocapture", "--exact"]));
        assert_eq!(
            slash.get("extra_test_binary_args"),
            hash.get("extra_test_binary_args")
        );
        let bare_hash = OverrideManager::parse_override_args(&args(&["#", "--nocapture"]));
        assert_eq!(
            bare_hash
                .get("extra_test_binary_args")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>()),
            Some(vec!["--nocapture"])
        );
    }

    #[test]
    fn bang_bang_and_dash_remove_override() {
        let bang = OverrideManager::parse_override_args(&args(&["!!"]));
        let dash = OverrideManager::parse_override_args(&args(&["-"]));
        assert_eq!(bang.get("remove_override"), Some(&json!(true)));
        assert_eq!(dash.get("remove_override"), Some(&json!(true)));
    }

    #[test]
    fn bare_at_enables_append_mode() {
        let parsed = OverrideManager::parse_override_args(&args(&["@", "--release"]));
        assert_eq!(parsed.get("append"), Some(&json!(true)));
        assert!(parsed.get("extra_args").is_some());
    }

    #[test]
    fn legacy_bang_resets_map_to_remove_flags() {
        let parsed = OverrideManager::parse_override_args(&args(&["!env", "!#"]));
        assert_eq!(parsed.get("remove_env"), Some(&json!(true)));
        assert_eq!(parsed.get("remove_test_args"), Some(&json!(true)));
    }
}
