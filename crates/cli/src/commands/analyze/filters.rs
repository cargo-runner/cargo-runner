//! Runnable filters for `runnables` / analyze.

use cargo_runner_core::{Runnable, RunnableKind};

use crate::commands::matching::{normalize_query, runnable_matches_query, runnable_symbol_name};

#[derive(Debug, Clone, Default)]
pub struct RunnableFilters {
    pub bin: bool,
    pub test: bool,
    pub bench: bool,
    pub doc: bool,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub exact: bool,
}

impl RunnableFilters {
    pub(crate) fn active(&self) -> bool {
        self.bin
            || self.test
            || self.bench
            || self.doc
            || self.name.is_some()
            || self.symbol.is_some()
            || self.exact
    }

    pub(crate) fn matches(&self, runnable: &Runnable) -> bool {
        let kind_matches = !self.bin && !self.test && !self.bench && !self.doc
            || matches_runnable_kind(&runnable.kind, self.bin, self.test, self.bench, self.doc);

        kind_matches
            && runnable_matches_query(runnable, self.name.as_deref(), self.exact)
            && matches_symbol_filter(runnable, self.symbol.as_deref(), self.exact)
    }
}

pub(crate) fn matches_runnable_kind(
    kind: &RunnableKind,
    bin: bool,
    test: bool,
    bench: bool,
    doc: bool,
) -> bool {
    match kind {
        RunnableKind::Binary { .. } => bin,
        RunnableKind::Test { .. } | RunnableKind::ModuleTests { .. } => test,
        RunnableKind::Benchmark { .. } => bench,
        RunnableKind::DocTest { .. } => doc,
        _ => false,
    }
}

pub(crate) fn matches_symbol_filter(runnable: &Runnable, query: Option<&str>, exact: bool) -> bool {
    if query.is_none() {
        return true;
    }

    let symbol_name = runnable_symbol_name(runnable);
    let Some(symbol_name) = symbol_name else {
        return false;
    };

    let query = normalize_query(query.expect("Query is verified is_some"));
    let candidate = normalize_query(&symbol_name);
    if exact {
        candidate == query
    } else {
        candidate.contains(&query)
    }
}

pub(crate) fn describe_runnable_kind(kind: &cargo_runner_core::RunnableKind) -> &'static str {
    match kind {
        cargo_runner_core::RunnableKind::Test { .. } => "test",
        cargo_runner_core::RunnableKind::DocTest { .. } => "doc test",
        cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark",
        cargo_runner_core::RunnableKind::Binary { .. } => "binary",
        cargo_runner_core::RunnableKind::ModuleTests { .. } => "module tests",
        cargo_runner_core::RunnableKind::Standalone { .. } => "standalone",
        cargo_runner_core::RunnableKind::SingleFileScript { .. } => "single-file script",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_runner_core::types::{Position, Scope, ScopeKind};
    use std::path::PathBuf;

    fn sample_runnable(
        kind: RunnableKind,
        label: &str,
        module_path: &str,
        scope_name: Option<&str>,
    ) -> Runnable {
        Runnable {
            label: label.to_string(),
            scope: Scope {
                start: Position::new(0, 0),
                end: Position::new(1, 0),
                kind: ScopeKind::Function,
                name: scope_name.map(str::to_string),
            },
            kind,
            module_path: module_path.to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            extended_scope: None,
        }
    }

    #[test]
    fn name_filter_is_case_and_separator_insensitive() {
        let filters = RunnableFilters {
            name: Some("My Function".to_string()),
            ..Default::default()
        };
        let runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "my_function".to_string(),
                is_async: false,
            },
            "Run test 'my_function'",
            "crate::tests",
            Some("my_function"),
        );

        assert!(filters.matches(&runnable));
    }

    #[test]
    fn exact_name_filter_requires_full_normalized_match() {
        let fuzzy = RunnableFilters {
            name: Some("my".to_string()),
            exact: true,
            ..Default::default()
        };
        let exact = RunnableFilters {
            name: Some("my function".to_string()),
            exact: true,
            ..Default::default()
        };
        let runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "my_function".to_string(),
                is_async: false,
            },
            "Run test 'my_function'",
            "crate::tests",
            Some("my_function"),
        );

        assert!(!fuzzy.matches(&runnable));
        assert!(exact.matches(&runnable));
    }

    #[test]
    fn symbol_filter_targets_doc_test_symbols_only() {
        let filters = RunnableFilters {
            symbol: Some("Users".to_string()),
            ..Default::default()
        };
        let doc_symbol = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: None,
            },
            "Run doc test for 'Users'",
            "crate::models",
            Some("Users"),
        );
        let doc_method = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: Some("new".to_string()),
            },
            "Run doc test for 'Users::new'",
            "crate::models",
            Some("Users"),
        );

        assert!(filters.matches(&doc_symbol));
        assert!(!filters.matches(&doc_method));
    }

    #[test]
    fn kind_filters_can_be_combined_with_name_and_symbol_filters() {
        let filters = RunnableFilters {
            bin: true,
            test: true,
            name: Some("app".to_string()),
            symbol: Some("Users".to_string()),
            ..Default::default()
        };
        let test_runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "test_add".to_string(),
                is_async: false,
            },
            "Run test 'test_add'",
            "crate::tests",
            Some("test_add"),
        );
        let binary_runnable = sample_runnable(
            RunnableKind::Binary {
                bin_name: Some("app".to_string()),
            },
            "Run binary 'app'",
            "crate::main",
            Some("app"),
        );
        let symbol_runnable = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: None,
            },
            "Run doc test for 'Users'",
            "crate::models",
            Some("Users"),
        );

        assert!(!filters.matches(&test_runnable));
        assert!(!filters.matches(&binary_runnable));
        assert!(!filters.matches(&symbol_runnable));
    }
}
