use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Cargo,
    Rustc,
    Shell,        // For dx, trunk, and other shell commands
    RustSFScript, // For cargo script test execution
}

#[derive(Debug, Clone)]
pub struct CargoCommand {
    pub command_type: CommandType,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: Vec<(String, String)>,
    /// For rustc test commands, the test name to filter
    pub test_filter: Option<String>,
}

impl CargoCommand {
    pub fn new(args: Vec<String>) -> Self {
        Self {
            command_type: CommandType::Cargo,
            args,
            working_dir: None,
            env: Vec::new(),
            test_filter: None,
        }
    }

    pub fn new_rustc(args: Vec<String>) -> Self {
        Self {
            command_type: CommandType::Rustc,
            args,
            working_dir: None,
            env: Vec::new(),
            test_filter: None,
        }
    }

    pub fn new_shell(command: String, args: Vec<String>) -> Self {
        let mut all_args = vec![command];
        all_args.extend(args);
        Self {
            command_type: CommandType::Shell,
            args: all_args,
            working_dir: None,
            env: Vec::new(),
            test_filter: None,
        }
    }

    pub fn new_rust_sf_script(args: Vec<String>) -> Self {
        Self {
            command_type: CommandType::RustSFScript,
            args,
            working_dir: None,
            env: Vec::new(),
            test_filter: None,
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

    pub fn with_test_filter(mut self, filter: String) -> Self {
        self.test_filter = Some(filter);
        self
    }

    pub fn to_shell_command(&self) -> String {
        match self.command_type {
            CommandType::Rustc => {
                let mut cmd = String::from("rustc");
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }

                // Extract output name and append run command
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        cmd.push_str(&format!(" && ./{}", self.args[i + 1]));

                        // If this is a test command with a filter, add it
                        if self.args.contains(&"--test".to_string()) {
                            if let Some(ref test_filter) = self.test_filter {
                                cmd.push_str(&format!(" {}", test_filter));
                            }
                        }
                        break;
                    }
                }
                cmd
            }
            CommandType::Shell => {
                // For shell commands, first arg is the command itself
                if self.args.is_empty() {
                    String::new()
                } else {
                    let mut cmd = String::new();
                    for (i, arg) in self.args.iter().enumerate() {
                        if i > 0 {
                            cmd.push(' ');
                        }
                        if arg.contains(' ') {
                            cmd.push_str(&format!("'{arg}'"));
                        } else {
                            cmd.push_str(arg);
                        }
                    }
                    cmd
                }
            }
            CommandType::RustSFScript | CommandType::Cargo => {
                let mut cmd = String::from("cargo");
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }
                cmd
            }
        }
    }

    pub fn execute(&self) -> io::Result<ExitStatus> {
        match self.command_type {
            CommandType::Rustc => {
                // Extract the output filename from args (after -o flag)
                let mut output_name = None;
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        output_name = Some(&self.args[i + 1]);
                        break;
                    }
                }

                // First compile with rustc
                let mut rustc_cmd = Command::new("rustc");
                rustc_cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    rustc_cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    eprintln!("Setting env: {}={}", key, value);
                    rustc_cmd.env(key, value);
                }

                // Compile
                let compile_status = rustc_cmd.status()?;
                if !compile_status.success() {
                    return Ok(compile_status);
                }

                // If compilation succeeded and we have an output name, run it
                if let Some(output) = output_name {
                    eprintln!("Running: ./{}", output);
                    let mut run_cmd = Command::new(format!("./{}", output));

                    // If this is a test command with a filter, add it as argument
                    if self.args.contains(&"--test".to_string()) {
                        if let Some(ref test_filter) = self.test_filter {
                            run_cmd.arg(test_filter);
                        }
                    }

                    // Set working directory if specified
                    if let Some(ref dir) = self.working_dir {
                        run_cmd.current_dir(dir);
                    }

                    // Set environment variables
                    for (key, value) in &self.env {
                        run_cmd.env(key, value);
                    }

                    run_cmd.status()
                } else {
                    Ok(compile_status)
                }
            }
            CommandType::Shell => {
                // For shell commands, first arg is the command
                if self.args.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "No command specified",
                    ));
                }

                let mut cmd = Command::new(&self.args[0]);
                if self.args.len() > 1 {
                    cmd.args(&self.args[1..]);
                }

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    eprintln!("Setting env: {}={}", key, value);
                    cmd.env(key, value);
                }

                cmd.status()
            }
            CommandType::RustSFScript | CommandType::Cargo => {
                let mut cmd = Command::new("cargo");
                cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    eprintln!("Setting env: {}={}", key, value);
                    cmd.env(key, value);
                }

                cmd.status()
            }
        }
    }
}
