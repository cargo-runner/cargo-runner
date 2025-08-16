#!/usr/bin/env cargo-runner

fn main() {
    println!("Hello from standalone script!");
    let sum: i32 = (1..=100).sum();
    println!("Sum of 1 to 100: {}", sum);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn test_sum() {
        let sum: i32 = (1..=5).sum();
        assert_eq!(sum, 15);
    }
}