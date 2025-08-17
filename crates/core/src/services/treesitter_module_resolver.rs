//! Tree-sitter based module resolver implementation
//!
//! Wraps the existing ModuleResolver to implement the new interface.

use crate::{
    interfaces::{ExecutionContext, ModuleResolver},
    types::{Scope, ScopeKind},
};

/// Tree-sitter based implementation of ModuleResolver trait
/// Wraps the BasicModuleResolver from the parser module
pub struct TreeSitterModuleResolver {
    inner: crate::parser::BasicModuleResolver,
}

impl TreeSitterModuleResolver {
    pub fn new() -> Self {
        Self {
            inner: crate::parser::BasicModuleResolver::new(),
        }
    }
}

impl ModuleResolver for TreeSitterModuleResolver {
    fn resolve_module_path(&self, context: &ExecutionContext, scope: &Scope) -> String {
        // Use the existing resolver
        self.inner
            .resolve_module_path(&context.file_path, &context.scopes, scope)
            .unwrap_or_else(|_| String::new())
    }

    fn resolve_import_path(&self, from_module: &str, import_path: &str) -> String {
        // Simple implementation - can be enhanced
        if import_path.starts_with("crate::") {
            import_path.to_string()
        } else if import_path.starts_with("super::") {
            let parent = from_module.rsplit_once("::").map(|(p, _)| p).unwrap_or("");
            format!("{}::{}", parent, &import_path[7..])
        } else {
            import_path.to_string()
        }
    }

    fn module_path_from_file(
        &self,
        file_path: &std::path::Path,
        package_name: Option<&str>,
    ) -> String {
        // Use existing logic from ModuleResolver
        let components: Vec<_> = file_path
            .components()
            .filter_map(|c| {
                use std::path::Component;
                match c {
                    Component::Normal(s) => s.to_str(),
                    _ => None,
                }
            })
            .collect();

        // Find src/ or tests/ directory
        let start_idx = components
            .iter()
            .position(|&c| c == "src" || c == "tests")
            .map(|i| i + 1)
            .unwrap_or(0);

        let mut module_parts = Vec::new();

        // Add package name if not in tests
        if let Some(pkg) = package_name {
            if !components.contains(&"tests") {
                module_parts.push(pkg.to_string());
            }
        }

        // Add path components
        for component in &components[start_idx..] {
            if component == &"mod.rs" || component == &"lib.rs" || component == &"main.rs" {
                continue;
            }

            let part = if component.ends_with(".rs") {
                &component[..component.len() - 3]
            } else {
                component
            };

            module_parts.push(part.to_string());
        }

        module_parts.join("::")
    }

    fn is_in_test_module(&self, context: &ExecutionContext, scope: &Scope) -> bool {
        context
            .scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Module))
            .filter(|s| s.contains_line(scope.start.line))
            .any(|s| s.name.as_deref() == Some("tests"))
    }

    fn extract_impl_type(&self, impl_scope: &Scope) -> Option<String> {
        impl_scope.name.as_ref().and_then(|name| {
            if name.starts_with("impl ") {
                let type_name = name
                    .strip_prefix("impl ")
                    .unwrap_or(name)
                    .split(" for ")
                    .last()
                    .unwrap_or(name)
                    .trim();
                Some(type_name.to_string())
            } else {
                None
            }
        })
    }
}
