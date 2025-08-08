use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use super::scope::{ExtendedScope, Scope};

/// Represents a runnable item in Rust code (test, benchmark, binary, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runnable {
    pub label: String,
    pub scope: Scope,
    pub kind: RunnableKind,
    pub module_path: String,
    pub file_path: PathBuf,
    /// Extended scope information if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_scope: Option<ExtendedScope>,
}

/// Different kinds of runnable items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnableKind {
    Test {
        test_name: String,
        is_async: bool,
    },
    DocTest {
        struct_or_module_name: String,
        method_name: Option<String>,
    },
    Benchmark {
        bench_name: String,
    },
    Binary {
        bin_name: Option<String>,
    },
    ModuleTests {
        module_name: String,
    },
}

/// Runnable with scoring information for prioritization
#[derive(Debug, Clone)]
pub struct RunnableWithScore {
    pub runnable: Runnable,
    pub range_size: u32,
    pub is_module_test: bool,
}

impl RunnableWithScore {
    pub fn new(runnable: Runnable) -> Self {
        let range_size = runnable.scope.end.line - runnable.scope.start.line;
        let is_module_test = matches!(runnable.kind, RunnableKind::ModuleTests { .. });
        Self {
            runnable,
            range_size,
            is_module_test,
        }
    }
}

impl Ord for RunnableWithScore {
    fn cmp(&self, other: &Self) -> Ordering {
        // Module tests have lower priority than specific tests
        if self.is_module_test && !other.is_module_test {
            Ordering::Greater
        } else if !self.is_module_test && other.is_module_test {
            Ordering::Less
        } else {
            // Smaller scopes have higher priority
            self.range_size.cmp(&other.range_size)
        }
    }
}

impl PartialOrd for RunnableWithScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RunnableWithScore {
    fn eq(&self, other: &Self) -> bool {
        self.range_size == other.range_size && self.is_module_test == other.is_module_test
    }
}

impl Eq for RunnableWithScore {}