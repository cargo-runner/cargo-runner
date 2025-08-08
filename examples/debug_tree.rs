use tree_sitter::{Parser, TreeCursor};

fn print_tree(cursor: &mut TreeCursor, source: &str, depth: usize) {
    let node = cursor.node();
    let start = node.start_position();
    let end = node.end_position();
    
    println!("{}{} [{},{}]-[{},{}] {:?}", 
        " ".repeat(depth * 2), 
        node.kind(),
        start.row + 1, start.column + 1,
        end.row + 1, end.column + 1,
        node.utf8_text(source.as_bytes()).ok().map(|s| 
            if s.len() > 60 { format!("{}...", &s[..60]) } else { s.to_string() }
        )
    );
    
    if cursor.goto_first_child() {
        print_tree(cursor, source, depth + 1);
        while cursor.goto_next_sibling() {
            print_tree(cursor, source, depth + 1);
        }
        cursor.goto_parent();
    }
}

fn main() {
    let source = r#"#[cfg(test)]
mod tests {

    /// THIS DOEST WORK, THIS MUST BE START OF RANGE
    #[test] // THIS WORK
    fn test_it_works() {
        assert!(true);
    }
}"#;

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    if let Some(tree) = parser.parse(source, None) {
        let mut cursor = tree.walk();
        print_tree(&mut cursor, source, 0);
    }
}