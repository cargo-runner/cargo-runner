use core::{CargoRunner, Error};
use std::path::PathBuf;
use anyhow::Result;

/// Download allows you to download a config from a url
/// If No save_path is provided it would save the config to the default config path
/// And would merge the downloaded config with the default config
/// If a save_path is provided it would save the config to the specified path
#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "https://gist.githubusercontent.com/codeitlikemiley/26205a6d642c33dbdcf9fc85b79f29bf/raw/a59d51136aca2fed51ca45de6b2319039e977637/leptos.toml";
    let save_path = Some(PathBuf::from("example-downloaded.toml"));
    CargoRunner::download(url,save_path).await?;
    Ok(())
}