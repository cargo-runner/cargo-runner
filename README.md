# Cargo Runner

> cargo runner is an extensible tool to help you quickly run , build, test and bench rust code. It fills the gap that is missing when using **rust-analyzer** override command config, also it is easy to add , remove, other config from other frameworks, that can be re-used across different projects and cargo workspaces.



NOTE: If you wanted to add your custom **default configs** in the **built-in** default config please can submit a PR.




## Techstacks

- TypeScript
- Rust
- WASM
- WIT (WebAssembly Interface Types)


## Installation

### Install Cargo Runner Cli

To Install the Cli you can run on your terminal  `cargo install cargo-runner-cli` or `cargo install --git https://github.com/cargo-runner/cli`

> You can use the stand alone cli called `rx` or as cargo subcommand `cargo runner` to run the commands. 

### Install Cargo Runner as Vscode Extension
Download on vscode marketplace [here](https://marketplace.visualstudio.com/items?itemName=masterustacean.cargo-runner) as vsix file. or install it from command line using `code --install-extension rx.vsix` or go to vscode extensions and search for `cargo runner` and install it.

> For other text editor, you can use the `rx` cli or `cargo runner`  E.g. when using neovim , you bind a custom keymap e.g. <kbd>CMD</kbd> + <kbd>R</kbd> to invoke `cargo runner exec $filepath` 


### Core Lib

#### Cargo Runner Module
Use it as a library to make use of `CargoRunner` to **init** , **load** , **merge** , **generate** , **set_default** or **get_default** and **validate** configs.

<details>
<summary>Init Config</summary>

```rust
use core::{CargoRunner, Error};
use anyhow::Result;

/// Use when you want to initialize a new config at `~/.cargo-runner/config.toml`
fn main() -> Result<(),Error> {
    let config = CargoRunner::init()?;
    println!("{:#?}", config);
    Ok(())
}
```
</details>

<details>
<summary>Load Config</summary>

```rust
use core::CargoRunner;
use std::path::PathBuf;

/// Use when you want to load a specific config from a given path
fn main() {
    let path = PathBuf::from("cargo-runner-leptos.toml");
    let config = CargoRunner::load(path);
    println!("{:#?}", config);
}
```

</details>

<details>
<summary>Merge Config</summary>

```rust
use core::{CargoRunner, Context, Error};
use std::path::PathBuf;
use anyhow::Result;

/// Use when you want to merge a specific config and override it with another config
fn main() -> Result<(),Error> {
    let mut config = CargoRunner::default();

    let path = PathBuf::from("cargo-runner-leptos.toml");

    let leptos_config = CargoRunner::load(path)?;

    config.merge(leptos_config)?;

    let default = config.get_default(Context::Run);

    println!(
        "run default command config is set to: {:#?}",
        default.unwrap_or_default()
    );

    println!("{:#?}", config);
    config.save(None)?;
    Ok(())
}
```

</details>


<details>
<summary>Get and Set Default Config</summary>

```rust
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
```

</details>

<details>
<summary>Find Config by Context</summary>

```rust
use core::{CargoRunner, Context, Error};
use std::path::PathBuf;
use anyhow::Result;

/// Use when you want to find a specific config for a given context
fn main()-> Result<(),Error> {
    let mut config = CargoRunner::default();
    let path = PathBuf::from("example-leptos.toml");
    let  leptos = CargoRunner::load(path)?;
    {
        config.merge(leptos)?;
    }

    let default = config.find(Context::Run,"leptos");

    println!("{:#?}", default);
    Ok(())
}
```

</details>

<details>
<summary>Pluck Config by Name</summary>

```rust
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
```

</details>

<details>
<summary>Reset Config</summary>

```rust
use core::{CargoRunner, Error};

/// Use when the default config becomes polluted and wanna start fresh
/// This would backup the current default config
/// to a filename with format `config.$number.bak` 
/// Then replace the old config with the default config
fn main()-> anyhow::Result<(),Error> {
    CargoRunner::reset()?;
    Ok(())
}
```

</details>


<br>

TODO: 

<details>
<summary>Generate Config</summary>

```rust
use core::Config;

fn main() {
    todo!()
}
```

</details>

<details>
<summary>Download Config</summary>

```rust
use core::{CargoRunner, Error};
use anyhow::Result;

/// Download allows you to download a config from a url
/// If No save_path is provided it would save the config to the default config path
/// And would merge the downloaded config with the default config
#[tokio::main]
async fn main() -> Result<(), Error> {
    let url = "https://gist.githubusercontent.com/codeitlikemiley/26205a6d642c33dbdcf9fc85b79f29bf/raw/a59d51136aca2fed51ca45de6b2319039e977637/leptos.toml";
    CargoRunner::download(url,None).await?;
    Ok(())
}
```

- Download to a specified path
```rust
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
```

</details>


<details>
<summary>Validate Config</summary>

```rust
use core::Config;

fn main() {
    todo!()
}
```

</details>

#### Context Finder Module

This module will help what context to use when generating commands or overriding parameters.

This would use `filepath` , `line no` and `column no` to determine the context.

This would make use of `Cargo.toml` file to determine the context , in cases **bin** or **lib** is used and other important metadata like **features** , **workspace** etc.

This would also use some known patterns to determine the context. e.g. special filename like `main.rs` or `lib.rs`, directory name like `src`, `tests`, `benches`, `src/bin/`,
scope of the module, or function.

special patterns like when using different type of comments 
like /// or /** **/ or using special macro for doc test etc.

There will be cases that there are multiple possible contexts inside a file , but needs to be narrowed down to one context.

Once we have the **context** it is easy to execute the command.


#### Command Builder Module

Note: This can be use to build commands or used as a library to build commands .e.g it is used on `rx exec` command.

<details>
<summary>Build Command</summary>

```rust
use core::Config;

fn main() {
    todo!()
}
```

</details>


#### Executor  Module

Note: This can be used to execute commands or used as a library to execute commands .e.g it is used on `rx exec` command.

> Note: You can use this library to build your own plugin using **CargoRunner** modules with **Config** , **CommandBuilder** and **Executor**.

<details>
<summary>Execute Command</summary>

```rust
use core::Config;

fn main() {
    todo!()
}
```

Note: This would read the file , and current position if --ln and --col are provided, it would use that to determine the current context. and would use the nearest `cargo-runner.toml` file near `Cargo.toml` to generate the commands. It would use as well the `Cargo.toml` file to add other metadata like `package` name or `bin` name or `features` when generating commands.

</details>

### Cargo Runner CLI

#### a. Config manager


<details>
<summary>Generate Default config on <em>~/.cargo-runner/config.toml</em></summary>

```sh
rx init
```

</details>

<details>
<summary> Download pre-built config made by others </summary>

```sh
rx download https://github.com/cargo-runner/configs/raw/main/leptos/leptos.toml
# you can also download a config and set it as default config for specific context
rx download https://github.com/cargo-runner/configs/raw/main/leptos/leptos.toml --default run
```

</details>

<details>
<summary> Generate config</summary>

```sh
# pass an optional name 
rx generate
# if you pass the name it would generate a config for the given name if it exists
rx generate leptos
# by default it would be generated on current working directory
# if we want to generate on a different dir we can pass --dir
rx generate --dir ~/.cargo-runner/configs/leptos leptos
# to download a config and generate it on current working directory
rx generate --url https://github.com/cargo-runner/configs/raw/main/leptos/leptos.toml
# if you want to download and set different dir name you can also pass --dir
rx generate --url https://github.com/cargo-runner/configs/raw/main/leptos/leptos.toml --dir ~/.cargo-runner/configs/leptos
```

[example-override.toml](./cargo-runner-leptos.toml) generated 

```toml
[run]
default = "leptos"

[[run.commands]]
name = "leptos"
command_type = "sub_command"
command = "leptos"
sub_command = "watch"
allowed_subcommands = []

[run.commands.env]
```

</details>


<details>
<summary>Validate custom config , auto-generate backup if invalid format is used</summary>

```sh
# if you dont pass in a path it would assume it is on current working directory
rx validate
# you can pass a config file path
rx validate ~/.cargo-runner/configs/leptos/leptos.toml
# if you need to validate default config you can pass --default
rx validate --default
```

Invalid config would move the file to e.g. `$name.0.bak` and a valid config would be generated for you to modify.


</details>

<details>
<summary> Change Default for Specific Context </summary>

```sh
# the name params is optional
rx default run
# if the name if provided it would check if the context exists
# before setting it as default , if it doesnt exist nothing would happen
# and an error would be shown
rx default run leptos
```

</details>

<details>

<summary>Override Parameters for a specific context</summary>

```sh
rx params $context --path $path
# example
rx params run --path /Users/uriah/oss/rx/crates/cli/src/main.rs
```

Note: The file path here would be used to determine where to look for the `cargo-runner.toml` file, and update the `default` context with the parameters.


</details>


#### b. Cargo Runner Exec


```sh
rx exec $filepath --ln $ln --col $col
```

Note: This would use both `Config` and `CommandBuilder` **modules** to `generate` the correct **command**  and execute that command.


## VsCode Extension (Cargo Runner)

### Features

#### a. Config Builder on Vscode

- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and type `Cargo Runner: Init Config`  to Generate Default `Config` on `~/.cargo-runner/config.toml`
- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and type `Cargo Runner: Generate Config`  to Generate a `CommandConfig`. You can either type a name or pass a url to download a config.
- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and type `Cargo Runner: Download Config` to Download a `CommandConfig` from a url.
- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and type `Cargo Runner: Set Default Context` to choose a context from list: `run` , `build`, `test`, and `bench`.
- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>P</kbd> and type `Cargo Runner: Validate Config` to Validate a `CommandConfig` file.

Note: The default directory when running any of the commands above would either be the nearest `Cargo.toml` file on the current open file or workspace root.

#### b. Cargo Builder on Vscode
- Uses your default `config` or `cargo-runner.toml` file to generate commands for all `command_type` variants.
- Auto appends or prepends `sub_command, parameters or options` for **command_type** variants of  `cargo` or `sub_command` when executing a command.
- Reads your `Cargo.toml` file get other metadata like `package` name or `bin` name or `features` when generating commands.

#### c. Cargo Runner on Vscode

- Press <kbd>CMD</kbd> + <kbd>R</kbd> (default) or bind to a key of your choice to execute any command.

Note: Prior Executing commands , it would check for the current context e.g. the open file is **main.rs** it would do further check if the context is either `run` or `test`. Once the context is found it would use the `default` **CommandConfig** for that current context , and build the correct command to execute.

> Under the hood it uses the CargoBuilder to smartly build the command to execute or override the parameters.

#### d. Override Parameters on default Context on VSCode

- Press <kbd>CMD</kbd> + <kbd>SHIFT</kbd> + <kbd>R</kbd> to `Cargo Runner: Override Parameters` 

Note: This would only work if a rust file is open, and would look for current context e.g. the open file is **main.rs** it would do further check if the context is either `run` or `test`. It would update the `cargo-runner.toml` next to `Cargo.toml` using the current context , it would update the current context `default` **CommandConfig** 



