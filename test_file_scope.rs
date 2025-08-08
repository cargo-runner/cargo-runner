// This is a test file to demonstrate file scope line numbers

use std::collections::HashMap;

/// Main function
fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn test_another_thing() {
        assert!(true);
    }
}

/// A helper function
fn helper() -> i32 {
    42
}