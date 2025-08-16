fn main() {
    println!("Hello from standalone Rust!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_standalone() {
        assert_eq!(2 + 2, 4);
    }
}