use cargo_runner::models::Config;
use std::{fs, path::PathBuf};
use toml;

fn main() {
    
    let mut default_config = Config::init().unwrap_or_default();
    
    // Load the second config file
    let config_path = PathBuf::from("cargo-runner-dx1.toml");
    let config = if let Ok(content) = fs::read_to_string(&config_path) {
        toml::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };

    println!("loading data from cargo-runner-dx1.toml");
    println!("{:#?}", config);
 
    println!("loading default config");
    println!("{:#?}", default_config);

    default_config.merge(config);

    println!("final merged config");
    println!("{:#?}", default_config);

    let toml_string = toml::to_string_pretty(&default_config)
        .expect("Failed to serialize config to TOML");

    // Write to output.toml
    fs::write("output.toml", toml_string)
        .expect("Failed to write to output.toml");

    println!("Config has been written to output.toml");

}