fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn test_another() {
        assert!(true);
    }
}