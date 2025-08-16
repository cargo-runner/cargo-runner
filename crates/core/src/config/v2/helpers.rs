//! Helper functions for v2 configuration

use crate::types::FunctionIdentity;
use super::scope::ScopeContext;

/// Helper to create ScopeContext from FunctionIdentity
pub fn scope_context_from_identity(identity: &FunctionIdentity) -> ScopeContext {
    let mut context = ScopeContext::new();
    
    if let Some(package) = &identity.package {
        context = context.with_crate(package.clone());
    }
    
    if let Some(module_path) = &identity.module_path {
        context = context.with_module(module_path.clone());
    }
    
    if let Some(file_path) = &identity.file_path {
        context = context.with_file(file_path.clone());
    }
    
    if let Some(function_name) = &identity.function_name {
        context = context.with_function(function_name.clone());
    }
    
    context
}