use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildSystem {
    Cargo,
    Bazel,
    Rustc,
    CargoScript,
}

pub trait BuildSystemDetector {
    fn detect(&self, project_path: &Path) -> Option<BuildSystem>;
}

pub struct DefaultBuildSystemDetector;

impl BuildSystemDetector for DefaultBuildSystemDetector {
    fn detect(&self, project_path: &Path) -> Option<BuildSystem> {
        tracing::debug!("DefaultBuildSystemDetector::detect checking path: {:?}", project_path);
        
        // Check for Bazel first since a project might have both
        // For Bazel detection, we require BUILD files, not just MODULE.bazel/WORKSPACE
        // This prevents standalone files from being detected as Bazel just because
        // they're under a directory tree with a MODULE.bazel file far up the hierarchy
        let build_bazel = project_path.join("BUILD.bazel");
        let build = project_path.join("BUILD");
        
        if build_bazel.exists() {
            tracing::info!("Found BUILD.bazel at: {:?}", build_bazel);
            return Some(BuildSystem::Bazel);
        }
        
        if build.exists() {
            tracing::info!("Found BUILD at: {:?}", build);
            return Some(BuildSystem::Bazel);
        }

        // Check for Cargo
        let cargo_toml = project_path.join("Cargo.toml");
        if cargo_toml.exists() {
            tracing::debug!("Found Cargo.toml at: {:?}", cargo_toml);
            return Some(BuildSystem::Cargo);
        }
        
        // Check if this is a standalone Rust file
        if project_path.is_file() && project_path.extension().map_or(false, |ext| ext == "rs") {
            // Check if this file is NOT part of a Cargo project
            let mut current = project_path.parent();
            while let Some(dir) = current {
                if dir.join("Cargo.toml").exists() {
                    // This is part of a Cargo project, not standalone
                    return Some(BuildSystem::Cargo);
                }
                current = dir.parent();
            }
            tracing::debug!("Detected standalone Rust file: {:?}", project_path);
            return Some(BuildSystem::Rustc);
        }

        tracing::debug!("No build system found at: {:?}", project_path);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cargo_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();

        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Cargo));
    }

    #[test]
    fn test_detect_bazel_project_with_build_file() {
        let temp_dir = TempDir::new().unwrap();
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&build_file, "rust_binary(name = \"test\")").unwrap();

        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_detect_bazel_project_with_module_file() {
        let temp_dir = TempDir::new().unwrap();
        let module_file = temp_dir.path().join("MODULE.bazel");
        fs::write(&module_file, "module(name = \"test\")").unwrap();

        // MODULE.bazel alone doesn't indicate a Bazel project - need BUILD files
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, None);
        
        // Add a BUILD file to make it a Bazel project
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&build_file, "# BUILD file").unwrap();
        
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_detect_bazel_project_with_workspace() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_file = temp_dir.path().join("WORKSPACE");
        fs::write(&workspace_file, "workspace(name = \"test\")").unwrap();

        // WORKSPACE alone doesn't indicate a Bazel project - need BUILD files
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, None);
        
        // Add a BUILD file to make it a Bazel project
        let build_file = temp_dir.path().join("BUILD");
        fs::write(&build_file, "# BUILD file").unwrap();
        
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_prefer_bazel_over_cargo() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();
        fs::write(&build_file, "rust_binary(name = \"test\")").unwrap();

        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_no_build_system() {
        let temp_dir = TempDir::new().unwrap();

        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(temp_dir.path());
        assert_eq!(build_system, None);
    }
    
    #[test]
    fn test_detect_standalone_rust_file() {
        let temp_dir = TempDir::new().unwrap();
        let rust_file = temp_dir.path().join("standalone.rs");
        fs::write(&rust_file, "fn main() { println!(\"Hello\"); }").unwrap();
        
        let detector = DefaultBuildSystemDetector;
        let build_system = detector.detect(&rust_file);
        assert_eq!(build_system, Some(BuildSystem::Rustc));
    }
    
    #[test]
    fn test_rust_file_in_cargo_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();
        
        let rust_file = temp_dir.path().join("main.rs");
        fs::write(&rust_file, "fn main() {}").unwrap();
        
        let detector = DefaultBuildSystemDetector;
        // Even though it's a .rs file, it should detect Cargo because of Cargo.toml
        let build_system = detector.detect(&rust_file);
        assert_eq!(build_system, Some(BuildSystem::Cargo));
    }
}
