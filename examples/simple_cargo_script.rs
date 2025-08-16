#!/usr/bin/env cargo +nightly -Zscript

fn main() {
    println!("Hello from cargo script!");
    let numbers: Vec<i32> = (1..=10).collect();
    let sum: i32 = numbers.iter().sum();
    println!("Sum of 1 to 10: {}", sum);
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