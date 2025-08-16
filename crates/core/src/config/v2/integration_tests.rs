//! Integration tests for v2 configuration system

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{
        build_system::BuildSystem,
        command::CommandType,
        types::RunnableKind,
    };
    use std::path::PathBuf;

    /// Test full workflow: create config, resolve command, verify output
    #[test]
    fn test_workspace_config_integration() {
        // Create a configuration
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .framework_binary("cargo-run")
                 .args_test(vec!["--nocapture".into()])
                 .env("RUST_LOG", "info");
            })
            .build();
        
        // Create resolver
        let resolver = config.resolver();
        
        // Create context for a test
        let context = scope::ScopeContext::new()
            .with_crate("my-crate".into())
            .with_module("tests".into())
            .with_function("test_something".into());
        
        // Resolve command
        let command = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "test_something".into(),
            is_async: false,
        }).unwrap();
        
        // Verify command
        assert_eq!(command.command_type, CommandType::Cargo);
        assert!(command.args.contains(&"test".into()));
        assert!(command.args.contains(&"--nocapture".into()));
        assert!(command.args.contains(&"--package".into()));
        assert!(command.args.contains(&"my-crate".into()));
        assert!(command.args.contains(&"tests::test_something".into()));
        assert!(command.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "info"));
    }

    /// Test cascading overrides
    #[test]
    fn test_cascading_overrides_integration() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .args_all(vec!["--verbose".into()])
                 .env("RUST_LOG", "warn")
                 .env("RUST_BACKTRACE", "0");
            })
            .crate_override("special-crate", |c| {
                c.framework_test("cargo-nextest")
                 .args_test(vec!["--no-capture".into()])
                 .env("RUST_LOG", "info");
            })
            .module_override("special-crate::integration", |m| {
                m.args_test_binary(vec!["--test-threads=1".into()])
                 .env("RUST_BACKTRACE", "1");
            })
            .function_override("test_database", |f| {
                f.env("DATABASE_URL", "postgres://test")
                 .env("RUST_LOG", "debug");
            })
            .build();
        
        let resolver = config.resolver();
        
        // Test 1: Workspace level
        let ctx1 = scope::ScopeContext::new()
            .with_crate("other-crate".into());
        let cmd1 = resolver.resolve_command(&ctx1, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        }).unwrap();
        
        assert!(cmd1.args.contains(&"test".into())); // cargo test
        assert!(cmd1.args.contains(&"--verbose".into()));
        assert!(cmd1.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "warn"));
        assert!(cmd1.env.iter().any(|(k, v)| k == "RUST_BACKTRACE" && v == "0"));
        
        // Test 2: Crate override
        let ctx2 = scope::ScopeContext::new()
            .with_crate("special-crate".into())
            .with_module("utils".into());
        let cmd2 = resolver.resolve_command(&ctx2, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        }).unwrap();
        
        assert!(cmd2.args.contains(&"nextest".into())); // cargo nextest
        assert!(cmd2.args.contains(&"run".into()));
        assert!(cmd2.args.contains(&"--verbose".into())); // from workspace
        assert!(cmd2.args.contains(&"--no-capture".into())); // from crate
        assert!(cmd2.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "info")); // overridden
        assert!(cmd2.env.iter().any(|(k, v)| k == "RUST_BACKTRACE" && v == "0")); // inherited
        
        // Test 3: Module override
        let ctx3 = scope::ScopeContext::new()
            .with_crate("special-crate".into())
            .with_module("special-crate::integration".into()); // Full module path needed
        let cmd3 = resolver.resolve_command(&ctx3, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        }).unwrap();
        
        // Verify module-level environment override
        assert!(cmd3.env.iter().any(|(k, v)| k == "RUST_BACKTRACE" && v == "1")); // overridden
        
        // Test 4: Function override
        let ctx4 = scope::ScopeContext::new()
            .with_crate("special-crate".into())
            .with_module("special-crate::integration".into())
            .with_function("test_database".into());
        let cmd4 = resolver.resolve_command(&ctx4, RunnableKind::Test {
            test_name: "test_database".into(),
            is_async: false,
        }).unwrap();
        
        assert!(cmd4.env.iter().any(|(k, v)| k == "DATABASE_URL" && v == "postgres://test"));
        assert!(cmd4.env.iter().any(|(k, v)| k == "RUST_LOG" && v == "debug")); // most specific
    }

    /// Test different framework strategies
    #[test]
    fn test_framework_strategies_integration() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .framework_binary("cargo-run")
                 .framework_benchmark("cargo-bench");
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new()
            .with_crate("my-crate".into())
            .with_file(PathBuf::from("src/main.rs"));
        
        // Test command
        let test_cmd = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "my_test".into(),
            is_async: false,
        }).unwrap();
        assert!(test_cmd.args.contains(&"test".into()));
        
        // Binary command
        let bin_cmd = resolver.resolve_command(&context, RunnableKind::Binary {
            bin_name: Some("my-app".into()),
        }).unwrap();
        assert!(bin_cmd.args.contains(&"run".into()));
        assert!(bin_cmd.args.contains(&"--bin".into()));
        assert!(bin_cmd.args.contains(&"my-app".into())); // the binary name we specified
        
        // Benchmark command
        let bench_context = scope::ScopeContext::new()
            .with_crate("my-crate".into())
            .with_function("my_bench".into()); // Need to set function name for benchmark
        let bench_cmd = resolver.resolve_command(&bench_context, RunnableKind::Benchmark {
            bench_name: "my_bench".into(),
        }).unwrap();
        assert!(bench_cmd.args.contains(&"bench".into()));
        assert!(bench_cmd.args.contains(&"--bench".into()));
        assert!(bench_cmd.args.contains(&"my_bench".into()));
    }

    /// Test file-based overrides
    #[test]
    fn test_file_based_overrides_integration() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_binary("cargo-run");
            })
            .file_override("src/bin/web_app.rs", |f| {
                f.framework_binary("cargo-run") // Use a known strategy
                 .args_binary(vec!["--hot-reload".into()])
                 .env("DIOXUS_LOG", "debug");
            })
            .file_override("examples/demo.rs", |f| {
                f.framework_binary("cargo-run")
                 .args_binary(vec!["--example".into(), "demo".into()]);
            })
            .build();
        
        let resolver = config.resolver();
        
        // Regular binary
        let ctx1 = scope::ScopeContext::new()
            .with_file(PathBuf::from("src/main.rs"));
        let cmd1 = resolver.resolve_command(&ctx1, RunnableKind::Binary {
            bin_name: None,
        }).unwrap();
        assert!(cmd1.args.contains(&"run".into()));
        assert!(!cmd1.args.contains(&"--hot-reload".into()));
        
        // Web app with overrides
        let ctx2 = scope::ScopeContext::new()
            .with_file(PathBuf::from("src/bin/web_app.rs"));
        let cmd2 = resolver.resolve_command(&ctx2, RunnableKind::Binary {
            bin_name: None,
        }).unwrap();
        assert!(cmd2.args.contains(&"--hot-reload".into()));
        assert!(cmd2.env.iter().any(|(k, v)| k == "DIOXUS_LOG" && v == "debug"));
        
        // Example file
        let ctx3 = scope::ScopeContext::new()
            .with_file(PathBuf::from("examples/demo.rs"));
        let cmd3 = resolver.resolve_command(&ctx3, RunnableKind::Binary {
            bin_name: None,
        }).unwrap();
        assert!(cmd3.args.contains(&"--example".into()));
        assert!(cmd3.args.contains(&"demo".into()));
    }

    /// Test argument merging
    #[test]
    fn test_argument_merging_integration() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .args_all(vec!["--verbose".into()])
                 .args_test(vec!["--nocapture".into()]);
            })
            .crate_override("my-crate", |c| {
                c.args_test(vec!["--no-fail-fast".into()])
                 .args_test_binary(vec!["--test-threads=1".into()]);
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new()
            .with_crate("my-crate".into())
            .with_function("test_something".into());
        
        let command = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "test_something".into(),
            is_async: false,
        }).unwrap();
        
        // Should have all args
        assert!(command.args.contains(&"--verbose".into())); // from args_all
        assert!(command.args.contains(&"--nocapture".into())); // from workspace args_test
        assert!(command.args.contains(&"--no-fail-fast".into())); // from crate args_test
        
        // Test binary args should be after --
        let separator_pos = command.args.iter().position(|arg| arg == "--").unwrap();
        let after_separator = &command.args[separator_pos + 1..];
        assert!(after_separator.contains(&"--test-threads=1".into()));
    }

    /// Test missing build system error
    #[test]
    fn test_missing_build_system_error() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                // No build system specified
                w.framework_test("cargo-test");
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new();
        
        let result = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        });
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No build system specified");
    }

    /// Test missing framework strategy error
    #[test]
    fn test_missing_framework_error() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo);
                // No test framework specified
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new();
        
        let result = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        });
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No framework strategy"));
    }

    /// Test unknown strategy error
    #[test]
    fn test_unknown_strategy_error() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("unknown-strategy");
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new();
        
        let result = resolver.resolve_command(&context, RunnableKind::Test {
            test_name: "test".into(),
            is_async: false,
        });
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown strategy: unknown-strategy"));
    }

    /// Test all runnable kinds
    #[test]
    fn test_all_runnable_kinds_integration() {
        let config = ConfigBuilder::new()
            .workspace(|w| {
                w.build_system(BuildSystem::Cargo)
                 .framework_test("cargo-test")
                 .framework_binary("cargo-run")
                 .framework_benchmark("cargo-bench");
            })
            .build();
        
        let resolver = config.resolver();
        let context = scope::ScopeContext::new()
            .with_crate("my-crate".into());
        
        // Test all RunnableKind variants
        let kinds = vec![
            RunnableKind::Test { test_name: "test".into(), is_async: false },
            RunnableKind::Binary { bin_name: Some("app".into()) },
            RunnableKind::Benchmark { bench_name: "bench".into() },
            RunnableKind::DocTest { struct_or_module_name: "MyStruct".into(), method_name: None },
            RunnableKind::ModuleTests { module_name: "tests".into() },
            RunnableKind::Standalone { has_tests: false },
            RunnableKind::SingleFileScript { shebang: "#!/usr/bin/env rust-script".into() },
        ];
        
        for kind in kinds {
            let is_doctest = matches!(&kind, RunnableKind::DocTest { .. });
            let result = resolver.resolve_command(&context, kind);
            // DocTest doesn't have a default strategy, so it will fail
            if is_doctest {
                assert!(result.is_err());
            } else {
                assert!(result.is_ok(), "Failed for {:?}", result);
            }
        }
    }
}