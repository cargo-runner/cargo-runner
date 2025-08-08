use cargo_runner::parser::RustParser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
/// A color enum
enum Color {
    Red,
    Green,
    Blue,
}

/// A union type
union MyUnion {
    i: i32,
    f: f32,
}
"#;

    let mut parser = RustParser::new()?;
    let tree = parser.parse(source)?;
    let root = tree.root_node();

    print_node_tree(&root, source, 0);

    Ok(())
}

fn print_node_tree(node: &tree_sitter::Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);

    if node.is_named() && (depth < 3 || node.kind().contains("item")) {
        println!(
            "{}{} [{}-{}]",
            indent,
            node.kind(),
            node.start_position().row,
            node.end_position().row
        );

        if node.kind() == "enum_item" || node.kind() == "union_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    println!("{}  name: {}", indent, name);
                }
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_node_tree(&child, source, depth + 1);
    }
}
