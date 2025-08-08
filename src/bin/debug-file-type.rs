use cargo_runner::parser::{RustParser, module_resolver::ModuleResolver};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = vec![
        "src/utils.rs",
        "/tmp/standalone.rs",
    ];
    
    for file in files {
        let path = Path::new(file);
        if path.exists() {
            let cargo_toml = ModuleResolver::find_cargo_toml(path);
            println!("{}: Cargo.toml = {:?}", file, cargo_toml.is_some());
            
            let source = std::fs::read_to_string(path)?;
            let mut parser = RustParser::new()?;
            let scopes = parser.get_extended_scopes(&source, path)?;
            
            if let Some(file_scope) = scopes.first() {
                match &file_scope.scope.kind {
                    cargo_runner::ScopeKind::File(fs) => {
                        println!("  FileScope: {:?}", fs);
                    }
                    _ => println!("  Not a file scope"),
                }
            }
        } else {
            println!("{}: File not found", file);
        }
    }
    
    Ok(())
}