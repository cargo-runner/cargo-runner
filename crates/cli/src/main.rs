use core::CargoRunner;
use std::path::PathBuf;
use anyhow::Result;
fn main() -> Result<()> {
    let _default = CargoRunner::init()?;
    let config_path  = PathBuf::from("cargo-runner-leptos.toml");
    let config = CargoRunner::load(config_path);
    println!("{:#?}", config);
    Ok(())
}
