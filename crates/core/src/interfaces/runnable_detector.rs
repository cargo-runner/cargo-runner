//! Runnable detection interface
//! 
//! Provides abstraction for detecting runnable items in Rust code.

use crate::types::{Runnable, RunnableWithScore, Scope, ExtendedScope};

/// Trait for detecting runnable items in code
pub trait RunnableDetector: Send + Sync {
    /// Detect all runnables from the provided scopes
    fn detect(
        &self,
        scopes: &[Scope],
        extended_scopes: &[ExtendedScope],
        source: &str,
        file_path: &std::path::Path,
    ) -> Vec<Runnable>;
    
    /// Filter runnables to those containing a specific line
    fn filter_by_line(
        &self,
        runnables: Vec<Runnable>,
        line: u32,
    ) -> Vec<Runnable>;
    
    /// Score and sort runnables by specificity
    fn score_runnables(
        &self,
        runnables: Vec<Runnable>,
    ) -> Vec<RunnableWithScore>;
    
    /// Get the best runnable for a given line
    fn get_best_runnable(
        &self,
        runnables: Vec<Runnable>,
        line: Option<u32>,
    ) -> Option<Runnable>;
    
    /// Check if a scope has test attributes
    fn has_test_attribute(
        &self,
        scope: &Scope,
        extended_scope: Option<&ExtendedScope>,
    ) -> bool;
    
    /// Check if a scope has benchmark attributes
    fn has_bench_attribute(
        &self,
        scope: &Scope,
        extended_scope: Option<&ExtendedScope>,
    ) -> bool;
}