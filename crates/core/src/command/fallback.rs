use crate::{command::CargoCommand, error::Result};
use cargo_toml::Manifest;
use std::path::Path;

/// Generate a fallback command when no specific runnable is found at the given line
pub fn generate_fallback_command(
    file_path: &Path,
    package_name: Option<&str>,
    project_root: Option<&Path>,
) -> Result<Option<CargoCommand>> {
    let path_str = file_path.to_str().unwrap_or("");
    let file_name = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let mut args = vec![];

    // Normalize path separators and check patterns
    let normalized_path = path_str.replace('\\', "/");

    // Determine command based on file location patterns
    if normalized_path.contains("/src/bin/")
        || normalized_path.contains("src/bin/")
        || normalized_path.ends_with("/src/main.rs")
        || normalized_path.ends_with("src/main.rs")
    {
        // Binary target
        args.push("run".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        if (normalized_path.contains("/src/bin/") || normalized_path.contains("src/bin/"))
            && file_name != "main"
        {
            args.push("--bin".to_string());
            args.push(file_name.to_string());
        }
    } else if normalized_path.contains("/benches/") || normalized_path.contains("benches/") {
        // Benchmark target
        args.push("bench".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        args.push("--bench".to_string());
        args.push(file_name.to_string());
    } else if (normalized_path.contains("/tests/") || normalized_path.contains("tests/"))
        && !normalized_path.ends_with("/mod.rs")
        && !normalized_path.ends_with("mod.rs")
    {
        // Integration test
        args.push("test".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        args.push("--test".to_string());
        args.push(file_name.to_string());
    } else if normalized_path.ends_with("/src/lib.rs")
        || normalized_path.ends_with("src/lib.rs")
        || ((normalized_path.contains("/src/") || normalized_path.starts_with("src/"))
            && !normalized_path.contains("/src/bin/")
            && !normalized_path.starts_with("src/bin/"))
    {
        // Library target - lib.rs or any other file under src/ (except bin/)
        args.push("test".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        args.push("--lib".to_string());
    } else if normalized_path.contains("/examples/") || normalized_path.contains("examples/") {
        // Example target
        args.push("run".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }

        args.push("--example".to_string());
        args.push(file_name.to_string());
    } else if normalized_path.ends_with("/build.rs") || normalized_path.ends_with("build.rs") {
        // Build script
        args.push("build".to_string());

        if let Some(pkg) = package_name {
            args.push("--package".to_string());
            args.push(pkg.to_string());
        }
    } else {
        // No specific pattern matched - check Cargo.toml for custom targets
        if let Some(project_root) = project_root {
            if let Some(cmd) = check_cargo_toml_for_target(file_path, project_root, package_name)? {
                return Ok(Some(cmd));
            }
        }

        // Check if this is a standalone Rust file (no Cargo project)
        // A file is standalone if it has no project_root AND no package_name
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs")
            && project_root.is_none()
            && package_name.is_none()
        {
            return generate_rustc_command(file_path);
        }

        return Ok(None);
    }

    Ok(Some(CargoCommand::new(args)))
}

/// Check Cargo.toml for custom targets that match the file path
fn check_cargo_toml_for_target(
    file_path: &Path,
    project_root: &Path,
    package_name: Option<&str>,
) -> Result<Option<CargoCommand>> {
    let cargo_toml_path = project_root.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Ok(None);
    }

    let manifest = Manifest::from_path(&cargo_toml_path).map_err(|e| {
        crate::Error::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse Cargo.toml: {}", e),
        ))
    })?;

    let mut args = vec![];

    // Get relative path from project root
    let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
    let relative_str = relative_path.to_str().unwrap_or("");

    // Check [[bin]] entries
    if !manifest.bin.is_empty() {
        let bins = &manifest.bin;
        for bin in bins {
            if let Some(path) = &bin.path {
                if path == relative_str {
                    args.push("run".to_string());
                    if let Some(pkg) = package_name {
                        args.push("--package".to_string());
                        args.push(pkg.to_string());
                    }
                    args.push("--bin".to_string());
                    args.push(bin.name.clone().unwrap_or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    }));
                    return Ok(Some(CargoCommand::new(args)));
                }
            }
        }
    }

    // Check [[example]] entries
    if !manifest.example.is_empty() {
        let examples = &manifest.example;
        for example in examples {
            if let Some(path) = &example.path {
                if path == relative_str {
                    args.push("run".to_string());
                    if let Some(pkg) = package_name {
                        args.push("--package".to_string());
                        args.push(pkg.to_string());
                    }
                    args.push("--example".to_string());
                    args.push(example.name.clone().unwrap_or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    }));
                    return Ok(Some(CargoCommand::new(args)));
                }
            }
        }
    }

    // Check [[test]] entries
    if !manifest.test.is_empty() {
        let tests = &manifest.test;
        for test in tests {
            if let Some(path) = &test.path {
                if path == relative_str {
                    args.push("test".to_string());
                    if let Some(pkg) = package_name {
                        args.push("--package".to_string());
                        args.push(pkg.to_string());
                    }
                    args.push("--test".to_string());
                    args.push(test.name.clone().unwrap_or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    }));
                    return Ok(Some(CargoCommand::new(args)));
                }
            }
        }
    }

    // Check [[bench]] entries
    if !manifest.bench.is_empty() {
        let benches = &manifest.bench;
        for bench in benches {
            if let Some(path) = &bench.path {
                if path == relative_str {
                    args.push("bench".to_string());
                    if let Some(pkg) = package_name {
                        args.push("--package".to_string());
                        args.push(pkg.to_string());
                    }
                    args.push("--bench".to_string());
                    args.push(bench.name.clone().unwrap_or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string()
                    }));
                    return Ok(Some(CargoCommand::new(args)));
                }
            }
        }
    }

    // Check [lib] entry
    if let Some(lib) = &manifest.lib {
        if let Some(path) = &lib.path {
            if path == relative_str {
                args.push("test".to_string());
                if let Some(pkg) = package_name {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
                args.push("--lib".to_string());
                return Ok(Some(CargoCommand::new(args)));
            }
        }
    }

    Ok(None)
}

/// Generate a rustc command for standalone Rust files
fn generate_rustc_command(file_path: &Path) -> Result<Option<CargoCommand>> {
    let file_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| crate::Error::ParseError("Invalid file name".to_string()))?;

    // Create rustc command: rustc file.rs -o file
    let args = vec![
        file_path.to_str().unwrap_or("").to_string(),
        "-o".to_string(),
        file_name.to_string(),
    ];

    Ok(Some(CargoCommand::new_rustc(args)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandType;
    use std::path::PathBuf;

    #[test]
    fn test_binary_fallback() {
        let path = PathBuf::from("/project/src/bin/my_tool.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(
            cmd.args,
            vec!["run", "--package", "my_crate", "--bin", "my_tool"]
        );
    }

    #[test]
    fn test_main_binary_fallback() {
        let path = PathBuf::from("/project/src/main.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(cmd.args, vec!["run", "--package", "my_crate"]);
    }

    #[test]
    fn test_lib_fallback() {
        let path = PathBuf::from("/project/src/lib.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(cmd.args, vec!["test", "--package", "my_crate", "--lib"]);
    }

    #[test]
    fn test_example_fallback() {
        let path = PathBuf::from("/project/examples/demo.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(
            cmd.args,
            vec!["run", "--package", "my_crate", "--example", "demo"]
        );
    }

    #[test]
    fn test_integration_test_fallback() {
        let path = PathBuf::from("/project/tests/integration.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(
            cmd.args,
            vec!["test", "--package", "my_crate", "--test", "integration"]
        );
    }

    #[test]
    fn test_bench_fallback() {
        let path = PathBuf::from("/project/benches/performance.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(
            cmd.args,
            vec!["bench", "--package", "my_crate", "--bench", "performance"]
        );
    }

    #[test]
    fn test_build_script_fallback() {
        let path = PathBuf::from("/project/build.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None)
            .unwrap()
            .unwrap();

        assert_eq!(cmd.args, vec!["build", "--package", "my_crate"]);
    }

    #[test]
    fn test_no_pattern_match() {
        // Test a file in src/ that isn't lib.rs/main.rs in a Cargo project
        // Since package_name is provided, it's in a Cargo project
        let path = PathBuf::from("/project/src/utils.rs");
        let cmd = generate_fallback_command(&path, Some("my_crate"), None).unwrap();

        assert!(cmd.is_none());
    }

    #[test]
    fn test_standalone_rust_file() {
        // Test a standalone Rust file (no package name, no project root)
        let path = PathBuf::from("/tmp/test.rs");
        let cmd = generate_fallback_command(&path, None, None)
            .unwrap()
            .unwrap();

        assert_eq!(cmd.command_type, CommandType::Rustc);
        assert_eq!(cmd.args, vec!["/tmp/test.rs", "-o", "test"]);
    }
}
