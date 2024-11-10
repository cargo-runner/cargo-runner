use core::{CargoRunner, Error};
use std::path::PathBuf;
use anyhow::Result;

/// Use when you need to pluck all config with same name 
/// on different context, does providing you a new [CargRunner] instance
/// that has that **config_name** available to any context.
/// e.g. when you want to pluck only the **leptos** config and remove other configs.
/// prior merging to other configs.
/// It also set all  default for any context that matches the **config_name**
fn main() -> Result<(), Error> {
    let mut config = CargoRunner::default();
    let path = PathBuf::from("example-leptos.toml");
    let  leptos = CargoRunner::load(path)?;
    {
        config.merge(leptos)?;
    }

    let default = config.pluck("leptos");

    println!("{:#?}", default);
    config.save(None)?;
    Ok(())
}
