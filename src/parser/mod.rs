pub mod module_resolver;
pub mod scope_detector;

use crate::{Error, Position, Result, Scope, ScopeContext};
use std::path::Path;
use tree_sitter::{Node, Parser};

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| Error::TreeSitterError(format!("Failed to set language: {}", e)))?;
        Ok(Self { parser })
    }

    pub fn parse(&mut self, source: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| Error::ParseError("Failed to parse source code".to_string()))
    }

    pub fn get_scopes(&mut self, source: &str, file_path: &Path) -> Result<Vec<Scope>> {
        let tree = self.parse(source)?;
        let mut detector = scope_detector::ScopeDetector::new();
        let extended_scopes = detector.detect_scopes(&tree, source, file_path)?;
        Ok(extended_scopes.into_iter().map(|es| es.to_scope()).collect())
    }
    
    pub fn get_extended_scopes(&mut self, source: &str, file_path: &Path) -> Result<Vec<crate::ExtendedScope>> {
        let tree = self.parse(source)?;
        let mut detector = scope_detector::ScopeDetector::new();
        detector.detect_scopes(&tree, source, file_path)
    }

    pub fn get_scope_context(&mut self, source: &str, file_path: &Path, line: u32) -> Result<ScopeContext> {
        let scopes = self.get_scopes(source, file_path)?;

        let _position = Position { line, character: 0 };
        let mut containing_scopes: Vec<Scope> = scopes
            .iter()
            .filter(|scope| scope.contains_line(line))
            .cloned()
            .collect();

        containing_scopes.sort_by_key(|scope| scope.end.line - scope.start.line);

        let current_scope = containing_scopes.first().cloned();
        let parent_scopes = if containing_scopes.len() > 1 {
            containing_scopes[1..].to_vec()
        } else {
            vec![]
        };

        Ok(ScopeContext {
            current_scope,
            parent_scopes,
            all_scopes: scopes,
        })
    }
}

pub fn node_to_position(node: &Node, start: bool) -> Position {
    let point = if start {
        node.start_position()
    } else {
        node.end_position()
    };
    Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}

pub fn find_doc_comments_before(node: &Node, source: &str) -> Option<(Position, Position)> {
    let mut current = node.prev_sibling();
    let mut first_doc_line = None;
    let mut last_doc_line = None;

    while let Some(sibling) = current {
        if sibling.kind() == "line_comment" {
            let comment_text = sibling.utf8_text(source.as_bytes()).ok()?;
            if comment_text.starts_with("///") {
                let start_pos = node_to_position(&sibling, true);
                if first_doc_line.is_none() {
                    first_doc_line = Some(start_pos);
                }
                last_doc_line = Some(start_pos);
            } else {
                break;
            }
        } else if sibling.kind() != "attribute_item" {
            break;
        }
        current = sibling.prev_sibling();
    }

    if let (Some(first), Some(_)) = (first_doc_line, last_doc_line) {
        let end = node_to_position(node, false);
        Some((first, end))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = RustParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_basic_parsing() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let tree = parser.parse(source);
        assert!(tree.is_ok());
    }
}
