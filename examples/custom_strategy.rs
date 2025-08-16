//! Example of creating a custom strategy for the v2 configuration system

use cargo_runner_core::config::v2::strategy::{FrameworkStrategy, FrameworkKind, CommandContext};
use cargo_runner_core::command::CargoCommand;
use std::sync::Arc;

/// Example: Tauri development server strategy
pub struct TauriDevStrategy {
    name: String,
}

impl TauriDevStrategy {
    pub fn new() -> Self {
        Self {
            name: "tauri-dev".to_string(),
        }
    }
}

impl FrameworkStrategy for TauriDevStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        // Tauri uses cargo but with a special subcommand
        let mut args = vec!["tauri".to_string(), "dev".to_string()];
        
        // Add any additional args based on context
        if let Some(package) = &context.package_name {
            args.push("--package".to_string());
            args.push(package.clone());
        }
        
        let mut command = CargoCommand::new(args);
        
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }
        
        Ok(command)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }
}

/// Example: Custom test runner that uses a different tool
pub struct CustomTestStrategy {
    name: String,
}

impl CustomTestStrategy {
    pub fn new() -> Self {
        Self {
            name: "my-test-runner".to_string(),
        }
    }
}

impl FrameworkStrategy for CustomTestStrategy {
    fn build_command(&self, context: &CommandContext) -> Result<CargoCommand, String> {
        // This uses a completely different tool, not cargo
        let mut args = vec![];
        
        // Add test name if specified
        if let Some(test_name) = &context.function_name {
            args.push("--filter".to_string());
            args.push(test_name.clone());
        }
        
        // Add file path
        if let Some(file_path) = &context.file_path {
            args.push(file_path.clone());
        }
        
        let mut command = CargoCommand::new_shell("my-test-tool".to_string(), args);
        
        if let Some(wd) = &context.working_dir {
            command = command.with_working_dir(wd.clone());
        }
        
        Ok(command)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn framework_kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }
}

fn main() {
    // Example of how to register custom strategies
    use cargo_runner_core::config::v2::registry::StrategyRegistry;
    
    let mut registry = StrategyRegistry::new();
    
    // Register our custom strategies
    registry.register_strategy(Arc::new(TauriDevStrategy::new()));
    registry.register_strategy(Arc::new(CustomTestStrategy::new()));
    
    // Now these can be used in config:
    // {
    //   "frameworks": {
    //     "binary": "tauri-dev",
    //     "test": "my-test-runner"
    //   }
    // }
    
    println!("Registered strategies: {:?}", registry.list_strategies());
}