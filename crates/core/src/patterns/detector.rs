use crate::{
    error::Result,
    parser::{RustParser, scope_detector::ScopeDetector},
    patterns::{
        BenchmarkPattern, BinaryPattern, DocTestPattern, ModTestPattern, Pattern, TestFnPattern,
    },
    types::{Runnable, RunnableKind, RunnableWithScore, Scope, ScopeKind},
};
use std::path::Path;

pub struct RunnableDetector {
    patterns: Vec<Box<dyn Pattern>>,
    parser: RustParser,
}

impl RunnableDetector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            patterns: vec![
                Box::new(TestFnPattern),
                Box::new(ModTestPattern),
                Box::new(BenchmarkPattern),
                Box::new(BinaryPattern),
                Box::new(DocTestPattern),
            ],
            parser: RustParser::new()?,
        })
    }

    pub fn detect_runnables(
        &mut self,
        file_path: &Path,
        line: Option<u32>,
    ) -> Result<Vec<Runnable>> {
        let source = std::fs::read_to_string(file_path)?;
        let tree = self.parser.parse(&source)?;
        let mut detector = ScopeDetector::new();
        let extended_scopes = detector.detect_scopes(&tree, &source, file_path)?;

        let mut runnables = Vec::new();

        // Detect doc tests
        let doc_tests = detector.find_doc_tests(&tree.root_node(), &source);
        for (start, end, _text) in doc_tests {
            let scope = Scope {
                start,
                end,
                kind: ScopeKind::DocTest,
                name: Some(format!("doc test at line {}", start.line + 1)),
            };

            // Find the parent struct/impl/function for this doc test
            // Strategy: Find the scope whose extended range starts with this doc test
            let parent_extended = extended_scopes
                .iter()
                .filter(|es| {
                    matches!(
                        es.scope.kind,
                        ScopeKind::Struct | ScopeKind::Impl | ScopeKind::Function
                    )
                })
                // Check if this doc test is at the beginning of the extended scope
                .filter(|es| {
                    // The doc test should be within the extended scope
                    let contains = es.scope.contains_line(start.line);
                    // And the doc test should start near the beginning of the extended scope
                    let at_start = start.line >= es.scope.start.line
                        && start.line
                            < es.scope.start.line + es.doc_comment_lines + es.attribute_lines + 2;

                    contains && at_start
                })
                // Prefer the smallest scope (most specific)
                .min_by_key(|es| es.scope.end.line - es.scope.start.line)
                .cloned();

            if let Some(parent_ext) = parent_extended {
                let parent = &parent_ext.scope;
                let (struct_or_module_name, method_name) =
                    if matches!(parent.kind, ScopeKind::Function) {
                        // For functions inside impl blocks, find the impl name
                        let impl_scope = extended_scopes
                            .iter()
                            .filter(|es| matches!(es.scope.kind, ScopeKind::Impl))
                            .find(|es| es.scope.contains_line(parent.start.line))
                            .map(|es| &es.scope);

                        if let Some(impl_scope) = impl_scope {
                            // Extract the type name from "impl Type" or "impl Trait for Type"
                            let impl_name = impl_scope.name.as_deref().unwrap_or("impl");
                            let type_name = if impl_name.starts_with("impl ") {
                                impl_name
                                    .strip_prefix("impl ")
                                    .unwrap_or(impl_name)
                                    .split(" for ")
                                    .last()
                                    .unwrap_or(impl_name)
                            } else {
                                impl_name
                            };
                            (type_name.to_string(), parent.name.clone())
                        } else {
                            // Standalone function
                            (parent.name.clone().unwrap_or_default(), None)
                        }
                    } else if matches!(parent.kind, ScopeKind::Impl) {
                        // For impl blocks, extract the type name
                        let impl_name = parent.name.as_deref().unwrap_or("impl");
                        let type_name = if impl_name.starts_with("impl ") {
                            let stripped = impl_name.strip_prefix("impl ").unwrap_or(impl_name);
                            let final_name =
                                stripped.split(" for ").last().unwrap_or(stripped).trim();
                            final_name
                        } else {
                            impl_name
                        };
                        (type_name.to_string(), None)
                    } else {
                        // Struct
                        (parent.name.clone().unwrap_or_default(), None)
                    };

                // Create a proper label based on whether it's a method or type
                let label = if let Some(ref method) = method_name {
                    format!("Run doc test for '{struct_or_module_name}::{method}'")
                } else {
                    format!("Run doc test for '{struct_or_module_name}'")
                };

                let runnable = Runnable {
                    label,
                    scope: scope.clone(), // Use the doc test's own scope, not the parent's
                    kind: RunnableKind::DocTest {
                        struct_or_module_name,
                        method_name,
                    },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: Some(parent_ext.clone()),
                };
                runnables.push(runnable);
            }
        }

        // Detect other patterns
        for extended_scope in &extended_scopes {
            let scope = &extended_scope.scope;
            for pattern in &self.patterns {
                if let Some(mut runnable) = pattern.detect(scope, &source, file_path)? {
                    runnable.extended_scope = Some(extended_scope.clone());
                    runnables.push(runnable);
                }
            }
        }

        // Module test detection is now handled by ModTestPattern
        // But we also need to detect modules that contain tests even if not named "tests"
        for extended_scope in &extended_scopes {
            let scope = &extended_scope.scope;
            if let ScopeKind::Module = scope.kind {
                if let Some(module_name) = &scope.name {
                    // Check if this module contains any test functions
                    let has_tests = extended_scopes.iter().any(|s| {
                        matches!(s.scope.kind, ScopeKind::Test)
                            && s.scope.start.line >= scope.start.line
                            && s.scope.end.line <= scope.end.line
                    });

                    if has_tests {
                        // Check if we already have a runnable for this module
                        let already_exists = runnables.iter().any(|r| {
                            matches!(&r.kind, RunnableKind::ModuleTests { module_name: existing_name }
                                if existing_name == module_name)
                        });

                        if !already_exists {
                            let runnable = Runnable {
                                label: format!("Run all tests in module '{module_name}'"),
                                scope: scope.clone(),
                                kind: RunnableKind::ModuleTests {
                                    module_name: module_name.clone(),
                                },
                                module_path: String::new(), // Will be filled by module resolver
                                file_path: file_path.to_path_buf(),
                                extended_scope: Some(extended_scope.clone()),
                            };
                            runnables.push(runnable);
                        }
                    }
                }
            }
        }

        // Sort runnables by their start position to maintain file order
        runnables.sort_by_key(|r| (r.scope.start.line, r.scope.start.character));

        // Filter by line if specified
        if let Some(line) = line {
            let mut scored_runnables: Vec<RunnableWithScore> = runnables
                .into_iter()
                .filter(|r| {
                    // For doc tests with extended scope, check if line is within the extended scope
                    if matches!(r.kind, RunnableKind::DocTest { .. }) {
                        if let Some(ref extended) = r.extended_scope {
                            // Check if line is within the extended scope (includes doc comments)
                            line >= extended.scope.start.line && line <= extended.scope.end.line
                        } else {
                            r.scope.contains_line(line)
                        }
                    } else {
                        r.scope.contains_line(line)
                    }
                })
                .map(RunnableWithScore::new)
                .collect();

            scored_runnables.sort();

            Ok(scored_runnables.into_iter().map(|r| r.runnable).collect())
        } else {
            Ok(runnables)
        }
    }

    pub fn get_best_runnable_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Option<Runnable>> {
        let runnables = self.detect_runnables(file_path, Some(line))?;
        Ok(runnables.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_test_function() -> Result<()> {
        let source = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}
"#;

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{source}")?;

        let mut detector = RunnableDetector::new()?;
        let runnables = detector.detect_runnables(temp_file.path(), None)?;

        assert_eq!(runnables.len(), 1);
        assert!(matches!(
            &runnables[0].kind,
            RunnableKind::Test { test_name, .. } if test_name == "test_addition"
        ));

        Ok(())
    }

    #[test]
    fn test_detect_main_function() -> Result<()> {
        let source = r#"
fn main() {
    println!("Hello, world!");
}
"#;

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{source}")?;

        let mut detector = RunnableDetector::new()?;
        let runnables = detector.detect_runnables(temp_file.path(), None)?;

        assert_eq!(runnables.len(), 1);
        assert!(matches!(&runnables[0].kind, RunnableKind::Binary { .. }));

        Ok(())
    }

    #[test]
    fn test_line_filtering() -> Result<()> {
        let source = r#"
#[test]
fn test_one() {
    assert!(true);
}

#[test]
fn test_two() {
    assert!(true);
}
"#;

        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{source}")?;

        let mut detector = RunnableDetector::new()?;

        // Line 3 is inside test_one
        let runnables = detector.detect_runnables(temp_file.path(), Some(3))?;
        assert_eq!(runnables.len(), 1);
        assert!(matches!(
            &runnables[0].kind,
            RunnableKind::Test { test_name, .. } if test_name == "test_one"
        ));

        // Line 8 is inside test_two
        let runnables = detector.detect_runnables(temp_file.path(), Some(8))?;
        assert_eq!(runnables.len(), 1);
        assert!(matches!(
            &runnables[0].kind,
            RunnableKind::Test { test_name, .. } if test_name == "test_two"
        ));

        Ok(())
    }
}
