//! Pattern-based runnable detector implementation
//!
//! Wraps the existing RunnableDetector to implement the new interface.

use crate::{
    interfaces::RunnableDetector,
    types::{ExtendedScope, Runnable, RunnableWithScore, Scope, ScopeKind},
};

/// Pattern-based implementation of RunnableDetector
pub struct PatternRunnableDetector {
    inner: crate::patterns::RunnableDetector,
}

impl PatternRunnableDetector {
    pub fn new() -> crate::error::Result<Self> {
        Ok(Self {
            inner: crate::patterns::RunnableDetector::new()?,
        })
    }
}

impl RunnableDetector for PatternRunnableDetector {
    fn detect(
        &self,
        _scopes: &[Scope],
        _extended_scopes: &[ExtendedScope],
        _source: &str,
        file_path: &std::path::Path,
    ) -> Vec<Runnable> {
        // We need to use the existing detector's API
        // This is a bit of a hack - in a real refactor we'd update the inner API
        // For now, we'll create a temporary mutable clone
        let mut detector = match crate::patterns::RunnableDetector::new() {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        // The existing detector expects to read the file itself
        // We'll need to work around this by detecting all runnables
        match detector.detect_runnables(file_path, None) {
            Ok(runnables) => runnables,
            Err(_) => Vec::new(),
        }
    }

    fn filter_by_line(&self, runnables: Vec<Runnable>, line: u32) -> Vec<Runnable> {
        runnables
            .into_iter()
            .filter(|r| r.scope.contains_line(line))
            .collect()
    }

    fn score_runnables(&self, runnables: Vec<Runnable>) -> Vec<RunnableWithScore> {
        let mut scored: Vec<_> = runnables.into_iter().map(RunnableWithScore::new).collect();

        scored.sort();
        scored
    }

    fn get_best_runnable(&self, runnables: Vec<Runnable>, line: Option<u32>) -> Option<Runnable> {
        let filtered = if let Some(line) = line {
            self.filter_by_line(runnables, line)
        } else {
            runnables
        };

        let scored = self.score_runnables(filtered);
        scored.into_iter().next().map(|s| s.runnable)
    }

    fn has_test_attribute(&self, scope: &Scope, extended_scope: Option<&ExtendedScope>) -> bool {
        // Check if this is already a test scope
        if matches!(scope.kind, ScopeKind::Test) {
            return true;
        }

        // Check extended scope for attributes
        if let Some(ext) = extended_scope {
            ext.attribute_lines > 0 // Simplified check
        } else {
            false
        }
    }

    fn has_bench_attribute(&self, scope: &Scope, extended_scope: Option<&ExtendedScope>) -> bool {
        // Check if this is already a benchmark scope
        if matches!(scope.kind, ScopeKind::Benchmark) {
            return true;
        }

        // Check extended scope for attributes
        if let Some(ext) = extended_scope {
            ext.attribute_lines > 0 // Simplified check
        } else {
            false
        }
    }
}
