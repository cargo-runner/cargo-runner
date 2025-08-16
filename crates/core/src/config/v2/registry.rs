//! Strategy registry for framework commands
//!
//! Manages registration and lookup of framework strategies.

use super::strategy::{
    BazelBenchStrategy, BazelBuildStrategy, BazelRunStrategy, BazelTestStrategy,
    CargoBenchStrategy, CargoBuildStrategy, CargoDocTestStrategy, CargoLeptosStrategy,
    CargoNextestStrategy, CargoRunStrategy, CargoScriptRunStrategy, CargoScriptTestStrategy,
    CargoShuttleStrategy, CargoTauriStrategy, CargoTestStrategy, DioxusServeStrategy, 
    DxServeStrategy, FrameworkStrategy, LeptosWatchStrategy, RustcRunStrategy, 
    RustcTestStrategy, TrunkServeStrategy,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for framework strategies
#[derive(Clone)]
pub struct StrategyRegistry {
    strategies: HashMap<String, Arc<dyn FrameworkStrategy>>,
}

impl std::fmt::Debug for StrategyRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StrategyRegistry")
            .field("strategies", &self.strategies.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl StrategyRegistry {
    /// Create a new registry with default strategies
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
        };

        // Register default Cargo strategies
        registry.register_strategy(Arc::new(CargoTestStrategy::new()));
        registry.register_strategy(Arc::new(CargoNextestStrategy::new()));
        registry.register_strategy(Arc::new(CargoRunStrategy::new()));
        registry.register_strategy(Arc::new(CargoBenchStrategy::new()));
        registry.register_strategy(Arc::new(CargoDocTestStrategy::new()));
        registry.register_strategy(Arc::new(CargoBuildStrategy::new()));

        // Register Bazel strategies
        registry.register_strategy(Arc::new(BazelTestStrategy::new()));
        registry.register_strategy(Arc::new(BazelRunStrategy::new()));
        registry.register_strategy(Arc::new(BazelBenchStrategy::new()));
        registry.register_strategy(Arc::new(BazelBuildStrategy::new()));

        // Register framework-specific strategies
        registry.register_strategy(Arc::new(LeptosWatchStrategy::new()));
        registry.register_strategy(Arc::new(DioxusServeStrategy::new()));
        registry.register_strategy(Arc::new(TrunkServeStrategy::new()));
        registry.register_strategy(Arc::new(CargoTauriStrategy::new()));
        registry.register_strategy(Arc::new(CargoLeptosStrategy::new()));
        registry.register_strategy(Arc::new(CargoShuttleStrategy::new()));
        registry.register_strategy(Arc::new(DxServeStrategy::new()));

        // Register Rustc strategies
        registry.register_strategy(Arc::new(RustcRunStrategy::new()));
        registry.register_strategy(Arc::new(RustcTestStrategy::new()));

        // Register CargoScript strategies
        registry.register_strategy(Arc::new(CargoScriptRunStrategy::new()));
        registry.register_strategy(Arc::new(CargoScriptTestStrategy::new()));

        registry
    }

    /// Register a new strategy
    pub fn register_strategy(&mut self, strategy: Arc<dyn FrameworkStrategy>) {
        self.strategies
            .insert(strategy.name().to_string(), strategy);
    }

    /// Get a strategy by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn FrameworkStrategy>> {
        self.strategies.get(name).cloned()
    }

    /// Check if a strategy exists
    pub fn contains(&self, name: &str) -> bool {
        self.strategies.contains_key(name)
    }

    /// Get all registered strategy names
    pub fn list_strategies(&self) -> Vec<&str> {
        self.strategies.keys().map(|s| s.as_str()).collect()
    }

    /// Create a registry from a custom strategy map
    pub fn from_strategies(strategies: HashMap<String, Arc<dyn FrameworkStrategy>>) -> Self {
        Self { strategies }
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating custom strategy registries
pub struct StrategyRegistryBuilder {
    strategies: HashMap<String, Arc<dyn FrameworkStrategy>>,
}

impl StrategyRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
        }
    }

    /// Add a strategy to the builder
    pub fn with_strategy(mut self, strategy: Arc<dyn FrameworkStrategy>) -> Self {
        self.strategies
            .insert(strategy.name().to_string(), strategy);
        self
    }

    /// Add default Cargo strategies
    pub fn with_cargo_defaults(mut self) -> Self {
        self = self.with_strategy(Arc::new(CargoTestStrategy::new()));
        self = self.with_strategy(Arc::new(CargoNextestStrategy::new()));
        self = self.with_strategy(Arc::new(CargoRunStrategy::new()));
        self = self.with_strategy(Arc::new(CargoBenchStrategy::new()));
        self = self.with_strategy(Arc::new(CargoDocTestStrategy::new()));
        self
    }

    /// Build the registry
    pub fn build(self) -> StrategyRegistry {
        StrategyRegistry::from_strategies(self.strategies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runners::framework::FrameworkKind;

    #[test]
    fn test_default_registry() {
        let registry = StrategyRegistry::new();

        // Check default strategies are registered
        assert!(registry.contains("cargo-test"));
        assert!(registry.contains("cargo-nextest"));
        assert!(registry.contains("cargo-run"));
        assert!(registry.contains("cargo-bench"));
    }

    #[test]
    fn test_get_strategy() {
        let registry = StrategyRegistry::new();

        let strategy = registry.get("cargo-test").unwrap();
        assert_eq!(strategy.name(), "cargo-test");
        assert_eq!(strategy.framework_kind(), FrameworkKind::Test);
    }

    #[test]
    fn test_custom_registry() {
        let registry = StrategyRegistryBuilder::new()
            .with_strategy(Arc::new(CargoTestStrategy::new()))
            .build();

        assert!(registry.contains("cargo-test"));
        assert!(!registry.contains("cargo-nextest")); // Not added
    }

    #[test]
    fn test_list_strategies() {
        let registry = StrategyRegistry::new();
        let strategies = registry.list_strategies();

        assert!(strategies.contains(&"cargo-test"));
        assert!(strategies.contains(&"cargo-nextest"));
        assert!(strategies.contains(&"cargo-run"));
        assert!(strategies.contains(&"cargo-bench"));
        assert!(strategies.contains(&"cargo-build"));
        assert!(strategies.contains(&"bazel-build"));
    }
}
