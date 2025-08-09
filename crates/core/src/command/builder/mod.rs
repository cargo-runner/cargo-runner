//! Clean API for command building with encapsulated config resolution

mod cargo;
mod config_access;
mod config_resolver;
mod rustc;

pub use self::cargo::{
    BenchmarkCommandBuilder, BinaryCommandBuilder, DocTestCommandBuilder, ModuleTestCommandBuilder,
    TestCommandBuilder,
};
pub use self::config_access::ConfigAccess;
pub use self::config_resolver::ConfigResolver;
pub use self::rustc::{RustcCommandBuilder, SingleFileScriptBuilder};

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    types::{FileType, FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;

/// Main entry point for building commands
///
/// # Example
/// ```
/// let runnable = /* ... */;
/// let command = CommandBuilder::for_runnable(&runnable)
///     .with_package("my-package")
///     .with_project_root("/path/to/project")
///     .build()?;
/// ```
pub struct CommandBuilder<'a> {
    runnable: &'a Runnable,
    package_name: Option<String>,
    project_root: Option<&'a Path>,
    config_override: Option<Config>,
}

impl<'a> CommandBuilder<'a> {
    /// Create a new command builder for a runnable
    pub fn for_runnable(runnable: &'a Runnable) -> Self {
        Self {
            runnable,
            package_name: None,
            project_root: None,
            config_override: None,
        }
    }

    /// Set the package name
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package_name = Some(package.into());
        self
    }

    /// Set the project root
    pub fn with_project_root(mut self, root: &'a Path) -> Self {
        self.project_root = Some(root);
        self
    }

    /// Override the configuration (for testing or special cases)
    pub fn with_config(mut self, config: Config) -> Self {
        self.config_override = Some(config);
        self
    }

    /// Check if a file is a cargo script file
    fn is_cargo_script_file(&self, file_path: &Path) -> Result<bool> {
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Some(first_line) = content.lines().next() {
                    return Ok(first_line.starts_with("#!")
                        && first_line.contains("cargo")
                        && first_line.contains("-Zscript"));
                }
            }
        }
        Ok(false)
    }

    /// Check if a file is a standalone file (has main() and not part of a Cargo project)
    fn is_standalone_file(&self, file_path: &Path) -> bool {
        // First check if file has a main function
        let has_main = if let Ok(content) = std::fs::read_to_string(file_path) {
            content.contains("fn main(") || content.contains("fn main (")
        } else {
            return false; // Can't read file, not standalone
        };

        if !has_main {
            return false; // No main function, not standalone
        }

        // Check if file is part of a Cargo project
        let cargo_root = file_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists());

        match cargo_root {
            None => true, // No Cargo.toml found, definitely standalone
            Some(root) => {
                // Check if the file is in a standard Cargo source location
                if let Ok(relative) = file_path.strip_prefix(root) {
                    let path_str = relative.to_str().unwrap_or("");

                    // Check standard binary locations
                    if path_str == "src/main.rs"
                        || path_str.starts_with("src/bin/")
                        || path_str.starts_with("examples/")
                    {
                        return false; // In standard location, not standalone
                    }

                    // Check if it's listed in Cargo.toml
                    if let Ok(manifest) = std::fs::read_to_string(root.join("Cargo.toml")) {
                        // Simple check for [[bin]] entries
                        if manifest.contains("[[bin]]") && manifest.contains(&path_str) {
                            return false; // Listed in Cargo.toml, not standalone
                        }
                    }
                }

                // If we get here, it has main() but isn't in a standard location
                // and isn't listed in Cargo.toml, so it's standalone
                true
            }
        }
    }

    /// Detect the file type based on the runnable
    fn detect_file_type(&self) -> Result<FileType> {
        match &self.runnable.kind {
            RunnableKind::Standalone { .. } => {
                if self.is_cargo_script_file(&self.runnable.file_path)? {
                    Ok(FileType::SingleFileScript)
                } else {
                    Ok(FileType::Standalone)
                }
            }
            RunnableKind::SingleFileScript { .. } => Ok(FileType::SingleFileScript),
            _ => {
                // Check cargo script FIRST since it's more specific than standalone
                if self.is_cargo_script_file(&self.runnable.file_path)? {
                    Ok(FileType::SingleFileScript)
                } else if self.is_standalone_file(&self.runnable.file_path) {
                    Ok(FileType::Standalone)
                } else {
                    Ok(FileType::CargoProject)
                }
            }
        }
    }

    /// Build the command
    pub fn build(self) -> Result<CargoCommand> {
        let file_type = self.detect_file_type()?;

        // Create the function identity
        let identity = FunctionIdentity {
            package: self.package_name.clone(),
            module_path: if self.runnable.module_path.is_empty() {
                None
            } else {
                Some(self.runnable.module_path.clone())
            },
            file_path: Some(self.runnable.file_path.clone()),
            function_name: self.runnable.get_function_name(),
            file_type: Some(file_type),
        };

        // Resolve configuration
        let config = if let Some(config) = self.config_override {
            config
        } else {
            ConfigResolver::new(self.project_root, &identity).resolve()?
        };

        // Delegate to specific builders based on file type first, then kind
        match (file_type, &self.runnable.kind) {
            // Standalone files use rustc directly
            (FileType::Standalone, RunnableKind::Test { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Binary { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::ModuleTests { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Benchmark { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Standalone { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::DocTest { .. }) => {
                // Doc tests in standalone files aren't supported
                Err(crate::error::Error::ParseError(
                    "Doc tests are not supported in standalone files".to_string()
                ))
            },
            (FileType::Standalone, RunnableKind::SingleFileScript { .. }) => {
                // This shouldn't happen - single file script should be FileType::SingleFileScript
                SingleFileScriptBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            },
            
            // Single file scripts
            (FileType::SingleFileScript, _) => SingleFileScriptBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            
            // Cargo projects use cargo commands
            (FileType::CargoProject, RunnableKind::DocTest { .. }) => DocTestCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Test { .. }) => TestCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Binary { .. }) => BinaryCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::ModuleTests { .. }) => ModuleTestCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Benchmark { .. }) => BenchmarkCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Standalone { .. }) => {
                // This shouldn't happen, but fallback to rustc
                RustcCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            },
            (FileType::CargoProject, RunnableKind::SingleFileScript { .. }) => {
                SingleFileScriptBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            },
        }
    }
}

/// Trait for building specific command types
pub trait CommandBuilderImpl: ConfigAccess {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand>;
}
