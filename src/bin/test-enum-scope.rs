use cargo_runner::parser::RustParser;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
/// A color enum
#[derive(Debug)]
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

/// A struct
struct Person {
    name: String,
    age: u32,
}
"#;
    
    let mut parser = RustParser::new()?;
    let scopes = parser.get_extended_scopes(source, Path::new("test.rs"))?;
    
    println!("=== All Scopes ===");
    for (i, scope) in scopes.iter().enumerate() {
        println!("{}. {:?} {} - lines {}-{}", 
            i + 1,
            scope.scope.kind,
            scope.scope.name.as_deref().unwrap_or("<unnamed>"),
            scope.scope.start.line + 1,
            scope.scope.end.line + 1
        );
        
        if scope.doc_comment_lines > 0 {
            println!("   Doc comments: {} lines", scope.doc_comment_lines);
        }
    }
    
    Ok(())
}