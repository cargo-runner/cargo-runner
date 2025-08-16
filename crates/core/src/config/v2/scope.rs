//! Scope-based configuration matching
//! 
//! Defines the scope hierarchy for configuration overrides.

use std::path::PathBuf;

/// Represents the scope at which a configuration applies
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Workspace-wide configuration (lowest precedence)
    Workspace,
    /// Crate/package-specific configuration
    Crate(String),
    /// File-specific configuration
    File(PathBuf),
    /// Module-specific configuration
    Module(String),
    /// Type-specific configuration (struct, enum, union)
    Type(String),
    /// Function-specific configuration
    Function(String),
    /// Method-specific configuration (highest precedence)
    Method(String),
}

impl Scope {
    /// Check if this scope matches the given context
    pub fn matches(&self, context: &ScopeContext) -> bool {
        match self {
            Scope::Workspace => true,
            Scope::Crate(name) => context.crate_name.as_ref() == Some(name),
            Scope::File(path) => {
                if let Some(ctx_path) = &context.file_path {
                    let path_str = path.to_string_lossy();
                    let ctx_path_str = ctx_path.to_string_lossy();
                    
                    // Check if the path contains glob patterns
                    if path_str.contains('*') || path_str.contains('?') {
                        // Use glob pattern matching
                        glob_match(&path_str, &ctx_path_str)
                    } else {
                        // Support both exact match and suffix match
                        ctx_path == path || ctx_path.ends_with(path)
                    }
                } else {
                    false
                }
            }
            Scope::Module(module) => {
                if let Some(ctx_module) = &context.module_path {
                    ctx_module == module || ctx_module.starts_with(&format!("{}::", module))
                } else {
                    false
                }
            }
            Scope::Type(type_name) => context.type_name.as_ref() == Some(type_name),
            Scope::Function(func) => {
                // Support both simple function names and module-qualified names
                if let Some(ctx_func) = &context.function_name {
                    // First check exact match
                    if ctx_func == func {
                        return true;
                    }
                    
                    // If the function name contains "::", it's module-qualified
                    if func.contains("::") {
                        // Need both module path and function name to match
                        if let Some(ctx_module) = &context.module_path {
                            let qualified_name = format!("{}::{}", ctx_module, ctx_func);
                            return qualified_name == *func;
                        }
                    }
                }
                false
            }
            Scope::Method(method) => context.method_name.as_ref() == Some(method),
        }
    }

    /// Get the specificity level of this scope (higher = more specific)
    pub fn specificity(&self) -> u32 {
        match self {
            Scope::Workspace => 0,
            Scope::Crate(_) => 1,
            Scope::File(_) => 2,
            Scope::Module(_) => 3,
            Scope::Type(_) => 4,
            Scope::Function(_) => 5,
            Scope::Method(_) => 6,
        }
    }
}

/// Simple glob pattern matching for file paths
fn glob_match(pattern: &str, path: &str) -> bool {
    // Convert Windows paths to Unix style for consistent matching
    let pattern = pattern.replace('\\', "/");
    let path = path.replace('\\', "/");
    
    // For now, implement simple glob matching
    // This handles patterns like "examples/leptos_*.rs"
    let mut pattern_parts = pattern.split('*');
    let mut path_remainder = path.as_str();
    
    // Check if the first part matches
    if let Some(first_part) = pattern_parts.next() {
        if !first_part.is_empty() {
            if !path_remainder.ends_with(first_part) && !path_remainder.contains(first_part) {
                return false;
            }
            if let Some(pos) = path_remainder.find(first_part) {
                path_remainder = &path_remainder[pos + first_part.len()..];
            }
        }
    }
    
    // Check remaining parts
    for part in pattern_parts {
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = path_remainder.find(part) {
            path_remainder = &path_remainder[pos + part.len()..];
        } else {
            return false;
        }
    }
    
    true
}

/// The kind of scope for more detailed matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Test,
    Binary,
    Benchmark,
    DocTest,
    Example,
    Build,
}

/// Context for scope matching
#[derive(Debug, Clone, Default)]
pub struct ScopeContext {
    pub crate_name: Option<String>,
    pub file_path: Option<PathBuf>,
    pub module_path: Option<String>,
    pub type_name: Option<String>,
    pub function_name: Option<String>,
    pub method_name: Option<String>,
    pub scope_kind: Option<ScopeKind>,
}

impl ScopeContext {
    /// Create a new scope context
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method for crate name
    pub fn with_crate(mut self, name: String) -> Self {
        self.crate_name = Some(name);
        self
    }

    /// Builder method for file path
    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    /// Builder method for module path
    pub fn with_module(mut self, path: String) -> Self {
        self.module_path = Some(path);
        self
    }

    /// Builder method for function name
    pub fn with_function(mut self, name: String) -> Self {
        self.function_name = Some(name);
        self
    }

    /// Builder method for scope kind
    pub fn with_kind(mut self, kind: ScopeKind) -> Self {
        self.scope_kind = Some(kind);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_specificity() {
        assert!(Scope::Workspace.specificity() < Scope::Crate("test".into()).specificity());
        assert!(Scope::Crate("test".into()).specificity() < Scope::File("test.rs".into()).specificity());
        assert!(Scope::File("test.rs".into()).specificity() < Scope::Module("test".into()).specificity());
        assert!(Scope::Module("test".into()).specificity() < Scope::Function("test".into()).specificity());
    }

    #[test]
    fn test_scope_matching() {
        let context = ScopeContext::new()
            .with_crate("my-crate".into())
            .with_file("src/lib.rs".into())
            .with_module("tests".into())
            .with_function("test_something".into());

        assert!(Scope::Workspace.matches(&context));
        assert!(Scope::Crate("my-crate".into()).matches(&context));
        assert!(Scope::File("src/lib.rs".into()).matches(&context));
        assert!(Scope::Module("tests".into()).matches(&context));
        assert!(Scope::Function("test_something".into()).matches(&context));

        // Test non-matches
        assert!(!Scope::Crate("other-crate".into()).matches(&context));
        assert!(!Scope::Function("other_function".into()).matches(&context));
    }

    #[test]
    fn test_file_suffix_matching() {
        let context = ScopeContext::new()
            .with_file("/home/user/project/src/lib.rs".into());

        // Should match on suffix
        assert!(Scope::File("src/lib.rs".into()).matches(&context));
        assert!(Scope::File("lib.rs".into()).matches(&context));

        // Should not match different files
        assert!(!Scope::File("main.rs".into()).matches(&context));
    }

    #[test]
    fn test_module_prefix_matching() {
        let context = ScopeContext::new()
            .with_module("parser::tests::unit".into());

        // Should match on prefix
        assert!(Scope::Module("parser".into()).matches(&context));
        assert!(Scope::Module("parser::tests".into()).matches(&context));
        assert!(Scope::Module("parser::tests::unit".into()).matches(&context));

        // Should not match different modules
        assert!(!Scope::Module("lexer".into()).matches(&context));
    }
    
    #[test]
    fn test_function_matching_with_module_path() {
        // Test simple function name matching
        let context1 = ScopeContext::new()
            .with_function("test_user".into());
        assert!(Scope::Function("test_user".into()).matches(&context1));
        assert!(!Scope::Function("test_admin".into()).matches(&context1));
        
        // Test module-qualified function name matching
        let context2 = ScopeContext::new()
            .with_module("tests".into())
            .with_function("test_user".into());
        
        // Should match simple name
        assert!(Scope::Function("test_user".into()).matches(&context2));
        
        // Should match module-qualified name
        assert!(Scope::Function("tests::test_user".into()).matches(&context2));
        
        // Should not match wrong module-qualified name
        assert!(!Scope::Function("auth::test_user".into()).matches(&context2));
        
        // Test with nested modules
        let context3 = ScopeContext::new()
            .with_module("auth::tests".into())
            .with_function("test_user".into());
        
        assert!(Scope::Function("test_user".into()).matches(&context3));
        assert!(Scope::Function("auth::tests::test_user".into()).matches(&context3));
        assert!(!Scope::Function("tests::test_user".into()).matches(&context3));
    }
}