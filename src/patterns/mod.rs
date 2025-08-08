pub mod detector;

use crate::{Result, Runnable, RunnableKind, Scope, ScopeKind};
use std::path::Path;

pub trait Pattern {
    fn detect(&self, scope: &Scope, source: &str, file_path: &Path) -> Result<Option<Runnable>>;
}

pub struct TestFnPattern;
pub struct ModTestPattern;
pub struct DocTestPattern;
pub struct BenchmarkPattern;
pub struct BinaryPattern;

impl Pattern for TestFnPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Test = scope.kind {
            if let Some(name) = &scope.name {
                let runnable = Runnable {
                    label: format!("Run test '{}'", name),
                    scope: scope.clone(),
                    kind: RunnableKind::Test {
                        test_name: name.clone(),
                        is_async: false, // TODO: detect async tests
                    },
                    module_path: String::new(), // Will be filled by module resolver
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}

impl Pattern for ModTestPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Module = scope.kind {
            // Check if this is a test module (named "tests" or has #[cfg(test)])
            if scope.name.as_deref() == Some("tests") {
                let runnable = Runnable {
                    label: format!("Run all tests in module '{}'", scope.name.as_ref().unwrap()),
                    scope: scope.clone(),
                    kind: RunnableKind::ModuleTests {
                        module_name: scope.name.as_ref().unwrap().clone(),
                    },
                    module_path: String::new(), // Will be filled by module resolver
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}

impl Pattern for BenchmarkPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Benchmark = scope.kind {
            if let Some(name) = &scope.name {
                let runnable = Runnable {
                    label: format!("Run benchmark '{}'", name),
                    scope: scope.clone(),
                    kind: RunnableKind::Benchmark {
                        bench_name: name.clone(),
                    },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}

impl Pattern for BinaryPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Function = scope.kind {
            if scope.name.as_deref() == Some("main") {
                let bin_name = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());

                let runnable = Runnable {
                    label: if let Some(ref name) = bin_name {
                        format!("Run binary '{}'", name)
                    } else {
                        "Run main()".to_string()
                    },
                    scope: scope.clone(),
                    kind: RunnableKind::Binary { bin_name },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}

impl Pattern for DocTestPattern {
    fn detect(&self, scope: &Scope, _source: &str, _file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::DocTest = scope.kind {
            // Doc test detection is handled by the detector module
            // This pattern exists for completeness but returns None
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Position, Scope, ScopeKind};

    #[test]
    fn test_test_fn_pattern() {
        let pattern = TestFnPattern;
        let scope = Scope {
            start: Position { line: 0, character: 0 },
            end: Position { line: 5, character: 0 },
            kind: ScopeKind::Test,
            name: Some("test_example".to_string()),
        };
        
        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_some());
        
        let runnable = result.unwrap();
        assert_eq!(runnable.label, "Run test 'test_example'");
        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                assert_eq!(test_name, "test_example");
            }
            _ => panic!("Expected Test runnable kind"),
        }
    }
    
    #[test]
    fn test_mod_test_pattern() {
        let pattern = ModTestPattern;
        let scope = Scope {
            start: Position { line: 0, character: 0 },
            end: Position { line: 10, character: 0 },
            kind: ScopeKind::Module,
            name: Some("tests".to_string()),
        };
        
        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_some());
        
        let runnable = result.unwrap();
        assert_eq!(runnable.label, "Run all tests in module 'tests'");
        match &runnable.kind {
            RunnableKind::ModuleTests { module_name } => {
                assert_eq!(module_name, "tests");
            }
            _ => panic!("Expected ModuleTests runnable kind"),
        }
    }
    
    #[test]
    fn test_non_test_module_ignored() {
        let pattern = ModTestPattern;
        let scope = Scope {
            start: Position { line: 0, character: 0 },
            end: Position { line: 10, character: 0 },
            kind: ScopeKind::Module,
            name: Some("utils".to_string()),
        };
        
        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_none());
    }
}
