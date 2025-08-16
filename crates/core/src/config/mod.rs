//! Configuration management for cargo-runner
//! 
//! This module provides the v2 configuration system.

// V2 configuration system
pub mod v2;

// Re-export v2 types as primary config
pub use v2::{
    V2Config, ConfigBuilder, ConfigLayer, ConfigLoader, ConfigResolver,
    FrameworkKind, FrameworkStrategy, JsonConfig, LayerConfig, LayerConfigExt,
    Scope, ScopeKind, ScopeContext, StrategyRegistry, scope_context_from_identity,
};