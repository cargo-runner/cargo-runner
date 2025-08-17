//! Module resolution interface
//! 
//! Provides abstraction for resolving Rust module paths and imports.

use crate::types::Scope;
use super::ExecutionContext;

/// Trait for module path resolution
pub trait ModuleResolver: Send + Sync {
    /// Resolve the full module path for a scope within the context
    fn resolve_module_path(
        &self,
        context: &ExecutionContext,
        scope: &Scope,
    ) -> String;
    
    /// Resolve an import path from one module to another
    fn resolve_import_path(
        &self,
        from_module: &str,
        import_path: &str,
    ) -> String;
    
    /// Get the module path from a file path
    fn module_path_from_file(
        &self,
        file_path: &std::path::Path,
        package_name: Option<&str>,
    ) -> String;
    
    /// Check if a scope is inside a test module
    fn is_in_test_module(
        &self,
        context: &ExecutionContext,
        scope: &Scope,
    ) -> bool;
    
    /// Extract type name from an impl block
    fn extract_impl_type(
        &self,
        impl_scope: &Scope,
    ) -> Option<String>;
}