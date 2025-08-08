use cargo_runner::parser::RustParser;
use std::path::Path;

fn main() {
    let source = r#"
    /// THIS DOEST WORK, THIS MUST BE START OF RANGE : runs -> cargo test --package project-a --lib -- tests::test_it_works --exact --show-output
    #[test] // THIS WORK
    fn test_it_works() {
        assert!(true); // Fixed: now running: cargo test --package project-a --lib -- tests::test_it_works --exact --show-output
    } // Fixed! now running: cargo test --package project-a --lib -- tests::test_it_works --exact --show-output
"#;

    let mut parser = RustParser::new().unwrap();
    let extended_scopes = parser
        .get_extended_scopes(source, Path::new("test.rs"))
        .unwrap();

    for scope in extended_scopes {
        println!(
            "Scope: {:?} {}",
            scope.scope.kind,
            scope.scope.name.as_deref().unwrap_or("<unnamed>")
        );
        println!(
            "  Range: lines {}-{}",
            scope.scope.start.line + 1,
            scope.scope.end.line + 1
        );
        println!("  Doc comments: {}", scope.doc_comment_lines);
        println!("  Attributes: {}", scope.attribute_lines);
        println!("  Original start: line {}", scope.original_start.line + 1);
        println!();
    }
}
