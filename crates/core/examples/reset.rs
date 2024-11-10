use core::{CargoRunner, Error};

/// Use when the default config becomes polluted and wanna start fresh
/// This would backup the current default config
/// to a filename with format `config.$number.bak` 
/// Then replace the old config with the default config
fn main()-> anyhow::Result<(),Error> {
    CargoRunner::reset()?;
    Ok(())
}