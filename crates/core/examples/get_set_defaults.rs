use core::{CargoRunner, Config, Context, Error};
use std::path::PathBuf;

fn main()-> anyhow::Result<(),Error> {
    let path = PathBuf::from("cargo-runner-leptos.toml");

    let mut config = CargoRunner::load(path.clone())?;

    config.merge(CargoRunner::default())?;

    let default = config.get_default(Context::Run);

    println!(
        "previous default for run context: {:#?}",
        default.unwrap_or_default()
    );

    config.set_default(Context::Run, "leptos")?;

    let default = config.get_default(Context::Run);

    println!(
        "latest default for run context: {:#?}",
        default.unwrap_or_default()
    );

    config.save(Some(&path))?;

    Ok(())
}
