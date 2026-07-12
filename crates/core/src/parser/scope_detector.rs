use super::utils::node_to_position;
use crate::{
    error::{Error, Result},
    types::{ExtendedScope, FileScope, Position, Scope, ScopeKind},
};
use cargo_toml::Manifest;
use std::path::Path;
use tree_sitter::{Node, Tree};

#[derive(Debug, Clone)]
struct ExtendedScopeDetails {
    #[allow(dead_code)]
    original_start: Position,
    #[allow(dead_code)]
    extended_start: Position,
    doc_comment_lines: u32,
    attribute_lines: u32,
    has_doc_tests: bool,
}

pub struct ScopeDetector;

impl Default for ScopeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_scopes(
        &mut self,
        tree: &Tree,
        source: &str,
        file_path: &Path,
    ) -> Result<Vec<ExtendedScope>> {
        let mut scopes = Vec::new();
        let root_node = tree.root_node();

        let file_type = self.determine_file_type(file_path)?;
        let file_scope = Scope {
            start: node_to_position(&root_node, true),
            end: node_to_position(&root_node, false),
            kind: ScopeKind::File(file_type),
            name: None,
        };
        scopes.push(ExtendedScope::from(file_scope));

        self.visit_node(&root_node, source, &mut scopes)?;

        Ok(scopes)
    }

    fn determine_file_type(&self, file_path: &Path) -> Result<FileScope> {
        let path_str = file_path
            .to_str()
            .ok_or(Error::InvalidPath("Invalid file path"))?;
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check well-known patterns first
        if path_str.contains("/src/lib.rs")
            || path_str == "src/lib.rs"
            || path_str.ends_with("/lib.rs")
        {
            return Ok(FileScope::Lib);
        }

        if path_str.contains("/src/main.rs")
            || path_str == "src/main.rs"
            || path_str.ends_with("/main.rs")
        {
            return Ok(FileScope::Bin {
                name: Some("main".to_string()),
            });
        }

        if path_str.contains("/src/bin/") || path_str.starts_with("src/bin/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Bin { name });
        }

        if (path_str.contains("/tests/") || path_str.starts_with("tests/"))
            && !path_str.contains("/src/")
        {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Test { name });
        }

        if path_str.contains("/benches/") || path_str.starts_with("benches/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Bench { name });
        }

        if path_str.contains("/examples/") || path_str.starts_with("examples/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Example { name });
        }

        if file_name == "build.rs" {
            return Ok(FileScope::Build);
        }

        // Any other .rs file under src/ is part of the library
        if path_str.contains("/src/")
            && file_path.extension().and_then(|s| s.to_str()) == Some("rs")
        {
            return Ok(FileScope::Lib);
        }

        // Check Cargo.toml for custom paths
        use crate::parser::module_resolver::ModuleResolver;
        let has_cargo_toml = ModuleResolver::find_cargo_toml(file_path).is_some();

        if has_cargo_toml {
            if let Some(file_type) = self.check_cargo_toml_for_file_type(file_path)? {
                return Ok(file_type);
            }
            // Has Cargo.toml but file doesn't match any patterns
            return Ok(FileScope::Unknown);
        }

        // No Cargo.toml found - this is a standalone Rust file
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Standalone { name });
        }

        Ok(FileScope::Unknown)
    }

    fn check_cargo_toml_for_file_type(&self, file_path: &Path) -> Result<Option<FileScope>> {
        use crate::parser::module_resolver::ModuleResolver;

        // Find the Cargo.toml file
        let cargo_toml_path = if let Some(path) = ModuleResolver::find_cargo_toml(file_path) {
            path
        } else {
            return Ok(None);
        };

        // Get project root
        let project_root = cargo_toml_path
            .parent()
            .ok_or(Error::InvalidPath("Cannot determine project root"))?;

        // Parse Cargo.toml
        let manifest = Manifest::from_path(&cargo_toml_path).map_err(Error::CargoTomlParse)?;

        // Get relative path from project root
        let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
        let relative_str = relative_path
            .to_str()
            .ok_or(Error::InvalidPath("Invalid relative path"))?;

        // Check [[bin]] entries
        for bin in &manifest.bin {
            if let Some(path) = &bin.path
                && path == relative_str
            {
                let name = bin.name.clone().or_else(|| {
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                });
                return Ok(Some(FileScope::Bin { name }));
            }
        }

        // Check [[test]] entries
        for test in &manifest.test {
            if let Some(path) = &test.path
                && path == relative_str
            {
                let name = test.name.clone().or_else(|| {
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                });
                return Ok(Some(FileScope::Test { name }));
            }
        }

        // Check [[bench]] entries
        for bench in &manifest.bench {
            if let Some(path) = &bench.path
                && path == relative_str
            {
                let name = bench.name.clone().or_else(|| {
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                });
                return Ok(Some(FileScope::Bench { name }));
            }
        }

        // Check [[example]] entries
        for example in &manifest.example {
            if let Some(path) = &example.path
                && path == relative_str
            {
                let name = example.name.clone().or_else(|| {
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                });
                return Ok(Some(FileScope::Example { name }));
            }
        }

        // Check [lib] entry
        if let Some(lib) = &manifest.lib
            && let Some(path) = &lib.path
            && path == relative_str
        {
            return Ok(Some(FileScope::Lib));
        }

        Ok(None)
    }

    fn visit_node(&self, node: &Node, source: &str, scopes: &mut Vec<ExtendedScope>) -> Result<()> {
        match node.kind() {
            "function_item" => {
                self.handle_function(node, source, scopes)?;
            }
            "mod_item" => {
                self.handle_module(node, source, scopes)?;
            }
            "struct_item" => {
                self.handle_struct(node, source, scopes)?;
            }
            "enum_item" => {
                self.handle_enum(node, source, scopes)?;
            }
            "union_item" => {
                self.handle_union(node, source, scopes)?;
            }
            "impl_item" => {
                self.handle_impl(node, source, scopes)?;
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.visit_node(&child, source, scopes)?;
        }

        Ok(())
    }

    fn handle_function(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or(Error::MissingEntityName { entity: "Function" })?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::InvalidUtf8Name {
                entity: "Function",
                err: e,
            })?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let is_test = self.has_test_attribute(node, source);
        let is_bench = self.has_bench_attribute(node, source);

        let kind = if is_test {
            ScopeKind::Test
        } else if is_bench {
            ScopeKind::Benchmark
        } else {
            ScopeKind::Function
        };

        let scope = Scope {
            start: extended_start,
            end,
            kind,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_module(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or(Error::MissingEntityName { entity: "Module" })?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::InvalidUtf8Name {
                entity: "Module",
                err: e,
            })?
            .to_string();

        // Get extended scope info for modules too (e.g., #[cfg(test)])
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Module,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_struct(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or(Error::MissingEntityName { entity: "Struct" })?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::InvalidUtf8Name {
                entity: "Struct",
                err: e,
            })?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Struct,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_enum(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or(Error::MissingEntityName { entity: "Enum" })?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::InvalidUtf8Name {
                entity: "Enum",
                err: e,
            })?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Enum,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_union(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or(Error::MissingEntityName { entity: "Union" })?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::InvalidUtf8Name {
                entity: "Union",
                err: e,
            })?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Union,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_impl(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let mut name = String::from("impl");

        if let Some(type_node) = node.child_by_field_name("type")
            && let Ok(type_name) = type_node.utf8_text(source.as_bytes())
        {
            name = format!("impl {type_name}");
        }

        if let Some(trait_node) = node.child_by_field_name("trait")
            && let Ok(trait_name) = trait_node.utf8_text(source.as_bytes())
        {
            name = format!("{trait_name} for");
            if let Some(type_node) = node.child_by_field_name("type")
                && let Ok(type_name) = type_node.utf8_text(source.as_bytes())
            {
                name = format!("{trait_name} for {type_name}");
            }
        }

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Impl,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn has_test_attribute(&self, node: &Node, source: &str) -> bool {
        // Check for attribute items before the function
        let mut sibling = node.prev_sibling();

        while let Some(s) = sibling {
            if s.kind() == "attribute_item" {
                if let Ok(text) = s.utf8_text(source.as_bytes())
                    && (text.contains("#[test]") || text.contains("#[tokio::test]"))
                {
                    return true;
                }
            } else if s.kind() != "line_comment" && s.kind() != "block_comment" {
                // Stop if we hit something that's not an attribute or comment
                break;
            }
            sibling = s.prev_sibling();
        }

        false
    }

    fn has_bench_attribute(&self, node: &Node, source: &str) -> bool {
        // Check for attribute items before the function
        let mut sibling = node.prev_sibling();

        while let Some(s) = sibling {
            if s.kind() == "attribute_item" {
                if let Ok(text) = s.utf8_text(source.as_bytes())
                    && text.contains("#[bench]")
                {
                    return true;
                }
            } else if s.kind() != "line_comment" && s.kind() != "block_comment" {
                // Stop if we hit something that's not an attribute or comment
                break;
            }
            sibling = s.prev_sibling();
        }

        false
    }

    fn find_extended_info(&self, node: &Node, source: &str) -> (Position, ExtendedScopeDetails) {
        let original_start = node_to_position(node, true);
        let mut current_sibling = node.prev_sibling();
        let mut extended_start = original_start;
        let mut doc_comment_lines = 0u32;
        let mut attribute_lines = 0u32;
        let mut has_doc_tests = false;
        let mut found_any = false;

        // Process siblings in reverse order to find the topmost doc comment or attribute
        let mut siblings_to_process = Vec::new();
        while let Some(sibling) = current_sibling {
            match sibling.kind() {
                "line_comment" => {
                    if let Ok(text) = sibling.utf8_text(source.as_bytes())
                        && is_doc_line_comment(text)
                    {
                        siblings_to_process.push((sibling, "doc_comment"));
                        if text.contains("```") {
                            has_doc_tests = true;
                        }
                    }
                    // Don't break on regular comments - just skip them
                    // This allows us to find doc comments that come before regular comments
                }
                "block_comment" => {
                    if let Ok(text) = sibling.utf8_text(source.as_bytes())
                        && is_doc_block_comment(text)
                    {
                        siblings_to_process.push((sibling, "doc_comment"));
                        if text.contains("```") {
                            has_doc_tests = true;
                        }
                    } else {
                        // Non-doc block comment: skip without stopping (same as //)
                    }
                }
                "attribute_item" => {
                    siblings_to_process.push((sibling, "attribute"));
                }
                _ => {
                    // Stop if we hit anything else
                    break;
                }
            }
            current_sibling = sibling.prev_sibling();
        }

        // Process in forward order to get the correct counts
        for (sibling, kind) in siblings_to_process.iter().rev() {
            let pos = node_to_position(sibling, true);
            if !found_any {
                extended_start = pos;
                found_any = true;
            }

            match *kind {
                "doc_comment" => {
                    // Line comments: one node per source line. Block docs may span many lines.
                    if sibling.kind() == "block_comment" {
                        let end_pos = node_to_position(sibling, false);
                        doc_comment_lines += end_pos.line.saturating_sub(pos.line) + 1;
                    } else {
                        doc_comment_lines += 1;
                    }
                }
                "attribute" => {
                    let end_pos = node_to_position(sibling, false);
                    attribute_lines += end_pos.line.saturating_sub(pos.line) + 1;
                }
                _ => {}
            }
        }

        let details = ExtendedScopeDetails {
            original_start,
            extended_start,
            doc_comment_lines,
            attribute_lines,
            has_doc_tests,
        };

        (extended_start, details)
    }

    pub fn find_doc_tests(&self, node: &Node, source: &str) -> Vec<DocTestSpan> {
        find_doc_tests_recursive(node, source)
    }
}

/// A fenced doc example found in source comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTestSpan {
    pub start: Position,
    pub end: Position,
    pub text: String,
    /// False when the opening fence uses `ignore`, `no_run`, or `compile_fail`.
    pub executable: bool,
}

/// Outer (`///`) or inner (`//!`) line doc comment (not `////…`).
fn is_doc_line_comment(text: &str) -> bool {
    let t = text.trim_start();
    (t.starts_with("///") && !t.starts_with("////")) || t.starts_with("//!")
}

/// Block doc comment (`/**` or `/*!`).
fn is_doc_block_comment(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("/**") || t.starts_with("/*!")
}

fn strip_line_doc_prefix(text: &str) -> Option<&str> {
    let t = text.trim_start();
    if t.starts_with("///") && !t.starts_with("////") {
        Some(t[3..].trim_start())
    } else if let Some(rest) = t.strip_prefix("//!") {
        Some(rest.trim_start())
    } else {
        None
    }
}

/// Strip `/**` / `/*!` / `*/` and leading `*` on each content line.
fn strip_block_doc_body(text: &str) -> String {
    let mut body = text.trim_start();
    if let Some(rest) = body.strip_prefix("/**") {
        body = rest;
    } else if let Some(rest) = body.strip_prefix("/*!") {
        body = rest;
    }
    if let Some(rest) = body.strip_suffix("*/") {
        body = rest;
    }

    let mut out = String::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        let content = if let Some(rest) = trimmed.strip_prefix('*') {
            rest.strip_prefix(' ').unwrap_or(rest)
        } else {
            line
        };
        out.push_str(content);
        out.push('\n');
    }
    out
}

/// Whether the first markdown fence in `text` is an executable rustdoc example.
pub fn fence_is_executable(text: &str) -> bool {
    let Some(idx) = text.find("```") else {
        return false;
    };
    let after = &text[idx + 3..];
    let tag_line = after.lines().next().unwrap_or("").trim();
    // Split on commas and whitespace: ```rust,ignore / ```ignore / ```no_run
    for part in tag_line.split(|c: char| c == ',' || c.is_whitespace()) {
        let tag = part.trim();
        if tag.is_empty() {
            continue;
        }
        if matches!(tag, "ignore" | "no_run" | "compile_fail") {
            return false;
        }
    }
    true
}

/// Extract fenced examples from a contiguous body of doc text spanning `start_line..=end_line`.
fn spans_from_doc_body(body: &str, start_line: u32, start_char: u32) -> Vec<DocTestSpan> {
    let mut spans = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        if !lines[i].contains("```") {
            i += 1;
            continue;
        }
        // Opening fence at line i
        let open_i = i;
        let mut full = String::new();
        full.push_str(lines[i]);
        full.push('\n');
        i += 1;
        let mut closed = false;
        while i < lines.len() {
            full.push_str(lines[i]);
            full.push('\n');
            if lines[i].contains("```") {
                closed = true;
                break;
            }
            i += 1;
        }
        if closed {
            let end_i = i;
            spans.push(DocTestSpan {
                start: Position {
                    line: start_line + open_i as u32,
                    character: if open_i == 0 { start_char } else { 0 },
                },
                end: Position {
                    line: start_line + end_i as u32,
                    character: lines[end_i].len() as u32,
                },
                text: full.clone(),
                executable: fence_is_executable(&full),
            });
            i += 1;
        } else {
            // Unclosed fence — stop scanning this body
            break;
        }
    }
    spans
}

fn find_doc_tests_recursive(node: &Node, source: &str) -> Vec<DocTestSpan> {
    let mut doc_tests = Vec::new();

    // Contiguous run of outer/inner line doc comments: only start at the first
    // line of a run so multi-example blocks are scanned once.
    if node.kind() == "line_comment"
        && let Ok(comment_text) = node.utf8_text(source.as_bytes())
        && is_doc_line_comment(comment_text)
    {
        let prev_is_doc = node
            .prev_sibling()
            .and_then(|p| {
                if p.kind() == "line_comment" {
                    p.utf8_text(source.as_bytes()).ok().map(is_doc_line_comment)
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if !prev_is_doc {
            let start = node_to_position(node, true);
            let mut body = String::new();
            let mut end_line = start.line;
            let mut current = Some(*node);

            while let Some(n) = current {
                if n.kind() != "line_comment" {
                    break;
                }
                let Ok(text) = n.utf8_text(source.as_bytes()) else {
                    break;
                };
                let Some(stripped) = strip_line_doc_prefix(text) else {
                    break;
                };
                body.push_str(stripped);
                body.push('\n');
                end_line = node_to_position(&n, false).line;
                current = n.next_sibling();
            }

            if body.contains("```") {
                let mut spans = spans_from_doc_body(&body, start.line, start.character);
                // Clamp end lines if spans_from_doc_body overshoots
                for s in &mut spans {
                    if s.end.line > end_line {
                        s.end.line = end_line;
                    }
                }
                doc_tests.extend(spans);
            }
        }
    }

    // Block doc comments are a single AST node (may contain multiple fences).
    if node.kind() == "block_comment"
        && let Ok(comment_text) = node.utf8_text(source.as_bytes())
        && is_doc_block_comment(comment_text)
        && comment_text.contains("```")
    {
        let start = node_to_position(node, true);
        let body = strip_block_doc_body(comment_text);
        doc_tests.extend(spans_from_doc_body(&body, start.line, start.character));
    }

    for child in node.children(&mut node.walk()) {
        doc_tests.extend(find_doc_tests_recursive(&child, source));
    }

    doc_tests
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RustParser;

    #[test]
    fn test_detect_function_scope() {
        let source = r#"
fn main() {
    println!("Hello, world!");
}

fn test_function() {
    assert_eq!(2 + 2, 4);
}
"#;

        let mut parser = RustParser::new().unwrap();
        let scopes = parser.get_scopes(source, Path::new("test.rs")).unwrap();

        assert!(scopes.iter().any(|s| s.name == Some("main".to_string())));
        assert!(
            scopes
                .iter()
                .any(|s| s.name == Some("test_function".to_string()))
        );
    }

    #[test]
    fn test_detect_test_scope() {
        let source = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}
"#;

        let mut parser = RustParser::new().unwrap();
        let scopes = parser.get_scopes(source, Path::new("test.rs")).unwrap();

        let test_scope = scopes
            .iter()
            .find(|s| s.name == Some("test_addition".to_string()))
            .unwrap();

        assert_eq!(test_scope.kind, ScopeKind::Test);
    }

    #[test]
    fn fence_is_executable_honors_rustdoc_tags() {
        assert!(fence_is_executable("```\nassert!(true);\n```\n"));
        assert!(fence_is_executable("```rust\nassert!(true);\n```\n"));
        assert!(fence_is_executable("```should_panic\npanic!();\n```\n"));
        assert!(!fence_is_executable("```ignore\nassert!(true);\n```\n"));
        assert!(!fence_is_executable("```rust,ignore\nx\n```\n"));
        assert!(!fence_is_executable("```no_run\nmain();\n```\n"));
        assert!(!fence_is_executable("```compile_fail\nbroken\n```\n"));
    }

    #[test]
    fn find_doc_tests_outer_line_docs() {
        let source = r#"
/// Adds numbers
///
/// ```
/// assert_eq!(1 + 1, 2);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let mut parser = RustParser::new().unwrap();
        let spans = parser.find_doc_tests(source).unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].executable);
        assert!(spans[0].text.contains("assert_eq!"));
    }

    #[test]
    fn find_doc_tests_inner_line_docs() {
        let source = r#"
mod sample {
    //! Inner docs with a fence
    //!
    //! ```
    //! assert!(true);
    //! ```
}
"#;
        let mut parser = RustParser::new().unwrap();
        let spans = parser.find_doc_tests(source).unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].executable);
    }

    #[test]
    fn find_doc_tests_block_docs() {
        let source = r#"
/**
 * Block docs
 *
 * ```
 * assert_eq!(2 + 2, 4);
 * ```
 */
pub struct Blocked;
"#;
        let mut parser = RustParser::new().unwrap();
        let spans = parser.find_doc_tests(source).unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].executable);
        assert!(spans[0].text.contains("assert_eq!"));
    }

    #[test]
    fn find_doc_tests_skips_ignore_fence() {
        let source = r#"
/// ```ignore
/// not_run();
/// ```
pub fn skipped() {}
"#;
        let mut parser = RustParser::new().unwrap();
        let spans = parser.find_doc_tests(source).unwrap();
        assert_eq!(spans.len(), 1);
        assert!(!spans[0].executable);
    }

    #[test]
    fn find_doc_tests_multi_example_in_one_block() {
        let source = r#"
/// First
/// ```
/// assert_eq!(1, 1);
/// ```
/// Second
/// ```
/// assert_eq!(2, 2);
/// ```
pub fn multi() {}
"#;
        let mut parser = RustParser::new().unwrap();
        let spans = parser.find_doc_tests(source).unwrap();
        assert_eq!(spans.len(), 2);
        assert!(spans.iter().all(|s| s.executable));
    }
}
