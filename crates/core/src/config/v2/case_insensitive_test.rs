//! Tests for case-insensitive enum parsing in configs

#[cfg(test)]
mod tests {
    use crate::build_system::BuildSystem;
    use crate::types::FileType;
    use serde_json;
    
    #[test]
    fn test_build_system_case_insensitive() {
        // Test lowercase
        let result: BuildSystem = serde_json::from_str(r#""cargo""#).unwrap();
        assert_eq!(result, BuildSystem::Cargo);
        
        // Test uppercase
        let result: BuildSystem = serde_json::from_str(r#""CARGO""#).unwrap();
        assert_eq!(result, BuildSystem::Cargo);
        
        // Test mixed case
        let result: BuildSystem = serde_json::from_str(r#""CaRgO""#).unwrap();
        assert_eq!(result, BuildSystem::Cargo);
        
        // Test the problematic case from the user's config
        let result: BuildSystem = serde_json::from_str(r#""RustC""#).unwrap();
        assert_eq!(result, BuildSystem::Rustc);
        
        let result: BuildSystem = serde_json::from_str(r#""rustc""#).unwrap();
        assert_eq!(result, BuildSystem::Rustc);
        
        let result: BuildSystem = serde_json::from_str(r#""Rustc""#).unwrap();
        assert_eq!(result, BuildSystem::Rustc);
        
        // Test Bazel variants
        let result: BuildSystem = serde_json::from_str(r#""bazel""#).unwrap();
        assert_eq!(result, BuildSystem::Bazel);
        
        let result: BuildSystem = serde_json::from_str(r#""BAZEL""#).unwrap();
        assert_eq!(result, BuildSystem::Bazel);
        
        let result: BuildSystem = serde_json::from_str(r#""Bazel""#).unwrap();
        assert_eq!(result, BuildSystem::Bazel);
        
        // Test invalid variant
        let result: Result<BuildSystem, _> = serde_json::from_str(r#""invalid""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown build system"));
    }
    
    #[test]
    fn test_file_type_case_insensitive() {
        // Test CargoProject variants
        let result: FileType = serde_json::from_str(r#""cargo_project""#).unwrap();
        assert_eq!(result, FileType::CargoProject);
        
        let result: FileType = serde_json::from_str(r#""CARGO_PROJECT""#).unwrap();
        assert_eq!(result, FileType::CargoProject);
        
        let result: FileType = serde_json::from_str(r#""Cargo_Project""#).unwrap();
        assert_eq!(result, FileType::CargoProject);
        
        // Test Standalone variants
        let result: FileType = serde_json::from_str(r#""standalone""#).unwrap();
        assert_eq!(result, FileType::Standalone);
        
        let result: FileType = serde_json::from_str(r#""STANDALONE""#).unwrap();
        assert_eq!(result, FileType::Standalone);
        
        let result: FileType = serde_json::from_str(r#""StandAlone""#).unwrap();
        assert_eq!(result, FileType::Standalone);
        
        // Test SingleFileScript variants
        let result: FileType = serde_json::from_str(r#""single_file_script""#).unwrap();
        assert_eq!(result, FileType::SingleFileScript);
        
        let result: FileType = serde_json::from_str(r#""SINGLE_FILE_SCRIPT""#).unwrap();
        assert_eq!(result, FileType::SingleFileScript);
        
        let result: FileType = serde_json::from_str(r#""Single_File_Script""#).unwrap();
        assert_eq!(result, FileType::SingleFileScript);
        
        // Test invalid variant
        let result: Result<FileType, _> = serde_json::from_str(r#""invalid""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown variant"));
    }
    
    #[test]
    fn test_config_with_case_variations() {
        use crate::config::v2::json::JsonConfig;
        
        // Test a full config with various case variations
        let json = r#"{
            "version": "2.0",
            "build_system": "RustC",
            "linked_projects": ["project-a", "project-b"],
            "crates": {
                "my_crate": {
                    "build_system": "CARGO"
                }
            },
            "files": {
                "src/main.rs": {
                    "build_system": "bazel"
                }
            }
        }"#;
        
        let config: JsonConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.build_system, Some(BuildSystem::Rustc));
        assert_eq!(config.crates.get("my_crate").unwrap().build_system, Some(BuildSystem::Cargo));
        assert_eq!(config.files.get("src/main.rs").unwrap().build_system, Some(BuildSystem::Bazel));
    }
}