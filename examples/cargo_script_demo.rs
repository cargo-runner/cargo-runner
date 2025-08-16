#!/usr/bin/env cargo +nightly -Zscript
//! A demo of cargo script with inline dependencies
//! 
//! ```cargo
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! serde_json = "1.0"
//! ```

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };

    let json = serde_json::to_string_pretty(&person).unwrap();
    println!("Person as JSON:");
    println!("{}", json);

    let parsed: Person = serde_json::from_str(&json).unwrap();
    println!("\nParsed back: {:?}", parsed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_serialization() {
        let person = Person {
            name: "Bob".to_string(),
            age: 25,
        };
        
        let json = serde_json::to_string(&person).unwrap();
        let parsed: Person = serde_json::from_str(&json).unwrap();
        
        assert_eq!(person.name, parsed.name);
        assert_eq!(person.age, parsed.age);
    }
    
    #[test]
    fn test_json_format() {
        let person = Person {
            name: "Charlie".to_string(),
            age: 40,
        };
        
        let json = serde_json::to_string(&person).unwrap();
        assert!(json.contains("\"name\":\"Charlie\""));
        assert!(json.contains("\"age\":40"));
    }
}