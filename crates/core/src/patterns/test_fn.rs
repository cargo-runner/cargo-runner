use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct TestFnPattern;

impl Pattern for TestFnPattern {
    fn detect(&self, scope: &Scope, source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Test = scope.kind
            && let Some(name) = &scope.name
        {
            let is_async = is_async_test_fn(source, scope, name);
            let runnable = Runnable {
                label: format!("Run test '{name}'"),
                scope: scope.clone(),
                kind: RunnableKind::Test {
                    test_name: name.clone(),
                    is_async,
                },
                module_path: String::new(), // Will be filled by module resolver
                file_path: file_path.to_path_buf(),
                extended_scope: None, // Will be filled by detector
            };
            return Ok(Some(runnable));
        }
        Ok(None)
    }
}

/// Detect `async fn` for a `#[test]` / `#[tokio::test]` function.
///
/// Inspects a small window of source lines around the scope start for patterns
/// like `async fn name` or `async fn name<...>(`.
fn is_async_test_fn(source: &str, scope: &Scope, name: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return false;
    }

    let start = scope.start.line as usize;
    // Attributes often sit above the fn; look a few lines up through the start.
    let from = start.saturating_sub(8);
    let to = (start + 2).min(lines.len().saturating_sub(1).saturating_add(1));

    for line in lines.iter().take(to).skip(from) {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        // `async fn foo` / `pub async fn foo` / `async fn foo<T>`
        if let Some(idx) = trimmed.find("async") {
            let after_async = trimmed[idx + "async".len()..].trim_start();
            if let Some(after_fn_kw) = after_async.strip_prefix("fn") {
                let after_fn = after_fn_kw.trim_start();
                if let Some(rest) = after_fn.strip_prefix(name) {
                    // next char should be end of ident: '(', '<', or whitespace then (
                    if rest.is_empty()
                        || rest.starts_with('(')
                        || rest.starts_with('<')
                        || rest.starts_with(char::is_whitespace)
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Position;

    fn scope_for(name: &str, line: u32) -> Scope {
        Scope {
            start: Position { line, character: 0 },
            end: Position {
                line: line + 5,
                character: 0,
            },
            kind: ScopeKind::Test,
            name: Some(name.to_string()),
        }
    }

    #[test]
    fn test_test_fn_pattern_sync() {
        let pattern = TestFnPattern;
        let source = "#[test]\nfn test_example() {\n    assert!(true);\n}\n";
        let scope = scope_for("test_example", 1);
        let result = pattern
            .detect(&scope, source, Path::new("test.rs"))
            .unwrap()
            .unwrap();
        match &result.kind {
            RunnableKind::Test {
                test_name,
                is_async,
            } => {
                assert_eq!(test_name, "test_example");
                assert!(!is_async);
            }
            _ => panic!("Expected Test"),
        }
    }

    #[test]
    fn test_test_fn_pattern_async() {
        let pattern = TestFnPattern;
        let source = "#[tokio::test]\nasync fn test_async_example() {\n    assert!(true);\n}\n";
        let scope = scope_for("test_async_example", 1);
        let result = pattern
            .detect(&scope, source, Path::new("test.rs"))
            .unwrap()
            .unwrap();
        match &result.kind {
            RunnableKind::Test {
                test_name,
                is_async,
            } => {
                assert_eq!(test_name, "test_async_example");
                assert!(is_async, "expected async test detection");
            }
            _ => panic!("Expected Test"),
        }
    }

    #[test]
    fn is_async_test_fn_pub_async() {
        let source = "#[test]\npub async fn my_test() {}\n";
        let scope = scope_for("my_test", 1);
        assert!(is_async_test_fn(source, &scope, "my_test"));
    }

    #[test]
    fn is_async_test_fn_rejects_sync() {
        let source = "#[test]\nfn my_test() {}\n";
        let scope = scope_for("my_test", 1);
        assert!(!is_async_test_fn(source, &scope, "my_test"));
    }
}
