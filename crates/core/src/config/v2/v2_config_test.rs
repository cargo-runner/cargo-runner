//! Integration tests for the v2 config system

#[cfg(test)]
mod tests {
    use crate::{
        build_system::BuildSystem,
        config::v2::{ConfigBuilder, builder::LayerConfigExt},
        types::{Position, Runnable, RunnableKind, Scope, ScopeKind},
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_runnable() -> Runnable {
        Runnable {
            label: "test_example".to_string(),
            kind: RunnableKind::Test {
                test_name: "test_example".to_string(),
                is_async: false,
            },
            scope: Scope {
                start: Position::new(1, 0),
                end: Position::new(10, 0),
                kind: ScopeKind::Function,
                name: Some("test_example".to_string()),
            },
            module_path: "tests".to_string(),
            file_path: PathBuf::from("src/tests.rs"),
            extended_scope: None,
        }
    }

    #[test]
    fn test_v2_config_with_runner() {
        // Create a v2 config
        let _v2_config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                    .framework_test("cargo-nextest")
                    .args_test(vec!["--no-capture".into()])
                    .env("RUST_LOG", "debug");
            })
            .build();

        let runnable = create_test_runnable();

        // Use the v2 config we created
        let resolver = _v2_config.resolver();
        let context = crate::config::v2::scope::ScopeContext::new()
            .with_crate("my-crate".to_string())
            .with_module(runnable.module_path.clone())
            .with_function(runnable.scope.name.clone().unwrap_or_default());
        
        let command = resolver.resolve_command(&context, runnable.kind).unwrap();

        // Verify the command uses nextest
        assert!(command.args.contains(&"nextest".to_string()));
        assert!(command.args.contains(&"run".to_string()));
        assert!(command.args.contains(&"--no-capture".to_string()));
        assert!(
            command
                .env
                .iter()
                .any(|(k, v)| k == "RUST_LOG" && v == "debug")
        );
    }

    #[test]
    fn test_v2_config_file_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".cargo-runner-v2.json");

        // Create JSON directly with new flat structure
        let json_config = crate::config::v2::json::JsonConfig {
            version: Some("2.0".to_string()),
            linked_projects: None,
            build_system: Some(BuildSystem::Cargo),
            frameworks: Some(crate::config::v2::json::JsonFrameworks {
                test: Some("cargo-nextest".to_string()),
                binary: None,
                benchmark: None,
                doctest: None,
                build: None,
            }),
            args: None,
            env: HashMap::new(),
            workspace: None,
            crates: HashMap::new(),
            files: HashMap::new(),
            modules: HashMap::new(),
            functions: {
                let mut map = HashMap::new();
                map.insert(
                    "test_special".to_string(),
                    crate::config::v2::json::JsonLayerConfig {
                        build_system: None,
                        frameworks: None,
                        args: Some(crate::config::v2::json::JsonArgs {
                            all: None,
                            test: Some(vec!["--test-threads=1".to_string()]),
                            binary: None,
                            benchmark: None,
                            build: None,
                            test_binary: None,
                        }),
                        env: HashMap::new(),
                    },
                );
                map
            },
        };

        // Save to JSON
        let json_str = serde_json::to_string_pretty(&json_config).unwrap();
        std::fs::write(&config_path, json_str).unwrap();

        // Load it back
        let loaded = std::fs::read_to_string(&config_path).unwrap();
        let loaded_json: crate::config::v2::json::JsonConfig =
            serde_json::from_str(&loaded).unwrap();

        // Verify the structure
        assert!(
            loaded.contains("\"test\": \"cargo-nextest\""),
            "Should contain test framework"
        );
        assert!(
            loaded.contains("test_special"),
            "Should contain function name"
        );
        assert!(
            loaded.contains("--test-threads=1"),
            "Should contain test args"
        );

        // Test conversion to Config
        let config = loaded_json.to_config();
        let resolver = config.resolver();

        // Verify it resolves correctly
        let context =
            crate::config::v2::scope::ScopeContext::new().with_function("test_special".to_string());
        let result = resolver.resolve_command(
            &context,
            RunnableKind::Test {
                test_name: "test_special".to_string(),
                is_async: false,
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_v2_config_with_module_scope() {
        // Create a v2 config with module-level overrides
        let _v2_config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                    .framework_test("cargo-test");
            })
            .module_override("integration", |m| {
                m.framework_test("cargo-nextest")
                    .args_test(vec!["--test-threads=1".into()]);
            })
            .build();

        // Test with a runnable in the integration module
        let mut runnable = create_test_runnable();
        runnable.module_path = "integration::tests".to_string();

        // Use the v2 config we created
        let resolver = _v2_config.resolver();
        let context = crate::config::v2::scope::ScopeContext::new()
            .with_crate("my-crate".to_string())
            .with_module(runnable.module_path.clone())
            .with_function(runnable.scope.name.clone().unwrap_or_default());
        
        let command = resolver.resolve_command(&context, runnable.kind).unwrap();

        // Should use nextest due to module override
        assert!(command.args.contains(&"nextest".to_string()));
        assert!(command.args.contains(&"--test-threads=1".to_string()));
    }

    #[test]
    fn test_v2_config_priority_order() {
        // Create a v2 config with multiple levels
        let _v2_config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                    .framework_test("cargo-test")
                    .args_test(vec!["--workspace-arg".into()]);
            })
            .crate_override("my-crate", |c| {
                c.args_test(vec!["--crate-arg".into()]);
            })
            .function_override("test_example", |f| {
                f.args_test(vec!["--function-arg".into()]);
            })
            .build();

        let runnable = create_test_runnable();

        // Use the v2 config we created
        let resolver = _v2_config.resolver();
        let context = crate::config::v2::scope::ScopeContext::new()
            .with_crate("my-crate".to_string())
            .with_module(runnable.module_path.clone())
            .with_function(runnable.scope.name.clone().unwrap_or_default());
        
        let command = resolver.resolve_command(&context, runnable.kind).unwrap();

        // Should have all args from all levels
        assert!(command.args.contains(&"--workspace-arg".to_string()));
        assert!(command.args.contains(&"--crate-arg".to_string()));
        assert!(command.args.contains(&"--function-arg".to_string()));
    }
}
