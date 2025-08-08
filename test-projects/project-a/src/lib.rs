/// A user struct
/// 
/// ```
/// let user = User::new("Alice", 30);
/// assert_eq!(user.name, "Alice");
/// ```
pub struct User {
    pub name: String,
    pub age: u32,
}

impl User {
    /// Creates a new user
    /// 
    /// # Examples
    /// 
    /// ```
    /// let user = User::new("Bob", 25);
    /// assert_eq!(user.age, 25);
    /// ```
    pub fn new(name: &str, age: u32) -> Self {
        User {
            name: name.to_string(),
            age,
        }
    }
    
    /// Echo the user's name
    /// 
    /// ```
    /// let user = User::new("Charlie", 35);
    /// assert_eq!(user.echo(), "Charlie");
    /// ```
    pub fn echo(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
    
    #[test]
    fn test_user() {
        let user = User::new("Dave", 40);
        assert_eq!(user.name, "Dave");
        assert_eq!(user.age, 40);
        assert_eq!(user.echo(), "Dave");
    }
}