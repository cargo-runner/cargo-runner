# Bazel Target Detection Redesign

## Current Problems
1. Manual string parsing with regex - brittle and error-prone
2. Hard-coded rule types - not extensible
3. Can't handle complex Starlark expressions
4. Misses rule relationships and attributes
5. No proper AST understanding

## Proposed Solution
Use tree-sitter-starlark to properly parse BUILD files and extract runnable targets.

## Runnable Rule Types
From https://bazelbuild.github.io/rules_rust/rules.html:

### Core Runnable Rules
- `rust_binary` - Executable binaries (bazel run)
- `rust_test` - Unit tests (bazel test)
- `rust_test_suite` - Integration test suites (bazel test)
- `rust_doc_test` - Documentation tests (bazel test)
- `rust_benchmark` - Benchmarks (bazel run)

### Quasi-Runnable Rules
- `cargo_build_script` - Build scripts (bazel build, but generates output)

### Non-Runnable Rules (for context)
- `rust_library` - Libraries (referenced by tests)
- Code generation rules (rust_bindgen, rust_prost, etc.)

## Architecture Design

### 1. Starlark Parser Module
```rust
// crates/core/src/bazel/starlark_parser.rs
pub struct StarlarkParser {
    parser: tree_sitter::Parser,
}

impl StarlarkParser {
    pub fn new() -> Result<Self>;
    pub fn parse_build_file(&self, content: &str) -> Result<StarlarkAst>;
}

pub struct StarlarkAst {
    tree: tree_sitter::Tree,
    source: String,
}
```

### 2. Rule Extractor
```rust
// crates/core/src/bazel/rule_extractor.rs
pub struct RuleCall {
    pub rule_type: String,
    pub name: String,
    pub attributes: HashMap<String, AttributeValue>,
    pub location: SourceLocation,
}

pub enum AttributeValue {
    String(String),
    List(Vec<String>),
    Label(String),
    Glob(GlobPattern),
}

pub struct RuleExtractor;

impl RuleExtractor {
    pub fn extract_rules(ast: &StarlarkAst) -> Result<Vec<RuleCall>>;
}
```

### 3. Target Analyzer
```rust
// crates/core/src/bazel/target_analyzer.rs
pub struct BazelTarget {
    pub label: String,  // e.g., "//mylib:test"
    pub kind: BazelTargetKind,
    pub name: String,
    pub sources: Vec<String>,
    pub dependencies: Vec<String>,
    pub test_only: bool,
    pub attributes: TargetAttributes,
}

#[derive(Debug, Clone)]
pub enum BazelTargetKind {
    Binary,
    Test,
    TestSuite,
    DocTest,
    Benchmark,
    BuildScript,
    Library,  // Keep for dependency resolution
    Unknown(String),  // For extensibility
}

pub struct TargetAnalyzer {
    rule_handlers: HashMap<&'static str, Box<dyn RuleHandler>>,
}

trait RuleHandler {
    fn can_handle(&self, rule_type: &str) -> bool;
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget>;
    fn is_runnable(&self) -> bool;
}
```

### 4. Rule Handlers (Extensible)
```rust
// crates/core/src/bazel/rules/rust_binary.rs
struct RustBinaryHandler;

impl RuleHandler for RustBinaryHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_binary"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        // Extract srcs, deps, etc.
        Some(BazelTarget {
            kind: BazelTargetKind::Binary,
            // ... populate from rule
        })
    }
    
    fn is_runnable(&self) -> bool { true }
}
```

### 5. Target Finder (Replaces current detection)
```rust
// crates/core/src/bazel/target_finder.rs
pub struct BazelTargetFinder {
    parser: StarlarkParser,
    extractor: RuleExtractor,
    analyzer: TargetAnalyzer,
}

impl BazelTargetFinder {
    pub fn find_targets_for_file(
        &self,
        file_path: &Path,
        workspace_root: &Path,
    ) -> Result<Vec<BazelTarget>>;
    
    pub fn find_runnable_target(
        &self,
        file_path: &Path,
        workspace_root: &Path,
        kind_filter: Option<BazelTargetKind>,
    ) -> Result<Option<BazelTarget>>;
}
```

## Benefits
1. **Proper parsing** - Handles all valid Starlark syntax
2. **Extensible** - Easy to add new rule types via RuleHandler
3. **Accurate** - Understands globs, selects, and complex expressions
4. **Maintainable** - Clear separation of concerns
5. **Future-proof** - Can handle new rules_rust additions

## Migration Plan
1. Add tree-sitter-starlark dependency
2. Implement StarlarkParser
3. Build RuleExtractor to walk AST
4. Create RuleHandlers for each runnable type
5. Replace current string-based detection
6. Add comprehensive tests with real BUILD files

## Example Usage
```rust
let finder = BazelTargetFinder::new()?;
let targets = finder.find_targets_for_file(
    Path::new("src/lib.rs"),
    workspace_root,
)?;

// Find test target for the file
let test_target = finder.find_runnable_target(
    Path::new("src/lib.rs"),
    workspace_root,
    Some(BazelTargetKind::Test),
)?;
```