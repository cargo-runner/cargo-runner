use anyhow::{anyhow, Result};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::warn;

use crate::Error;

use super::{CommandType, Config, Context};

pub type ConfigKey = String;

pub type DefaultContext = Option<String>;

pub type ListOfConfig = Option<Vec<Config>>;

pub type ConfigVal = (DefaultContext, ListOfConfig);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoRunner(pub HashMap<ConfigKey, ConfigVal>);

impl Serialize for CargoRunner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, (default, commands)) in &self.0 {
            #[derive(Serialize)]
            struct CommandEntry<'a> {
                default: Option<&'a String>,
                config: Option<&'a Vec<Config>>,
            }

            map.serialize_entry(
                key,
                &CommandEntry {
                    default: default.as_ref(),
                    config: commands.as_ref(),
                },
            )?;
        }

        map.end()
    }
}

// Custom deserialization implementation
impl<'de> Deserialize<'de> for CargoRunner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CommandEntry {
            default: Option<String>,
            config: Option<Vec<Config>>,
        }

        let map = HashMap::<String, CommandEntry>::deserialize(deserializer)?;

        let converted = map
            .into_iter()
            .map(|(k, v)| (k, (v.default, v.config)))
            .collect();

        Ok(CargoRunner(converted))
    }
}

impl Default for CargoRunner {
    fn default() -> Self {
        let mut commands = HashMap::new();

        commands.insert("run".to_string(), Self::default_configs("run"));
        commands.insert("test".to_string(), Self::default_configs("test"));
        commands.insert("build".to_string(), Self::default_configs("build"));
        commands.insert("bench".to_string(), Self::default_configs("bench"));

        CargoRunner(commands)
    }
}

impl TryFrom<String> for CargoRunner {
    type Error = Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        toml::from_str(&value).map_err(Error::Deserialize)
    }
}

impl TryFrom<&str> for CargoRunner {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        toml::from_str(value).map_err(Error::Deserialize)
    }
}

impl CargoRunner {
    fn default_configs(sub_command: &str) -> (DefaultContext, ListOfConfig) {
        let config = Config {
            name: "default".to_string(),
            command_type: Some(CommandType::Cargo),
            command: Some("cargo".to_string()),
            sub_command: Some(sub_command.to_string()),
            allowed_subcommands: Some(vec![]),
            env: Some(HashMap::new()),
        };
        (Some("default".to_string()), Some(vec![config]))
    }

    pub fn set_default(&mut self, context: Context, name: &str) -> Result<(), Error> {
        if let Some((_, configs)) = self.0.get(context.into()) {
            if let Some(configs) = configs {
                if configs.iter().any(|c| c.name == name) {
                    self.0.insert(
                        context.into(),
                        (Some(name.to_string()), Some(configs.clone())),
                    );
                    return Ok(());
                }
            }
        }
        Err(Error::SetDefault(context))
    }

    pub fn get_default(&self, context: Context) -> Option<&str> {
        self.0
            .get(context.into())
            .and_then(|(default, _)| default.as_ref())
            .map(|s| s.as_str())
    }

    fn get_default_config_path() -> Result<PathBuf, Error> {
        Ok(dirs::home_dir()
            .ok_or(Error::Other(anyhow!("Could not find home directory")))?
            .join(".cargo-runner")
            .join("config.toml"))
    }
}

impl CargoRunner {
    pub fn collect(&self, config_name: &str) -> Option<CargoRunner> {
        let mut found_configs = HashMap::new();

        for (context, (_, configs)) in &self.0 {
            if let Some(configs_vec) = configs {
                let matching_configs: Vec<Config> = configs_vec
                    .iter()
                    .filter(|c| c.name == config_name)
                    .cloned() // Clone the Config as we return owned values
                    .collect();

                if !matching_configs.is_empty() {
                    // Set the default to the config_name since it's now relevant
                    found_configs.insert(
                        context.clone(),
                        (Some(config_name.to_string()), Some(matching_configs)),
                    );
                }
            }
        }

        match found_configs.is_empty() {
            true => None,
            false => Some(CargoRunner(found_configs)),
        }
    }

    pub fn find(&self, context: Context, config_name: &str) -> Option<&Config> {
        self.0.get(context.into()).and_then(|(_, configs)| {
            configs
                .as_ref()
                .and_then(|configs_vec| configs_vec.iter().find(|c| c.name == config_name))
        })
    }
}

impl CargoRunner {
    pub fn init() -> Result<CargoRunner, Error> {
        let home =
            dirs::home_dir().ok_or(Error::Other(anyhow!("Could not find home directory")))?;

        let config_dir = home.join(".cargo-runner");
        let config_path = Self::get_default_config_path()?;

        fs::create_dir_all(&config_dir).map_err(|e| Error::Io(e))?;

        CargoRunner::load(config_path)
    }

    pub fn reset() -> Result<(), Error> {
        let config_path = Self::get_default_config_path()?;

        let default_config = Self::default();

        Self::create_backup(&config_path);

        fs::write(
            &config_path,
            toml::to_string_pretty(&default_config).map_err(|e| Error::Serialize(e))?,
        )
        .map_err(|e| Error::Io(e))?;

        Ok(())
    }
}

impl CargoRunner {
    pub fn pluck(&mut self, config_name: &str) -> CargoRunner {
        let mut removed_configs = HashMap::new();

        // Iterate over each context to apply the changes
        self.0.retain(|context, (default, configs)| {
            let Some(configs_vec) = configs else {
                // Retain context if no configurations are present
                return true;
            };
            let matching_configs: Vec<Config> = configs_vec
                .iter()
                .filter(|c| c.name == config_name)
                .cloned()
                .collect();

            let remaining_configs: Vec<Config> = configs_vec
                .iter()
                .filter(|c| c.name != config_name)
                .cloned()
                .collect();

            if !matching_configs.is_empty() {
                // Add matching configs to the output with `config_name` as the default
                removed_configs.insert(
                    context.clone(),
                    (
                        Some(config_name.to_string()),
                        Some(matching_configs.to_vec()),
                    ),
                );

                if remaining_configs.is_empty() {
                    // Remove the context if there are no remaining configs
                    *default = Some("default".to_string());
                    false
                } else {
                    // Retain the context with remaining configs and reset default if needed
                    *configs = Some(remaining_configs.to_vec());
                    if default.as_deref() == Some(config_name) {
                        *default = Some("default".to_string());
                    }
                    true
                }
            } else {
                true
            }
        });

        CargoRunner(removed_configs)
    }

    pub async fn download(url: &str, save_path: Option<PathBuf>) -> Result<(), Error> {
        let response = reqwest::get(url).await?;
        let content = response.text().await?;
        let mut config: CargoRunner = toml::from_str(&content)?;

        if let Some(path) = save_path {
            if path.exists() {
                let existing_content = fs::read_to_string(&path)?;
                let mut existing_config: CargoRunner = toml::from_str(&existing_content)?;

                existing_config.merge(config.clone())?;

                config = existing_config;
            }

            fs::create_dir_all(path.parent().unwrap())?;

            let toml_content = toml::to_string_pretty(&config)?;

            fs::write(&path, toml_content)?;
        } else {
            let mut default_config = Self::default();

            default_config.merge(config.clone())?;

            let config_path = Self::get_default_config_path()?;

            Self::create_backup(&config_path);

            let toml = toml::to_string_pretty(&default_config).map_err(|e| Error::Serialize(e))?;

            fs::write(config_path, toml).map_err(|e| Error::Io(e))?;
        }

        Ok(())
    }
}
impl CargoRunner {
    pub fn save(&self, file_path: Option<&PathBuf>) -> Result<(), Error> {
        // Determine the path to save the file
        let path_to_save = match file_path {
            Some(path) => path.clone(),
            None => Self::get_default_config_path()?,  // Assuming this is a function that returns a default path
        };

        // Serialize the CargoRunner struct to TOML format
        let toml_content = toml::to_string_pretty(&self).map_err(|e| Error::Serialize(e))?;

        // Write the serialized content to the file
        fs::write(&path_to_save, toml_content).map_err(|e| Error::Io(e))?;

        Ok(())
    }

    pub fn load(path: PathBuf) -> Result<CargoRunner, Error> {
        match fs::read_to_string(&path) {
            Ok(data) => match toml::from_str(&data) {
                Ok(config) => Ok(config),
                Err(_) => {
                    warn!(
                        "Failed to parse config file from: {} , creating a backup",
                        path.display()
                    );

                    Self::create_backup(&path);

                    let default_config = Self::default();

                    let toml =
                        toml::to_string_pretty(&default_config).map_err(|e| Error::Serialize(e))?;

                    fs::write(&path, toml).map_err(|e| Error::Io(e))?;

                    Ok(default_config)
                }
            },

            Err(_) => {
                warn!("Failed to read config path: {}", path.display());

                Self::create_backup(&path);

                let default_config = Self::default();

                let toml =
                    toml::to_string_pretty(&default_config).map_err(|e| Error::Serialize(e))?;

                fs::write(&path, toml).map_err(|e| Error::Io(e))?;

                Ok(default_config)
            }
        }
    }

    pub fn merge(&mut self, other: CargoRunner) -> Result<()> {
        for (command_type, (other_default, other_configs)) in other.0 {
            let (base_default, base_configs) = self
                .0
                .entry(command_type.clone())
                .or_insert_with(|| (None, Some(Vec::new())));

            if let Some(other_configs) = other_configs {
                let base = base_configs.get_or_insert_with(Vec::new);
                let mut existing_names: HashSet<_> = base.iter().map(|c| c.name.clone()).collect();

                for other_config in other_configs {
                    if existing_names.insert(other_config.name.clone()) {
                        base.push(other_config);
                    } else if let Some(existing) =
                        base.iter_mut().find(|c| c.name == other_config.name)
                    {
                        existing.merge(&other_config)?;
                    }
                }
            }

            if let Some(ref new_default) = other_default {
                if base_configs.as_ref().map_or(false, |base| {
                    base.iter().any(|cmd| cmd.name == *new_default)
                }) {
                    *base_default = Some(new_default.clone());
                } else {
                    return Err(anyhow!(
                        "Default command '{}' does not exist in the '{}' commands.",
                        new_default,
                        command_type
                    ));
                }
            }
        }

        Ok(())
    }

    fn create_backup(config_path: &PathBuf) {
        let backup_path_with_index = config_path.with_extension(""); // Start with the original path without extension
        let mut index = 0; // Start with 0

        // Check if the backup file already exists and append an index if it does
        loop {
            let backup_file_name = format!(
                "{}.{}.bak",
                backup_path_with_index
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                index
            );
            let backup_path = backup_path_with_index.with_file_name(backup_file_name);

            if !backup_path.exists() {
                // Copy the original config file to the backup path
                match fs::copy(config_path, &backup_path) {
                    Ok(_) => {
                        println!("Backup created at: {}", backup_path.display());
                        break; // Exit the loop after creating the backup
                    }
                    Err(e) => {
                        eprintln!("Failed to create backup of the config file: {}", e);
                        break; // Exit the loop on error
                    }
                }
            }
            index += 1; // Increment index for the next backup name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_commands() {
        let config = CargoRunner::default();

        // Test run command default
        assert_eq!(config.get_default("run".into()), Some("default"));

        // Test setting new default
        let mut config = CargoRunner::default();
        config
            .0
            .get_mut("run")
            .unwrap()
            .1
            .as_mut()
            .unwrap()
            .push(Config {
                name: "dx".to_string(),
                command_type: Some(CommandType::Shell),
                command: Some("dx".to_string()),
                sub_command: Some("serve".to_string()),
                allowed_subcommands: Some(vec![]),
                env: Some(HashMap::new()),
            });

        assert!(config.set_default(Context::Run, "dx").is_ok());
        assert_eq!(config.get_default("run".into()), Some("dx"));
    }

    #[test]
    fn test_parse_dx_config() {
        let dx_content = r#"
        [run]
        default = "dx"
        [[run.config]]
        name = "dx"
        command_type = "shell"
        command = "dx"
        sub_command = "serve"
        allowed_subcommands = ["build", "serve"]
        "#;

        let config: CargoRunner = toml::from_str(dx_content).expect("Failed to parse dx config");

        let (default, run_configs) = config.0.get("run").expect("Run config should exist");
        let run_configs = run_configs.as_ref().expect("Run config should have values");

        assert_eq!(run_configs.len(), 1);
        assert_eq!(default.as_ref().map(String::as_str), Some("dx"));

        let dx_config = &run_configs[0];
        assert_eq!(dx_config.name, "dx");
        assert_eq!(dx_config.command, Some("dx".to_string()));
        assert_eq!(dx_config.sub_command, Some("serve".to_string()));
        assert!(matches!(dx_config.command_type, Some(CommandType::Shell)));

        assert!(config.0.get("test").is_none());
        assert!(config.0.get("build").is_none());
        assert!(config.0.get("bench").is_none());
    }

    #[test]
    fn test_merge_configs() {
        let mut base_config = CargoRunner::default();

        let dx_content = r#"
        [run]
        default = "dx"
        [[run.config]]
        name = "dx"
        command_type = "shell"
        command = "dx"
        sub_command = "serve"
        allowed_subcommands = ["build", "serve"]
        "#;

        let dx_config: CargoRunner = toml::from_str(dx_content).expect("Failed to parse dx config");

        base_config.merge(dx_config).unwrap();

        let (default, run_configs) = base_config.0.get("run").expect("Run config should exist");
        let run_configs = run_configs.as_ref().expect("Run config should have values");

        assert_eq!(run_configs.len(), 2);
        assert_eq!(default.as_ref().map(String::as_str), Some("dx"));

        let dx_config = run_configs
            .iter()
            .find(|c| c.name == "dx")
            .expect("dx config should exist");

        assert_eq!(dx_config.command, Some("dx".to_string()));
        assert_eq!(dx_config.sub_command, Some("serve".to_string()));
        assert!(matches!(dx_config.command_type, Some(CommandType::Shell)));

        let default_config = run_configs
            .iter()
            .find(|c| c.name == "default")
            .expect("default config should exist");

        assert_eq!(default_config.command, Some("cargo".to_string()));
        assert_eq!(default_config.sub_command, Some("run".to_string()));
        assert_eq!(default_config.command_type, Some(CommandType::Cargo));
    }
}
