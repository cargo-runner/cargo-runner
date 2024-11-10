use core::{CargoRunner, Error};
use anyhow::Result;

/// Use when you want to initialize a new config at `~/.cargo-runner/config.toml`
fn main() -> Result<(),Error> {
    let config = CargoRunner::init()?;
    println!("{:#?}", config);
    Ok(())
}
