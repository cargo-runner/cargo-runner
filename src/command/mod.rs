pub mod builder;
pub mod fallback;

use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Cargo,
    Rustc,
}

#[derive(Debug, Clone)]
pub struct CargoCommand {
    pub command_type: CommandType,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: Vec<(String, String)>,
}

impl CargoCommand {
    pub fn new(args: Vec<String>) -> Self {
        Self {
            command_type: CommandType::Cargo,
            args,
            working_dir: None,
            env: Vec::new(),
        }
    }
    
    pub fn new_rustc(args: Vec<String>) -> Self {
        Self {
            command_type: CommandType::Rustc,
            args,
            working_dir: None,
            env: Vec::new(),
        }
    }

    pub fn with_working_dir(mut self, dir: String) -> Self {
        self.working_dir = Some(dir);
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.push((key, value));
        self
    }

    pub fn to_shell_command(&self) -> String {
        let executable = match self.command_type {
            CommandType::Cargo => "cargo",
            CommandType::Rustc => "rustc",
        };
        let mut cmd = String::from(executable);
        for arg in &self.args {
            cmd.push(' ');
            if arg.contains(' ') {
                cmd.push_str(&format!("'{}'", arg));
            } else {
                cmd.push_str(arg);
            }
        }
        cmd
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Lib,
    Bin(String),
    Example(String),
    Test(String),
    Bench(String),
}

impl Target {
    pub fn from_file_path(file_path: &Path) -> Option<Self> {
        let path_str = file_path.to_str()?;

        if path_str.contains("/src/bin/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Bin(name))
        } else if path_str.contains("/examples/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Example(name))
        } else if path_str.contains("/tests/") && !path_str.ends_with("/mod.rs") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Test(name))
        } else if path_str.contains("/benches/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Bench(name))
        } else if path_str.ends_with("/src/lib.rs") {
            Some(Target::Lib)
        } else if path_str.ends_with("/src/main.rs") {
            Some(Target::Bin("main".to_string()))
        } else if path_str.contains("/src/") && !path_str.contains("/src/bin/") {
            // Any other file under src/ is part of the library
            Some(Target::Lib)
        } else {
            None
        }
    }
}
