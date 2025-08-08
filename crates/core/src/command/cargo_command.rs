use std::io;
use std::process::{Command, ExitStatus};

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
        // Check if the first arg is a custom command
        let (executable, args) = if !self.args.is_empty() && !self.args[0].starts_with('-') && !self.args[0].starts_with('+') {
            // Check if this might be a custom command like "dx" or "trunk"
            match self.args[0].as_str() {
                "dx" | "trunk" => {
                    // Use the first arg as the command, rest as args
                    (self.args[0].as_str(), &self.args[1..])
                }
                _ => {
                    // Standard cargo/rustc command
                    let exec = match self.command_type {
                        CommandType::Cargo => "cargo",
                        CommandType::Rustc => "rustc",
                    };
                    (exec, &self.args[..])
                }
            }
        } else {
            // Standard command
            let exec = match self.command_type {
                CommandType::Cargo => "cargo",
                CommandType::Rustc => "rustc",
            };
            (exec, &self.args[..])
        };
        
        let mut cmd = String::from(executable);
        for arg in args {
            cmd.push(' ');
            if arg.contains(' ') {
                cmd.push_str(&format!("'{arg}'"));
            } else {
                cmd.push_str(arg);
            }
        }
        cmd
    }

    pub fn execute(&self) -> io::Result<ExitStatus> {
        // Check if the first arg is a custom command
        let (executable, args) = if !self.args.is_empty() && !self.args[0].starts_with('-') && !self.args[0].starts_with('+') {
            // Check if this might be a custom command like "dx" or "trunk"
            match self.args[0].as_str() {
                "dx" | "trunk" => {
                    // Use the first arg as the command, rest as args
                    (self.args[0].as_str(), &self.args[1..])
                }
                _ => {
                    // Standard cargo/rustc command
                    let exec = match self.command_type {
                        CommandType::Cargo => "cargo",
                        CommandType::Rustc => "rustc",
                    };
                    (exec, &self.args[..])
                }
            }
        } else {
            // Standard command
            let exec = match self.command_type {
                CommandType::Cargo => "cargo",
                CommandType::Rustc => "rustc",
            };
            (exec, &self.args[..])
        };
        
        let mut cmd = Command::new(executable);
        cmd.args(args);

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &self.env {
            eprintln!("Setting env: {}={}", key, value);
            cmd.env(key, value);
        }

        // Execute and wait for completion
        cmd.status()
    }
}
