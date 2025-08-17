use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::types::FileType;

/// Detect the type of a Rust file
pub fn detect_file_type(file_path: &Path) -> FileType {
    tracing::trace!("Detecting file type for: {:?}", file_path);
    
    // Check if it's a standalone file first
    if !is_part_of_cargo_project(file_path) {
        // Check if it's a single-file script
        if is_single_file_script(file_path) {
            tracing::debug!("Detected SingleFileScript: {:?}", file_path);
            return FileType::SingleFileScript;
        }
        tracing::debug!("Detected Standalone file: {:?}", file_path);
        return FileType::Standalone;
    }
    
    tracing::debug!("Detected CargoProject file: {:?}", file_path);
    FileType::CargoProject
}

/// Check if a file is part of a Cargo project
fn is_part_of_cargo_project(file_path: &Path) -> bool {
    let mut current = file_path.parent();
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return true;
        }
        current = dir.parent();
    }
    false
}

/// Check if a file is a single-file Rust script
fn is_single_file_script(file_path: &Path) -> bool {
    // Try to read the first line
    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        if let Some(Ok(first_line)) = reader.lines().next() {
            // Check for the cargo script shebang
            if first_line.starts_with("#!/usr/bin/env -S cargo +nightly -Zscript") {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cargo_project_file() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();
        
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();
        let main_rs = src_dir.join("main.rs");
        fs::write(&main_rs, "fn main() {}").unwrap();
        
        assert_eq!(detect_file_type(&main_rs), FileType::CargoProject);
    }
    
    #[test]
    fn test_standalone_file() {
        let temp_dir = TempDir::new().unwrap();
        let standalone_rs = temp_dir.path().join("standalone.rs");
        fs::write(&standalone_rs, "fn main() {}").unwrap();
        
        assert_eq!(detect_file_type(&standalone_rs), FileType::Standalone);
    }
    
    #[test]
    fn test_single_file_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_rs = temp_dir.path().join("script.rs");
        fs::write(
            &script_rs,
            "#!/usr/bin/env -S cargo +nightly -Zscript\nfn main() {}"
        ).unwrap();
        
        assert_eq!(detect_file_type(&script_rs), FileType::SingleFileScript);
    }
}