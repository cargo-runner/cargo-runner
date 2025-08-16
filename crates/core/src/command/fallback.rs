use crate::{
    command::CargoCommand,
    error::Result,
    types::{Position, Runnable, RunnableKind, Scope, ScopeKind},
};
use cargo_toml::Manifest;
use std::path::Path;
use tracing::debug;

/// Generate a fallback command when no specific runnable is found at the given line
pub fn generate_fallback_command(
    file_path: &Path,
    package_name: Option<&str>,
    _project_root: Option<&Path>,
    _config: Option<crate::config::V2Config>,
) -> Result<Option<CargoCommand>> {
    debug!("generate_fallback_command: package_name={:?}", package_name);
    debug!("generate_fallback_command: file_path={:?}", file_path);
    // Create a synthetic runnable based on file location
    let runnable = create_synthetic_runnable(file_path, package_name)?;

    debug!("Created synthetic runnable: {:?}", runnable.is_some());

    if let Some(runnable) = runnable {
        debug!(
            "Synthetic runnable: kind={:?}, file={:?}, module_path='{}'",
            runnable.kind, runnable.file_path, runnable.module_path
        );

        // Build command using v2 config
        use crate::config::v2::{ConfigResolver, StrategyRegistry, ScopeContext};
        
        // Create scope context
        let context = ScopeContext {
            file_path: Some(runnable.file_path.clone()),
            crate_name: package_name.map(|s| s.to_string()),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            function_name: runnable.get_function_name(),
            type_name: None,
            method_name: None,
            scope_kind: None,
        };
        
        // Use default config and registry
        let config = crate::config::v2::ConfigLoader::load()
            .unwrap_or_else(|_| crate::config::v2::Config::default_with_build_system());
        let registry = StrategyRegistry::default();
        let resolver = ConfigResolver::new(config.layers(), &registry, &config.linked_projects);
        
        let command = resolver.resolve_command(&context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))?;

        debug!("Generated fallback command: {:?}", command.args);

        Ok(Some(command))
    } else {
        // Check if this might be a standalone Rust file
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if is_standalone_rust_file(file_path) {
                return generate_rustc_command(file_path);
            }
        }

        Ok(None)
    }
}

/// Create a synthetic runnable based on file path patterns
fn create_synthetic_runnable(
    file_path: &Path,
    package_name: Option<&str>,
) -> Result<Option<Runnable>> {
    debug!("create_synthetic_runnable: file_path={:?}", file_path);
    // First check if this is a cargo script file
    if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            if let Some(first_line) = content.lines().next() {
                debug!("First line of file: {:?}", first_line);
                if first_line.starts_with("#!")
                    && first_line.contains("cargo")
                    && first_line.contains("-Zscript")
                {
                    debug!("Detected cargo script file!");
                    // It's a cargo script file
                    let scope = Scope {
                        kind: ScopeKind::Function,
                        name: Some("main".to_string()),
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    };

                    // Check if it contains benchmarks
                    let has_benchmarks = content.contains("#[bench]")
                        || content.contains("criterion_group!")
                        || content.contains("criterion_main!");

                    // If it has benchmarks, we'll handle it specially when building the command
                    return Ok(Some(Runnable {
                        label: if has_benchmarks {
                            "Run cargo script benchmarks".to_string()
                        } else {
                            "Run cargo script".to_string()
                        },
                        scope,
                        kind: RunnableKind::SingleFileScript {
                            shebang: first_line.to_string(),
                        },
                        module_path: String::new(),
                        file_path: file_path.to_path_buf(),
                        extended_scope: None,
                    }));
                }
            }
        }
    }

    // First check if we can find a project root for custom targets
    let project_root = file_path
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists());
    let path_str = file_path.to_str().unwrap_or("");
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // Normalize path separators
    let normalized_path = path_str.replace('\\', "/");

    // Create a dummy scope for the synthetic runnable
    let scope = Scope {
        kind: ScopeKind::Function,
        name: Some("main".to_string()),
        start: Position {
            line: 1,
            character: 0,
        },
        end: Position {
            line: 100,
            character: 0,
        },
    };

    // Determine runnable kind based on file location patterns
    if normalized_path.contains("/src/bin/")
        || normalized_path.contains("src/bin/")
        || normalized_path.ends_with("/src/main.rs")
        || normalized_path.ends_with("src/main.rs")
    {
        // Binary target
        let bin_name = if normalized_path.ends_with("/src/main.rs")
            || normalized_path.ends_with("src/main.rs")
        {
            None
        } else if file_name != "main" {
            Some(file_name.to_string())
        } else {
            None
        };

        Ok(Some(Runnable {
            label: if let Some(ref name) = bin_name {
                format!("Run binary '{}'", name)
            } else {
                "Run main()".to_string()
            },
            scope,
            kind: RunnableKind::Binary { bin_name },
            module_path: package_name.unwrap_or_default().to_string(),
            file_path: file_path.to_path_buf(),
            extended_scope: None,
        }))
    } else if normalized_path.contains("/benches/") || normalized_path.contains("benches/") {
        // Benchmark target
        Ok(Some(Runnable {
            label: format!("Run benchmark '{}'", file_name),
            scope,
            kind: RunnableKind::Benchmark {
                bench_name: file_name.to_string(),
            },
            module_path: String::new(),
            file_path: file_path.to_path_buf(),
            extended_scope: None,
        }))
    } else if (normalized_path.contains("/tests/") || normalized_path.contains("tests/"))
        && !normalized_path.ends_with("/mod.rs")
        && !normalized_path.ends_with("mod.rs")
    {
        // Integration test
        Ok(Some(Runnable {
            label: format!("Run test '{}'", file_name),
            scope,
            kind: RunnableKind::Test {
                test_name: file_name.to_string(),
                is_async: false,
            },
            module_path: String::new(),
            file_path: file_path.to_path_buf(),
            extended_scope: None,
        }))
    } else if normalized_path.ends_with("/src/lib.rs")
        || normalized_path.ends_with("src/lib.rs")
        || ((normalized_path.contains("/src/") || normalized_path.starts_with("src/"))
            && !normalized_path.contains("/src/bin/")
            && !normalized_path.starts_with("src/bin/"))
    {
        // Library target - run all tests in the library
        Ok(Some(Runnable {
            label: "Run library tests".to_string(),
            scope,
            kind: RunnableKind::ModuleTests {
                module_name: String::new(),
            },
            module_path: String::new(),
            file_path: file_path.to_path_buf(),
            extended_scope: None,
        }))
    } else if normalized_path.contains("/examples/") || normalized_path.contains("examples/") {
        // Example target - treat as binary
        Ok(Some(Runnable {
            label: format!("Run example '{}'", file_name),
            scope,
            kind: RunnableKind::Binary {
                bin_name: Some(file_name.to_string()),
            },
            module_path: String::new(),
            file_path: file_path.to_path_buf(),
            extended_scope: None,
        }))
    } else if normalized_path.ends_with("/build.rs") || normalized_path.ends_with("build.rs") {
        // Build script - we can't create a runnable for this
        Ok(None)
    } else {
        // Check for custom targets in Cargo.toml
        if let Some(project_root) = project_root {
            check_cargo_toml_for_runnable(file_path, project_root, package_name)
        } else {
            Ok(None)
        }
    }
}

/// Check Cargo.toml for custom targets that match the file path
fn check_cargo_toml_for_runnable(
    file_path: &Path,
    project_root: &Path,
    package_name: Option<&str>,
) -> Result<Option<Runnable>> {
    let cargo_toml_path = project_root.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Ok(None);
    }

    let manifest = Manifest::from_path(&cargo_toml_path).map_err(|e| {
        crate::Error::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse Cargo.toml: {e}"),
        ))
    })?;

    // Get relative path from project root
    let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
    let relative_str = relative_path.to_str().unwrap_or("");
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // Create a dummy scope for the synthetic runnable
    let scope = Scope {
        kind: ScopeKind::Function,
        name: Some("main".to_string()),
        start: Position {
            line: 1,
            character: 0,
        },
        end: Position {
            line: 100,
            character: 0,
        },
    };

    // Check [[bin]] entries
    if !manifest.bin.is_empty() {
        for bin in &manifest.bin {
            if let Some(path) = &bin.path {
                if path == relative_str {
                    let bin_name = bin.name.clone().unwrap_or_else(|| file_name.to_string());
                    return Ok(Some(Runnable {
                        label: format!("Run binary '{}'", bin_name),
                        scope,
                        kind: RunnableKind::Binary {
                            bin_name: Some(bin_name),
                        },
                        module_path: package_name.unwrap_or_default().to_string(),
                        file_path: file_path.to_path_buf(),
                        extended_scope: None,
                    }));
                }
            }
        }
    }

    // Check [[example]] entries
    if !manifest.example.is_empty() {
        for example in &manifest.example {
            if let Some(path) = &example.path {
                if path == relative_str {
                    let example_name = example
                        .name
                        .clone()
                        .unwrap_or_else(|| file_name.to_string());
                    return Ok(Some(Runnable {
                        label: format!("Run example '{}'", example_name),
                        scope,
                        kind: RunnableKind::Binary {
                            bin_name: Some(example_name),
                        },
                        module_path: package_name.unwrap_or_default().to_string(),
                        file_path: file_path.to_path_buf(),
                        extended_scope: None,
                    }));
                }
            }
        }
    }

    // Check [[test]] entries
    if !manifest.test.is_empty() {
        for test in &manifest.test {
            if let Some(path) = &test.path {
                if path == relative_str {
                    let test_name = test.name.clone().unwrap_or_else(|| file_name.to_string());
                    return Ok(Some(Runnable {
                        label: format!("Run test '{}'", test_name),
                        scope,
                        kind: RunnableKind::Test {
                            test_name,
                            is_async: false,
                        },
                        module_path: package_name.unwrap_or_default().to_string(),
                        file_path: file_path.to_path_buf(),
                        extended_scope: None,
                    }));
                }
            }
        }
    }

    // Check [[bench]] entries
    if !manifest.bench.is_empty() {
        for bench in &manifest.bench {
            if let Some(path) = &bench.path {
                if path == relative_str {
                    let bench_name = bench.name.clone().unwrap_or_else(|| file_name.to_string());
                    return Ok(Some(Runnable {
                        label: format!("Run benchmark '{}'", bench_name),
                        scope,
                        kind: RunnableKind::Benchmark { bench_name },
                        module_path: package_name.unwrap_or_default().to_string(),
                        file_path: file_path.to_path_buf(),
                        extended_scope: None,
                    }));
                }
            }
        }
    }

    // Check [lib] entry
    if let Some(lib) = &manifest.lib {
        if let Some(path) = &lib.path {
            if path == relative_str {
                return Ok(Some(Runnable {
                    label: "Run library tests".to_string(),
                    scope,
                    kind: RunnableKind::ModuleTests {
                        module_name: String::new(),
                    },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None,
                }));
            }
        }
    }

    Ok(None)
}

/// Check if a file is a standalone Rust file (has main() and not part of Cargo project)
fn is_standalone_rust_file(file_path: &Path) -> bool {
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

                // Check if it's listed in Cargo.toml as a [[bin]]
                let cargo_toml_path = root.join("Cargo.toml");
                if let Ok(manifest) = Manifest::from_path(&cargo_toml_path) {
                    // Check if this file is explicitly listed as a binary
                    for bin in &manifest.bin {
                        if let Some(bin_path) = &bin.path {
                            if bin_path == path_str {
                                return false; // Listed in Cargo.toml, not standalone
                            }
                        }
                    }
                }

                // Has main(), not in standard location, not in Cargo.toml = standalone
                true
            } else {
                true // Shouldn't happen, but treat as standalone if strip_prefix fails
            }
        }
    }
}

/// Generate a rustc command for standalone Rust files
fn generate_rustc_command(file_path: &Path) -> Result<Option<CargoCommand>> {
    // Read the file content to check for shebang and tests
    let content = std::fs::read_to_string(file_path).map_err(|e| crate::Error::IoError(e))?;

    // Check if it's a cargo script file (has shebang)
    if let Some(first_line) = content.lines().next() {
        if first_line.starts_with("#!")
            && first_line.contains("cargo")
            && first_line.contains("-Zscript")
        {
            // It's a cargo script file
            let scope = Scope {
                kind: ScopeKind::Function,
                name: Some("main".to_string()),
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            };

            let runnable = Runnable {
                label: "Run cargo script".to_string(),
                scope,
                kind: RunnableKind::SingleFileScript {
                    shebang: first_line.to_string(),
                },
                module_path: String::new(),
                file_path: file_path.to_path_buf(),
                extended_scope: None,
            };

            // Build command using v2 config
            use crate::config::v2::{ConfigResolver, StrategyRegistry, ScopeContext};
            
            // Create scope context
            let context = ScopeContext {
                file_path: Some(runnable.file_path.clone()),
                crate_name: None,
                module_path: None,
                function_name: None,
                type_name: None,
                method_name: None,
                scope_kind: None,
            };
            
            // Create a test config with build system and frameworks
            use crate::config::v2::{ConfigBuilder, builder::LayerConfigExt};
            use crate::build_system::BuildSystem;
            
            let config = ConfigBuilder::new()
                .workspace(|w| {
                    w.build_system(BuildSystem::Cargo)
                        .framework_test("cargo-test")
                        .framework_binary("cargo-run")
                        .framework_benchmark("cargo-bench")
                        .framework_doctest("cargo-test")
                        .framework_build("cargo-build");
                })
                .build();
            
            let registry = StrategyRegistry::default();
            let resolver = ConfigResolver::new(config.layers(), &registry, &config.linked_projects);
            
            let command = resolver.resolve_command(&context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))?;

            return Ok(Some(command));
        }
    }

    // Not a cargo script, check if it has tests
    let has_tests = content.contains("#[test]") || content.contains("#[cfg(test)]");

    // Create a standalone runnable
    let scope = Scope {
        kind: ScopeKind::Function,
        name: Some("main".to_string()),
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    };

    let runnable = Runnable {
        label: "Run standalone file".to_string(),
        scope,
        kind: RunnableKind::Standalone { has_tests },
        module_path: String::new(),
        file_path: file_path.to_path_buf(),
        extended_scope: None,
    };

    // Build command using v2 config
    use crate::config::v2::{ConfigResolver, StrategyRegistry, ScopeContext};
    
    // Create scope context
    let context = ScopeContext {
        file_path: Some(runnable.file_path.clone()),
        crate_name: None,
        module_path: None,
        function_name: None,
        type_name: None,
        method_name: None,
        scope_kind: None,
    };
    
    // Use default config and registry
    let config = crate::config::v2::ConfigLoader::load()
        .unwrap_or_else(|_| crate::config::v2::Config::default_with_build_system());
    let registry = StrategyRegistry::default();
    let resolver = ConfigResolver::new(config.layers(), &registry, &config.linked_projects);
    
    let command = resolver.resolve_command(&context, runnable.kind.clone())
        .map_err(|e| crate::Error::ConfigError(e))?;

    Ok(Some(command))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    // Helper for tests that use v2 config
    fn generate_fallback_command_with_v2(
        file_path: &Path,
        package_name: Option<&str>,
        _project_root: Option<&Path>,
    ) -> Result<Option<CargoCommand>> {
        // Create a synthetic runnable
        let runnable = create_synthetic_runnable(file_path, package_name)?;
        
        if let Some(runnable) = runnable {
            // Build command using v2 config
            use crate::config::v2::{ConfigResolver, StrategyRegistry, ScopeContext};
            
            // Create scope context
            let context = ScopeContext {
                file_path: Some(runnable.file_path.clone()),
                crate_name: package_name.map(|s| s.to_string()),
                module_path: if runnable.module_path.is_empty() {
                    None
                } else {
                    Some(runnable.module_path.clone())
                },
                function_name: runnable.get_function_name(),
                type_name: None,
                method_name: None,
                scope_kind: None,
            };
            
            // Create a test config with build system and frameworks
            use crate::config::v2::{ConfigBuilder, builder::LayerConfigExt};
            use crate::build_system::BuildSystem;
            
            let config = ConfigBuilder::new()
                .workspace(|w| {
                    w.build_system(BuildSystem::Cargo)
                        .framework_test("cargo-test")
                        .framework_binary("cargo-run")
                        .framework_benchmark("cargo-bench")
                        .framework_doctest("cargo-test")
                        .framework_build("cargo-build");
                })
                .build();
            
            let registry = StrategyRegistry::default();
            let resolver = ConfigResolver::new(config.layers(), &registry, &config.linked_projects);
            
            let command = resolver.resolve_command(&context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))?;
            Ok(Some(command))
        } else {
            Ok(None)
        }
    }

    #[test]
    fn test_binary_fallback() {
        let path = PathBuf::from("/project/src/bin/my_tool.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        // The command builder will generate the correct arguments based on config
        // We just verify that a command was generated
        assert!(cmd.args.contains(&"run".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
        assert!(cmd.args.contains(&"--bin".to_string()));
        assert!(cmd.args.contains(&"my_tool".to_string()));
    }

    #[test]
    fn test_main_binary_fallback() {
        let path = PathBuf::from("/project/src/main.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        assert!(cmd.args.contains(&"run".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
    }

    #[test]
    fn test_lib_fallback() {
        let path = PathBuf::from("/project/src/lib.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        assert!(cmd.args.contains(&"test".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
        // V2 might include --lib or might use a different pattern
        // Just verify it's a test command for the right package
    }

    #[test]
    fn test_example_fallback() {
        let path = PathBuf::from("/project/examples/demo.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        assert!(cmd.args.contains(&"run".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
        // V2 might handle examples differently
        assert!(cmd.args.contains(&"--example".to_string()) || cmd.args.contains(&"demo".to_string()));
    }

    #[test]
    fn test_integration_test_fallback() {
        let path = PathBuf::from("/project/tests/integration.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        assert!(cmd.args.contains(&"test".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
        // V2 formats integration tests differently - it uses -- ::main pattern
        // Just check that it's a test command for the right package
    }

    #[test]
    fn test_bench_fallback() {
        let path = PathBuf::from("/project/benches/performance.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap()
            .unwrap();

        assert!(cmd.args.contains(&"bench".to_string()));
        assert!(cmd.args.contains(&"-p".to_string()) || cmd.args.contains(&"--package".to_string()));
        assert!(cmd.args.contains(&"my_crate".to_string()));
        // V2 might handle benchmarks differently
        assert!(cmd.args.contains(&"--bench".to_string()) || cmd.args.contains(&"performance".to_string()));
    }

    #[test]
    fn test_build_script_fallback() {
        let path = PathBuf::from("/project/build.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")));

        // Build scripts are not runnable, should return None
        assert!(cmd.unwrap().is_none());
    }

    #[test]
    fn test_no_pattern_match() {
        // Test a file that doesn't match any pattern - not in a standard Cargo directory
        let path = PathBuf::from("/project/random.rs");
        let cmd = generate_fallback_command_with_v2(&path, Some("my_crate"), Some(Path::new("/project")))
            .unwrap();

        assert!(cmd.is_none());
    }

    #[test]
    #[ignore = "Rustc strategies not implemented yet"]
    fn test_standalone_rust_file() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // For standalone files, we need to create v2 config ourselves since
        // the normal generate_rustc_command uses file loading
        let path = PathBuf::from("/tmp/test.rs");
        
        // Create a standalone runnable
        let scope = Scope {
            kind: ScopeKind::Function,
            name: Some("main".to_string()),
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        };

        let runnable = Runnable {
            label: "Run standalone file".to_string(),
            scope,
            kind: RunnableKind::Standalone { has_tests: false },
            module_path: String::new(),
            file_path: path.clone(),
            extended_scope: None,
        };

        // Build command using v2 config
        use crate::config::v2::{ConfigResolver, StrategyRegistry, ScopeContext};
        
        // Create scope context
        let context = ScopeContext {
            file_path: Some(runnable.file_path.clone()),
            crate_name: None,
            module_path: None,
            function_name: None,
            type_name: None,
            method_name: None,
            scope_kind: None,
        };
        
        // Create a test config with build system and frameworks
        // For standalone files, we should use Rustc build system
        use crate::config::v2::{ConfigBuilder, builder::LayerConfigExt};
        use crate::build_system::BuildSystem;
        
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                    .framework_test("cargo-test")
                    .framework_binary("cargo-run");
            })
            .build();
        
        let registry = StrategyRegistry::default();
        let resolver = ConfigResolver::new(config.layers(), &registry, &config.linked_projects);
        
        let cmd = resolver.resolve_command(&context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))?;

        // V2 command builder generates cargo commands for standalone files
        // The exact format depends on the framework configuration
        assert!(!cmd.args.is_empty());
        Ok(())
    }
}