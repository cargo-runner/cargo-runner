//! Type-safe target selection with subcommand-specific constraints
//! 
//! This module ensures compile-time safety for cargo target selection flags.
//! Different subcommands (run, test, bench, build) have different valid targets.

use std::marker::PhantomData;

/// Marker types for cargo subcommands
pub mod command {
    pub struct Run;
    pub struct Test;
    pub struct Bench;
    pub struct Build;
}

/// Marker types for target selection states
pub mod state {
    /// No target selected yet
    pub struct NoTarget;
    
    /// Library target selected (--lib)
    pub struct Library;
    
    /// Binary target(s) selected (--bin, --bins)
    pub struct Binary;
    
    /// Example target(s) selected (--example, --examples)
    pub struct Example;
    
    /// Test target(s) selected (--test, --tests)
    pub struct TestTarget;
    
    /// Benchmark target(s) selected (--bench, --benches)
    pub struct Benchmark;
    
    /// Documentation target selected (--doc)
    pub struct Doc;
}

/// Target selector with command and state type parameters
pub struct TargetSelector<Cmd, State = state::NoTarget> {
    args: Vec<String>,
    _phantom: PhantomData<(Cmd, State)>,
}

/// Base implementation for all commands
impl<Cmd> TargetSelector<Cmd, state::NoTarget> {
    /// Create a new target selector for a specific command
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

/// Methods available for `cargo run`
impl TargetSelector<command::Run, state::NoTarget> {
    /// Select specific binary (--bin NAME)
    pub fn bin(mut self, name: impl Into<String>) -> TargetSelector<command::Run, state::Binary> {
        self.args.push("--bin".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    /// Select specific example (--example NAME)  
    pub fn example(mut self, name: impl Into<String>) -> TargetSelector<command::Run, state::Example> {
        self.args.push("--example".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
}

/// Methods available for `cargo test`
impl TargetSelector<command::Test, state::NoTarget> {
    /// Test library (--lib)
    pub fn lib(mut self) -> TargetSelector<command::Test, state::Library> {
        self.args.push("--lib".to_string());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Test specific binary (--bin NAME)
    pub fn bin(mut self, name: impl Into<String>) -> TargetSelector<command::Test, state::Binary> {
        self.args.push("--bin".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Test specific example (--example NAME)
    pub fn example(mut self, name: impl Into<String>) -> TargetSelector<command::Test, state::Example> {
        self.args.push("--example".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Test specific test target (--test NAME)
    pub fn test(mut self, name: impl Into<String>) -> TargetSelector<command::Test, state::TestTarget> {
        self.args.push("--test".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Test specific benchmark (--bench NAME)
    pub fn bench(mut self, name: impl Into<String>) -> TargetSelector<command::Test, state::Benchmark> {
        self.args.push("--bench".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Test documentation (--doc)
    pub fn doc(mut self) -> TargetSelector<command::Test, state::Doc> {
        self.args.push("--doc".to_string());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
}

/// Methods available for `cargo bench` (same as test minus --doc)
impl TargetSelector<command::Bench, state::NoTarget> {
    /// Benchmark library (--lib)
    pub fn lib(mut self) -> TargetSelector<command::Bench, state::Library> {
        self.args.push("--lib".to_string());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Benchmark specific binary (--bin NAME)
    pub fn bin(mut self, name: impl Into<String>) -> TargetSelector<command::Bench, state::Binary> {
        self.args.push("--bin".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Benchmark specific example (--example NAME)
    pub fn example(mut self, name: impl Into<String>) -> TargetSelector<command::Bench, state::Example> {
        self.args.push("--example".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Benchmark specific test target (--test NAME)
    pub fn test(mut self, name: impl Into<String>) -> TargetSelector<command::Bench, state::TestTarget> {
        self.args.push("--test".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Benchmark specific benchmark (--bench NAME)
    pub fn bench(mut self, name: impl Into<String>) -> TargetSelector<command::Bench, state::Benchmark> {
        self.args.push("--bench".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
}

/// Methods available for `cargo build` (same as bench)
impl TargetSelector<command::Build, state::NoTarget> {
    /// Build library (--lib)
    pub fn lib(mut self) -> TargetSelector<command::Build, state::Library> {
        self.args.push("--lib".to_string());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Build specific binary (--bin NAME)
    pub fn bin(mut self, name: impl Into<String>) -> TargetSelector<command::Build, state::Binary> {
        self.args.push("--bin".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Build specific example (--example NAME)
    pub fn example(mut self, name: impl Into<String>) -> TargetSelector<command::Build, state::Example> {
        self.args.push("--example".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Build specific test target (--test NAME)
    pub fn test(mut self, name: impl Into<String>) -> TargetSelector<command::Build, state::TestTarget> {
        self.args.push("--test".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
    
    /// Build specific benchmark (--bench NAME)
    pub fn bench(mut self, name: impl Into<String>) -> TargetSelector<command::Build, state::Benchmark> {
        self.args.push("--bench".to_string());
        self.args.push(name.into());
        TargetSelector {
            args: self.args,
            _phantom: PhantomData,
        }
    }
    
}

/// Common methods for all command/state combinations
impl<Cmd, State> TargetSelector<Cmd, State> {
    /// Build the arguments
    pub fn build(self) -> Vec<String> {
        self.args
    }
    
    /// Get the arguments without consuming self
    pub fn args(&self) -> &[String] {
        &self.args
    }
}

/// Smart constructors based on file path and command type
pub mod smart {
    use super::*;
    use crate::config::v2::target_detection::{detect_target_from_path, TargetType};
    
    /// Build target args for cargo run based on file path
    pub fn args_for_run(file_path: &str, package_name: Option<&str>) -> Vec<String> {
        match detect_target_from_path(file_path, package_name) {
            TargetType::Bin(name) => TargetSelector::<command::Run>::new().bin(name).build(),
            TargetType::Example(name) => TargetSelector::<command::Run>::new().example(name).build(),
            _ => vec![], // run only supports bin and example
        }
    }
    
    /// Build target args for cargo test based on file path
    pub fn args_for_test(file_path: &str, package_name: Option<&str>) -> Vec<String> {
        match detect_target_from_path(file_path, package_name) {
            TargetType::Lib => TargetSelector::<command::Test>::new().lib().build(),
            TargetType::Bin(name) => TargetSelector::<command::Test>::new().bin(name).build(),
            TargetType::Example(name) => TargetSelector::<command::Test>::new().example(name).build(),
            TargetType::Bench(name) => TargetSelector::<command::Test>::new().bench(name).build(),
            TargetType::NoTarget => vec![],
        }
    }
    
    /// Build target args for cargo bench based on file path
    pub fn args_for_bench(file_path: &str, package_name: Option<&str>) -> Vec<String> {
        match detect_target_from_path(file_path, package_name) {
            TargetType::Lib => TargetSelector::<command::Bench>::new().lib().build(),
            TargetType::Bin(name) => TargetSelector::<command::Bench>::new().bin(name).build(),
            TargetType::Example(name) => TargetSelector::<command::Bench>::new().example(name).build(),
            TargetType::Bench(name) => TargetSelector::<command::Bench>::new().bench(name).build(),
            TargetType::NoTarget => vec![],
        }
    }
    
    /// Build target args for cargo build based on file path
    pub fn args_for_build(file_path: &str, package_name: Option<&str>) -> Vec<String> {
        match detect_target_from_path(file_path, package_name) {
            TargetType::Lib => TargetSelector::<command::Build>::new().lib().build(),
            TargetType::Bin(name) => TargetSelector::<command::Build>::new().bin(name).build(),
            TargetType::Example(name) => TargetSelector::<command::Build>::new().example(name).build(),
            TargetType::Bench(name) => TargetSelector::<command::Build>::new().bench(name).build(),
            TargetType::NoTarget => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_run_target_selection() {
        // cargo run only allows --bin and --example
        let selector = TargetSelector::<command::Run>::new().bin("myapp");
        assert_eq!(selector.build(), vec!["--bin", "myapp"]);
        
        let selector = TargetSelector::<command::Run>::new().example("demo");
        assert_eq!(selector.build(), vec!["--example", "demo"]);
    }
    
    #[test]
    fn test_test_target_selection() {
        // cargo test allows all targets
        let selector = TargetSelector::<command::Test>::new().lib();
        assert_eq!(selector.build(), vec!["--lib"]);
        
        let selector = TargetSelector::<command::Test>::new().bin("app");
        assert_eq!(selector.build(), vec!["--bin", "app"]);
        
        let selector = TargetSelector::<command::Test>::new().doc();
        assert_eq!(selector.build(), vec!["--doc"]);
    }
    
    #[test]
    fn test_smart_constructors() {
        // Test run targets
        assert_eq!(smart::args_for_run("src/main.rs", Some("myapp")), vec!["--bin", "myapp"]);
        assert_eq!(smart::args_for_run("examples/demo.rs", None), vec!["--example", "demo"]);
        assert_eq!(smart::args_for_run("src/lib.rs", None), Vec::<String>::new()); // run doesn't support lib
        
        // Test test targets
        assert_eq!(smart::args_for_test("src/lib.rs", None), vec!["--lib"]);
        assert_eq!(smart::args_for_test("src/bin/app.rs", Some("app")), vec!["--bin", "app"]);
        assert_eq!(smart::args_for_test("examples/ex.rs", None), vec!["--example", "ex"]);
        assert_eq!(smart::args_for_test("benches/bench.rs", None), vec!["--bench", "bench"]);
    }
    
    // These would fail to compile, which is exactly what we want:
    // #[test]
    // fn test_invalid_combinations() {
    //     // cargo run doesn't support --lib
    //     // let selector = TargetSelector::<command::Run>::new().lib(); // Compile error!
    //     
    //     // cargo run doesn't support --doc
    //     // let selector = TargetSelector::<command::Run>::new().doc(); // Compile error!
    //     
    //     // Can't combine different target types
    //     // let selector = TargetSelector::<command::Test>::new().lib().bin("app"); // Compile error!
    // }
}