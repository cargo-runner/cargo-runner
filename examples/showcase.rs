//! This file showcases all types of runnables that cargo-runner can detect


/// A user struct with documentation
/// 
/// # Examples
/// 
/// ```rust
/// let user = User::new("Alice", 30);
/// assert_eq!(user.name, "Alice");
/// assert_eq!(user.age, 30);
/// ```
struct User {
    name: String,
    #[allow(dead_code)]
    age: u32,
}

impl User {
    /// Creates a new user
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let user = User::new("Bob", 25);
    /// assert_eq!(user.name(), "Bob");
    /// ```
    pub fn new(name: &str, age: u32) -> Self {
        Self {
            name: name.to_string(),
            age,
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A simple function with a doc test
/// 
/// # Examples
/// 
/// ```rust
/// assert_eq!(add(2, 3), 5);
/// assert_eq!(add(-1, 1), 0);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    assert_eq!(add(2, 2), 4);
    assert_eq!(add(0, 0), 0);
    assert_eq!(add(-5, 5), 0);
}

#[test]
fn test_user_creation() {
    let user = User::new("Charlie", 35);
    assert_eq!(user.name(), "Charlie");
    assert_eq!(user.age, 35);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_name() {
        let user = User::new("David", 40);
        assert_eq!(user.name(), "David");
    }
    
    #[test]
    fn test_add_negative() {
        assert_eq!(add(-10, -20), -30);
    }
    
    mod nested_tests {
        use super::*;
        
        #[test]
        fn test_nested() {
            assert!(true);
        }
    }
}

// Note: #[bench] requires nightly Rust, so these are commented out
// but would be detected by cargo-runner if uncommented on nightly

// #[bench]
// fn bench_add(b: &mut test::Bencher) {
//     b.iter(|| {
//         add(10, 20)
//     });
// }

// #[bench]
// fn bench_user_creation(b: &mut test::Bencher) {
//     b.iter(|| {
//         User::new("Bench User", 30)
//     });
// }

// Example of async test (requires tokio in dependencies)
// #[tokio::test]
// async fn test_async_operation() {
//     let result = async { 42 }.await;
//     assert_eq!(result, 42);
// }

fn main() {
    println!("This is a showcase of cargo-runner features!");
    
    let user = User::new("Main User", 25);
    println!("Created user: {}", user.name());
    
    let sum = add(5, 7);
    println!("5 + 7 = {}", sum);
}