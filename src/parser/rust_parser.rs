use crate::{
    error::{Error, Result},
    parser::scope_detector::ScopeDetector,
    types::{ExtendedScope, Scope},
};
use std::path::Path;
use tree_sitter::Parser;

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
        let mut detector = ScopeDetector::new();
        let extended_scopes = detector.detect_scopes(&tree, source, file_path)?;
        Ok(extended_scopes.into_iter().map(|es| es.to_scope()).collect())
    }
    
    pub fn get_extended_scopes(&mut self, source: &str, file_path: &Path) -> Result<Vec<ExtendedScope>> {
        let tree = self.parse(source)?;
        let mut detector = ScopeDetector::new();
        detector.detect_scopes(&tree, source, file_path)
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