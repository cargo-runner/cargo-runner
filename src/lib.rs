pub mod cache;
pub mod command;
pub mod config;
pub mod parser;
pub mod patterns;
pub mod runner;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub start: Position,
    pub end: Position,
    pub kind: ScopeKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedScope {
    pub scope: Scope,
    /// The original start position without doc comments/attributes
    pub original_start: Position,
    /// Number of doc comment lines
    pub doc_comment_lines: u32,
    /// Number of attribute lines
    pub attribute_lines: u32,
    /// Whether this scope has doc tests
    pub has_doc_tests: bool,
}

impl ExtendedScope {
    pub fn new(scope: Scope) -> Self {
        let original_start = scope.start;
        Self {
            scope,
            original_start,
            doc_comment_lines: 0,
            attribute_lines: 0,
            has_doc_tests: false,
        }
    }

    pub fn with_doc_comments(mut self, lines: u32, has_tests: bool) -> Self {
        self.doc_comment_lines = lines;
        self.has_doc_tests = has_tests;
        self
    }

    pub fn with_attributes(mut self, lines: u32) -> Self {
        self.attribute_lines = lines;
        self
    }

    pub fn with_extended_start(mut self, start: Position) -> Self {
        self.scope.start = start;
        self
    }
}

impl From<Scope> for ExtendedScope {
    fn from(scope: Scope) -> Self {
        ExtendedScope::new(scope)
    }
}

impl From<ExtendedScope> for Scope {
    fn from(extended: ExtendedScope) -> Self {
        extended.scope
    }
}

impl ExtendedScope {
    pub fn to_scope(self) -> Scope {
        self.scope
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileScope {
    /// Library crate (src/lib.rs)
    Lib,
    /// Binary target (src/main.rs, src/bin/*.rs, or custom path from Cargo.toml)
    Bin { name: Option<String> },
    /// Benchmark (benches/*.rs or custom path from Cargo.toml)
    Bench { name: Option<String> },
    /// Build script (build.rs or custom path)
    Build,
    /// Integration test (tests/*.rs or custom path from Cargo.toml)
    Test { name: Option<String> },
    /// Example (examples/*.rs or custom path from Cargo.toml)
    Example { name: Option<String> },
    /// Standalone Rust file (outside of a Cargo project)
    Standalone { name: Option<String> },
    /// Unknown/generic file
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    File(FileScope),
    Module,
    Struct,
    Enum,
    Union,
    Impl,
    Function,
    Test,
    Benchmark,
    DocTest,
}

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
        if self.is_module_test && !other.is_module_test {
            Ordering::Greater
        } else if !self.is_module_test && other.is_module_test {
            Ordering::Less
        } else {
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionIdentity {
    pub package: Option<String>,
    pub module_path: Option<String>,
    pub file_path: Option<PathBuf>,
    pub function_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScopeContext {
    pub current_scope: Option<Scope>,
    pub parent_scopes: Vec<Scope>,
    pub all_scopes: Vec<Scope>,
}

impl Scope {
    pub fn contains(&self, position: Position) -> bool {
        position >= self.start && position <= self.end
    }

    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start.line && line <= self.end.line
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    #[error("Pattern detection error: {0}")]
    PatternError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Re-export main API
pub use command::CargoCommand;
pub use config::Config;
pub use runner::CargoRunner;
